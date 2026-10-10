//! Public Storage forwarding fixture: one Waiting checkpoint succeeds, then its acknowledgement is lost.
use rom::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
pub struct FaultAudit {
    pub armed: AtomicBool,
    pub fired: AtomicBool,
    pub captured: Mutex<Option<Bundle>>,
    pub reopened_receipt: Mutex<Option<Receipt>>,
}
impl FaultAudit {
    pub fn new() -> Self {
        Self {
            armed: AtomicBool::new(true),
            fired: AtomicBool::new(false),
            captured: Mutex::new(None),
            reopened_receipt: Mutex::new(None),
        }
    }
}
pub struct WaitingFault {
    pub inner: Arc<dyn Storage>,
    pub audit: Arc<FaultAudit>,
}
impl Storage for WaitingFault {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)?;
        if let Some(bundle) = self.audit.captured.lock().unwrap().as_ref() {
            *self.audit.reopened_receipt.lock().unwrap() =
                self.inner.receipt(&bundle.receipt.identity)?;
        }
        Ok(())
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
        let selected = row.key.kind == "ai_runs"
            && row
                .value
                .as_ref()
                .and_then(|value| rom_ai::flow::AiRun::decode(value.clone()).ok())
                .and_then(|value| value.record().ok())
                .is_some_and(|record| {
                    matches!(record.state(), rom_ai::flow::RunState::Waiting { .. })
                });
        if selected && self.audit.armed.swap(false, Ordering::SeqCst) {
            let receipt = self.inner.commit(bundle)?;
            assert_eq!(
                receipt, bundle.receipt,
                "exact successful checkpoint receipt"
            );
            assert_eq!(self.inner.receipt(&receipt.identity)?, Some(receipt));
            *self.audit.captured.lock().unwrap() = Some(bundle.clone());
            self.audit.fired.store(true, Ordering::SeqCst);
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
