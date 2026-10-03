//! Fault and moving-ledger seams wrapped around real native storage.
use rom::*;
use std::sync::Arc;
pub type SnapshotHook =
    Arc<dyn Fn(StorageWorkSnapshot) -> Result<StorageWorkSnapshot> + Send + Sync>;
pub type ReceiptHook = Arc<dyn Fn(Option<Receipt>) -> Result<Option<Receipt>> + Send + Sync>;
pub type ControlHook = Arc<dyn Fn(WorkControlReceipt) -> Result<WorkControlReceipt> + Send + Sync>;
pub struct Wrapped {
    pub base: Arc<dyn Storage>,
    pub operator_support: bool,
    pub snapshot: Option<SnapshotHook>,
    pub receipt: Option<ReceiptHook>,
    pub control: Option<ControlHook>,
}
impl Wrapped {
    pub fn new(base: Arc<dyn Storage>) -> Self {
        Self {
            base,
            operator_support: true,
            snapshot: None,
            receipt: None,
            control: None,
        }
    }
}
impl Storage for Wrapped {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.base.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.base.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.base.register(descriptors)
    }
    fn supports_reactions(&self) -> bool {
        self.base.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.base.reaction_update(update)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.base.reaction_records()
    }
    fn supports_operator(&self) -> bool {
        self.operator_support && self.base.supports_operator()
    }
    fn work_snapshot(&self, records: usize, bytes: usize) -> Result<StorageWorkSnapshot> {
        let snapshot = self.base.work_snapshot(records, bytes)?;
        match &self.snapshot {
            Some(hook) => hook(snapshot),
            None => Ok(snapshot),
        }
    }
    fn control_work(&self, request: &StorageWorkControl) -> Result<WorkControlReceipt> {
        let receipt = self.base.control_work(request)?;
        match &self.control {
            Some(hook) => hook(receipt),
            None => Ok(receipt),
        }
    }
    fn capabilities(&self) -> Capabilities {
        self.base.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.base.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.base.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        let receipt = self.base.receipt(identity)?;
        match &self.receipt {
            Some(hook) => hook(receipt),
            None => Ok(receipt),
        }
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.base.commit(bundle)
    }
}
