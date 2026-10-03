//! Coherent storage reads, current authorization and bounded selection.
use super::{make_anchor, normalization::check_size};
use crate::query_storage::validate_query_read;
use crate::{
    Access, Actor, Error, ProjectedView, QUERY_SEMANTICS_VERSION, QueryAnchor, QueryBounds,
    QuerySpec, Result, Row, Runtime, SelectionMode, StorageQuery, Value,
};
use std::cmp::Ordering as Cmp;
impl Runtime {
    pub(crate) fn select_rows(
        &self,
        actor: &Actor,
        kind: &str,
        query: &QuerySpec,
    ) -> Result<Vec<Row>> {
        let plan = self.query_plan(actor, kind, query)?;
        let def = self.0.registry.get(kind).ok_or(Error::Unregistered)?;
        // Explicit actor-only authorization is one decision per selection attempt.
        // Final disclosure still performs its normal current-authority checks.
        let uniform_read = def.uniform_read(actor);
        if uniform_read == Some(false) {
            return Err(Error::Denied);
        }
        let uniform_fields = uniform_read.is_some() && def.uniform_fields();
        let request = StorageQuery {
            spec: plan.spec.clone(),
            descriptor: def.descriptor(),
            semantics: QUERY_SEMANTICS_VERSION,
            selection: if uniform_fields {
                SelectionMode::UniformReadAndFields
            } else {
                SelectionMode::ReferenceOnly
            },
        };
        let bounds = QueryBounds {
            max_rows: self.0.limits.snapshot_rows,
            max_bytes: self.0.limits.snapshot_bytes,
        };
        let rows = validate_query_read(
            &request,
            bounds,
            self.0.storage.query_read(&request, bounds)?,
        )?;
        let mut selected = Vec::new();
        for row in rows {
            if plan.spec.order.is_empty()
                && (plan
                    .spec
                    .after_id
                    .as_ref()
                    .is_some_and(|id| row.key.id <= *id)
                    || plan.spec.after.as_ref().is_some_and(|a| row.key.id <= a.id))
            {
                continue;
            }
            let Some(value) = row.value.as_ref() else {
                continue;
            };
            if uniform_read.is_none() && !def.allows(actor, Access::Read, value) {
                continue;
            }
            if !uniform_fields {
                for o in &plan.spec.order {
                    if !def.allows_field(actor, Access::Read, &o.field, value) {
                        return Err(Error::Denied);
                    }
                }
            }
            if !plan.matches(value)?
                || plan
                    .spec
                    .after_id
                    .as_ref()
                    .is_some_and(|id| row.key.id <= *id)
            {
                continue;
            }
            let keys = plan.keys(value)?;
            if let Some(anchor) = &plan.spec.after
                && plan.compare_keys(&keys, &row.key.id, &anchor.values, &anchor.id)?
                    != Cmp::Greater
            {
                continue;
            }
            selected.push((row, keys));
            // ID-sorted input already has final order. Keep the legacy page fast path;
            // full snapshot size and duplicate validation still happened above.
            if plan.spec.order.is_empty()
                && selected.len() == plan.spec.limit.unwrap_or(self.0.limits.snapshot_rows)
            {
                break;
            }
        }
        // Every key was validated against its scalar shape above; sorting is infallible.
        if !plan.spec.order.is_empty() {
            selected.sort_by(|(a, ak), (b, bk)| {
                plan.compare_keys(ak, &a.key.id, bk, &b.key.id)
                    .expect("validated query keys")
            });
        }
        selected.truncate(plan.spec.limit.unwrap_or(self.0.limits.snapshot_rows));
        Ok(selected.into_iter().map(|(row, _)| row).collect())
    }
    /// Build a client boundary from supplied projected values under current query/sort grants.
    /// This does not certify origin or current row/field visibility; selection checks those.
    pub async fn query_anchor(
        &self,
        actor: &Actor,
        spec: &QuerySpec,
        view: &ProjectedView,
    ) -> Result<QueryAnchor> {
        let a = actor.clone();
        let spec = spec.clone();
        let view = view.clone();
        self.observe(actor, move |runtime| {
            let plan = runtime.query_plan(&a, &view.key.kind, &spec)?;
            let def = runtime
                .0
                .registry
                .get(&view.key.kind)
                .ok_or(Error::Unregistered)?;
            let value = Value::Object(view.value.clone().ok_or(Error::Missing)?);
            let anchor = make_anchor(def.descriptor_ref(), &plan, &view.key.id, &value, |n, v| {
                def.normalize_field(n, v)
            })?;
            check_size(&anchor, runtime.0.limits.command_bytes)?;
            Ok(anchor)
        })
        .await
    }
}
