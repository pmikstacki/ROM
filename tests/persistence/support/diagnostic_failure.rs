//! Transparent actual-database wrapper with one explicit read failure boundary.
use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub struct ReadFailure {
    pub inner: Arc<dyn Storage>,
    pub mode: AtomicU64,
}
impl Storage for ReadFailure {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        if self.mode.load(Ordering::SeqCst) == 1 {
            Err(Error::Storage)
        } else {
            self.inner.load(key)
        }
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        if self.mode.load(Ordering::SeqCst) == 2 {
            Err(Error::Storage)
        } else {
            self.inner.receipt(identity)
        }
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.inner.commit(bundle)
    }
}
