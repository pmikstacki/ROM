//! Coherent bounded trusted snapshots for operator projection.
use super::*;
use crate::operator::{WorkHandle, WorkVersion};

/// Trusted adapter data; frozen records must never be serialized as transport views.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageWorkSnapshot {
    pub generation: String,
    pub records: Vec<WorkRecord>,
    pub limits: Option<ReactionLimits>,
    pub roots: BTreeMap<String, u32>,
    pub retry_epochs: RetryEpochs,
    pub operator: OperatorLedger,
}
impl StorageWorkSnapshot {
    pub fn validate(&self, max_records: usize, max_bytes: usize) -> Result<()> {
        if max_records == 0 || max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        WorkVersion {
            generation: self.generation.clone(),
            revision: 0,
        }
        .validate()?;
        if self
            .records
            .len()
            .checked_add(self.operator.receipts.len())
            .ok_or(Error::TooLarge)?
            > max_records
            || serde_json::to_vec(self).map_err(|_| Error::Storage)?.len() > max_bytes
        {
            return Err(Error::TooLarge);
        }
        let mut handles = BTreeSet::new();
        for record in &self.records {
            if !handles.insert(WorkHandle::from_work_id(&record.pending.id)) {
                return Err(Error::Storage);
            }
        }
        self.retry_epochs.validate()?;
        self.operator.validate_archive()?;
        Ok(())
    }
    pub fn version(&self, record: &WorkRecord) -> WorkVersion {
        WorkVersion {
            generation: self.generation.clone(),
            revision: record.revision,
        }
    }
}
impl StorageState {
    pub fn operator_receipt_count(&self) -> usize {
        self.operator.receipts.len()
    }
    /// Pure candidate transition; the adapter publishes the returned state in one transaction.
    pub fn control_work(&mut self, control: &StorageWorkControl) -> Result<WorkControlReceipt> {
        use crate::operator::receipts::{check_identifier, receipt_identity};
        use crate::operator::{
            OPERATOR_PROTOCOL_VERSION, WorkControlPrior, WorkControlResult, WorkScope,
        };
        control.request.validate()?;
        check_identifier(&control.principal)?;
        self.retry_epochs
            .check(control.request.retry_epoch, true, false)?;
        if let Some(receipt) = self.operator.receipt(&control.principal, &control.request) {
            if receipt.request != control.request {
                return Err(Error::IdentityMismatch);
            }
            let mut replay = receipt.clone();
            replay.result.replayed = true;
            return Ok(replay);
        }
        self.retry_epochs
            .check(control.request.retry_epoch, false, false)?;
        let evidence = match &control.decision {
            WorkControlDecision::DeliveryAccepted { evidence }
            | WorkControlDecision::DeliveryNotAccepted { evidence } => {
                check_identifier(evidence)?;
                Some(evidence.clone())
            }
            _ => None,
        };
        let records = self.work.records();
        let mut matches = records
            .iter()
            .filter(|r| WorkHandle::from_work_id(&r.pending.id) == control.request.handle);
        let record = matches.next().ok_or(Error::Missing)?;
        if matches.next().is_some() {
            return Err(Error::Storage);
        }
        if control.request.expected.generation != self.generation
            || control.request.expected.revision != record.revision
        {
            return Err(Error::Conflict);
        }
        self.retry_epochs
            .check(record.pending.cause.retry_epoch, true, false)?;
        let scope = WorkScope::from_record(record)?;
        let prior = WorkControlPrior::from_record(record);
        let id = record.pending.id.clone();
        let mut next = self.clone();
        let outcome = next.work.control(&id, control)?;
        let updated = next
            .work
            .records()
            .into_iter()
            .find(|r| r.pending.id == id)
            .ok_or(Error::Storage)?;
        let result = WorkControlResult {
            protocol_version: OPERATOR_PROTOCOL_VERSION,
            handle: control.request.handle.clone(),
            version: WorkVersion {
                generation: self.generation.clone(),
                revision: updated.revision,
            },
            key: control.request.key.clone(),
            operation: control.request.operation.clone(),
            outcome,
            replayed: false,
        };
        let receipt = WorkControlReceipt {
            principal: control.principal.clone(),
            request: control.request.clone(),
            result,
            now: control.now,
            scope,
            prior,
            evidence,
        };
        next.operator.receipts.insert(
            receipt_identity(&control.principal, &control.request),
            receipt.clone(),
        );
        next.operator.validate_archive()?;
        next.work.validate_retry_epochs(next.retry_epochs)?;
        *self = next;
        Ok(receipt)
    }
    /// Bound and charge the complete raw ledger envelope before public projection.
    pub fn work_snapshot(
        &self,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<StorageWorkSnapshot> {
        if max_records == 0 || max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        let snapshot = StorageWorkSnapshot {
            generation: self.generation.clone(),
            records: self.work.records(),
            limits: self.work.policy(),
            roots: self.work.root_usage(),
            retry_epochs: self.retry_epochs,
            operator: self.operator.clone(),
        };
        snapshot.validate(max_records, max_bytes)?;
        Ok(snapshot)
    }
}
