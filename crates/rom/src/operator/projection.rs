//! Authorized redacted records and whole-envelope page admission.
use super::*;
use crate::{Actor, Result, Runtime, StorageWorkSnapshot, WorkRecord};
pub(super) fn view(
    snapshot: &StorageWorkSnapshot,
    record: &WorkRecord,
    scope: WorkScope,
) -> WorkView {
    WorkView {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        handle: WorkHandle::from_work_id(&record.pending.id),
        version: snapshot.version(record),
        category: scope.category,
        definition: scope.definition,
        state: work_status(&record.state),
        attempts: record.attempts,
        due: record.due,
        delivery: record.delivery.clone(),
        source: scope.source,
        target: scope.target,
    }
}
fn matches(view: &WorkView, query: &WorkQuery) -> bool {
    query
        .state
        .as_ref()
        .is_none_or(|state| state == &view.state)
        && query
            .category
            .as_ref()
            .is_none_or(|category| category == &view.category)
        && query
            .definition
            .as_ref()
            .is_none_or(|name| name == &view.definition.name)
}
impl Runtime {
    pub(super) fn operator_snapshot(&self) -> Result<StorageWorkSnapshot> {
        let bounds = &self.0.operator_limits;
        let snapshot = self
            .0
            .storage
            .work_snapshot(bounds.max_snapshot_records, bounds.max_snapshot_bytes)?;
        snapshot.validate(bounds.max_snapshot_records, bounds.max_snapshot_bytes)?;
        Ok(snapshot)
    }
    pub(super) fn operator_page(
        &self,
        actor: &Actor,
        snapshot: &StorageWorkSnapshot,
        query: &WorkQuery,
    ) -> Result<WorkPage> {
        let after = super::cursor::decode(&snapshot.generation, query)?;
        let mut visible = Vec::new();
        for record in &snapshot.records {
            let scope = WorkScope::from_record(record)?;
            match self.authorize_operator(actor, OperatorAccess::Inspect, Some(&scope)) {
                Ok(()) => {}
                Err(crate::Error::Denied) => continue,
                Err(error) => return Err(error),
            }
            let view = view(snapshot, record, scope);
            if after.as_ref().is_none_or(|anchor| &view.handle > anchor) && matches(&view, query) {
                visible.push(view);
            }
        }
        visible.sort_by(|a, b| a.handle.cmp(&b.handle));
        let more = visible.len() > query.limit;
        visible.truncate(query.limit);
        let mut page = WorkPage {
            protocol_version: OPERATOR_PROTOCOL_VERSION,
            records: visible,
            cursor: None,
        };
        let mut more = more;
        loop {
            page.cursor = if more {
                page.records
                    .last()
                    .map(|last| super::cursor::encode(&snapshot.generation, query, &last.handle))
                    .transpose()?
            } else {
                None
            };
            match page.validate(&self.0.operator_limits.responses) {
                Ok(()) => return Ok(page),
                Err(crate::Error::TooLarge) if page.records.len() > 1 => {
                    page.records.pop();
                    more = true;
                }
                Err(error) => return Err(error),
            }
        }
    }
}
