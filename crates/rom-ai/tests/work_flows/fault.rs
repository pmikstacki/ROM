//! Transparent actual-adapter wrapper with one confirmed settlement noncommit.
use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub struct SettlementFault {
    pub inner: Arc<dyn Storage>,
    pub armed: AtomicBool,
    pub after_commit: bool,
}
impl Storage for SettlementFault {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.inner.reaction_update(update)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.inner.reaction_records()
    }
    fn supports_operator(&self) -> bool {
        self.inner.supports_operator()
    }
    fn work_snapshot(&self, rows: usize, bytes: usize) -> Result<StorageWorkSnapshot> {
        self.inner.work_snapshot(rows, bytes)
    }
    fn control_work(&self, control: &StorageWorkControl) -> Result<WorkControlReceipt> {
        self.inner.control_work(control)
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.inner.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(kind, rows, bytes)
    }
    fn query_read(&self, request: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        self.inner.query_read(request, bounds)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        self.inner.receipt(identity)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        let settled = bundle.receipt.row.key.kind == "ai_budgets"
            && bundle
                .receipt
                .row
                .value
                .as_ref()
                .and_then(|value| value.get("encoded"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|encoded| encoded.contains("\"Settled\""));
        if settled && self.armed.swap(false, Ordering::SeqCst) {
            if self.after_commit {
                self.inner.commit(bundle)?;
                return Err(Error::Unknown);
            }
            return Err(Error::NotCommitted);
        }
        self.inner.commit(bundle)
    }
    fn supports_journal(&self) -> bool {
        self.inner.supports_journal()
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.inner.journal_head(kind)
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        rows: usize,
        bytes: usize,
    ) -> Result<JournalPage> {
        self.inner.journal(kind, after, rows, bytes)
    }
}
