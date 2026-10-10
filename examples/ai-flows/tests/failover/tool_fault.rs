//! Public Storage forwarding fixture: one application commit succeeds, then its acknowledgement is lost.
use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub struct ActionFault {
    pub inner: Arc<dyn Storage>,
    pub armed: AtomicBool,
    pub publication: bool,
}
impl Storage for ActionFault {
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
        let row = &bundle.receipt.row;
        let selected = if self.publication {
            row.key.kind == "consumer_article_editions"
                && row
                    .value
                    .as_ref()
                    .is_some_and(|value| value["prepared"] == true)
        } else {
            row.key.kind == "consumer_triage_tickets"
                && row
                    .value
                    .as_ref()
                    .is_some_and(|value| value["classification"] == "urgent")
        };
        if selected && self.armed.swap(false, Ordering::SeqCst) {
            self.inner.commit(bundle)?;
            return Err(Error::Unknown);
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
