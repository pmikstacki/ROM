//! Transparent fixture wrapper. Bookkeeping never holds a lock during adapter operations.
use crate::QueueObservation;
use crate::storage_calls::{Call, Snapshot, StorageCalls};
use rom::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;
pub struct ObservedStorage {
    inner: Arc<dyn Storage>,
    observations: Mutex<QueueObservation>,
    calls: StorageCalls,
    batches: crate::work_batch_observation::WorkBatchObservation,
    #[cfg(test)]
    query_request: Mutex<Option<(StorageQuery, QueryBounds)>>,
}
impl ObservedStorage {
    pub fn new(inner: Arc<dyn Storage>) -> Self {
        Self {
            inner,
            observations: Mutex::new(QueueObservation::new(12000)),
            calls: StorageCalls::default(),
            batches: crate::work_batch_observation::WorkBatchObservation::default(),
            #[cfg(test)]
            query_request: Mutex::new(None),
        }
    }
    #[cfg(test)]
    pub(crate) fn take_query_request(&self) -> Option<(StorageQuery, QueryBounds)> {
        self.query_request.lock().unwrap().take()
    }
    pub fn calls(&self) -> Option<Vec<Snapshot>> {
        self.calls.snapshot()
    }
    pub fn work_batches(&self) -> Option<crate::work_batch_observation::Snapshot> {
        self.batches.snapshot()
    }
    pub fn timings(&self) -> Option<(Vec<u64>, usize, usize)> {
        self.observations
            .lock()
            .ok()
            .map(|q| (q.samples().to_vec(), q.missing(), q.rejected()))
    }
    fn acknowledged_claims<'a>(&self, claims: impl IntoIterator<Item = &'a WorkClaim>) {
        let confirmed = Instant::now();
        if let Ok(mut queue) = self.observations.lock() {
            for claim in claims {
                queue.claimed(&claim.work.pending.id, confirmed);
            }
        }
    }
}
impl Storage for ObservedStorage {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.calls
            .measure(Call::AcquireOwner, || self.inner.acquire_owner())
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.calls
            .measure(Call::RetryEpochs, || self.inner.retry_epochs())
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.calls
            .measure(Call::Register, || self.inner.register(descriptors))
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        let call = match &update {
            WorkUpdate::Claim { .. } => Call::Claim,
            WorkUpdate::Materialize { .. } => Call::Materialize,
            WorkUpdate::Finish { .. } => Call::Finish,
            WorkUpdate::DeliveryStarted { .. } => Call::DeliveryStarted,
            WorkUpdate::DeliveryFinished { .. } => Call::DeliveryFinished,
        };
        let result = self
            .calls
            .measure(call, || self.inner.reaction_update(update));
        if let Ok(WorkResult::Claimed(claim)) = &result {
            self.acknowledged_claims(std::iter::once(claim.as_ref()));
        }
        result
    }
    fn reaction_claim_prefix(&self, now: u64, max_claims: usize) -> Result<Vec<WorkClaim>> {
        let started = Instant::now();
        let result = self.calls.measure(Call::ClaimPrefix, || {
            self.inner.reaction_claim_prefix(now, max_claims)
        });
        self.batches.prefix(&result, started.elapsed());
        if let Ok(claims) = &result {
            self.acknowledged_claims(claims.iter());
        }
        result
    }
    fn reaction_claim_live(&self, claim: &ClaimKey, now: u64) -> Result<Option<bool>> {
        self.calls.measure(Call::ClaimLive, || {
            self.inner.reaction_claim_live(claim, now)
        })
    }
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        let width = updates.len();
        let started = Instant::now();
        let result = self.calls.measure(Call::AtomicWorkUpdates, || {
            self.inner.reaction_updates_atomic(updates)
        });
        self.batches.atomic(width, &result, started.elapsed());
        if let Ok(results) = &result {
            self.acknowledged_claims(results.iter().filter_map(|result| match result {
                WorkResult::Claimed(claim) => Some(claim.as_ref()),
                WorkResult::Changed | WorkResult::Idle => None,
            }));
        }
        result
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.calls
            .measure(Call::ReactionRecords, || self.inner.reaction_records())
    }
    fn supports_operator(&self) -> bool {
        self.inner.supports_operator()
    }
    fn work_snapshot(&self, records: usize, bytes: usize) -> Result<StorageWorkSnapshot> {
        self.calls.measure(Call::WorkSnapshot, || {
            self.inner.work_snapshot(records, bytes)
        })
    }
    fn control_work(&self, control: &StorageWorkControl) -> Result<WorkControlReceipt> {
        self.calls
            .measure(Call::ControlWork, || self.inner.control_work(control))
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.calls.measure(Call::Load, || self.inner.load(key))
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.calls
            .measure(Call::Snapshot, || self.inner.snapshot(kind, rows, bytes))
    }
    fn query_read(&self, query: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        #[cfg(test)]
        {
            *self.query_request.lock().unwrap() = Some((query.clone(), bounds));
        }
        self.calls
            .measure(Call::QueryRead, || self.inner.query_read(query, bounds))
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        self.calls
            .measure(Call::Receipt, || self.inner.receipt(identity))
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        if let Ok(mut q) = self.observations.lock() {
            for pending in &bundle.reactions {
                // A full observer must not change the adapter's commit decision.
                if q.offer(&pending.id).is_err() {
                    q.mark_rejected();
                }
            }
        }
        let result = self
            .calls
            .measure(Call::Commit, || self.inner.commit(bundle));
        let confirmed = Instant::now();
        if let Ok(mut q) = self.observations.lock() {
            for pending in &bundle.reactions {
                if result.is_ok() {
                    q.committed(&pending.id, confirmed);
                } else {
                    q.failed(&pending.id);
                }
            }
        }
        result
    }
    fn supports_journal(&self) -> bool {
        self.inner.supports_journal()
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.calls
            .measure(Call::JournalHead, || self.inner.journal_head(kind))
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        rows: usize,
        bytes: usize,
    ) -> Result<JournalPage> {
        self.calls.measure(Call::Journal, || {
            self.inner.journal(kind, after, rows, bytes)
        })
    }
}
#[cfg(test)]
#[path = "observed_storage_forwarding_tests.rs"]
mod forwarding_tests;
