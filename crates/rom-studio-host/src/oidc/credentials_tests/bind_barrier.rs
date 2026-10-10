//! Transparent native storage wrapper with a private finite authoritative-read barrier.
use rom::*;
use std::sync::{Arc, Condvar, Mutex};
#[derive(Default)]
struct Barrier {
    armed: bool,
    released: bool,
}
pub struct Controlled {
    inner: Arc<dyn Storage>,
    barrier: Mutex<Barrier>,
    changed: Condvar,
    pub started: tokio::sync::Notify,
}
impl Controlled {
    pub fn new(inner: Arc<dyn Storage>) -> Self {
        Self {
            inner,
            barrier: Mutex::new(Barrier::default()),
            changed: Condvar::new(),
            started: tokio::sync::Notify::new(),
        }
    }
    pub fn arm(&self) {
        *self.barrier.lock().unwrap() = Barrier {
            armed: true,
            released: false,
        };
    }
    pub fn release(&self) {
        self.barrier.lock().unwrap().released = true;
        self.changed.notify_all();
    }
}
impl Storage for Controlled {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.inner.retry_epochs()
    }
    fn register(&self, d: &[Descriptor]) -> Result<()> {
        self.inner.register(d)
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, u: WorkUpdate) -> Result<WorkResult> {
        self.inner.reaction_update(u)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.inner.reaction_records()
    }
    fn supports_operator(&self) -> bool {
        self.inner.supports_operator()
    }
    fn work_snapshot(&self, r: usize, b: usize) -> Result<StorageWorkSnapshot> {
        self.inner.work_snapshot(r, b)
    }
    fn control_work(&self, c: &StorageWorkControl) -> Result<WorkControlReceipt> {
        self.inner.control_work(c)
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        if key.kind == "users" {
            let mut barrier = self.barrier.lock().unwrap();
            if barrier.armed {
                barrier.armed = false;
                self.started.notify_one();
                let (barrier, timeout) = self
                    .changed
                    .wait_timeout_while(barrier, std::time::Duration::from_secs(5), |b| !b.released)
                    .unwrap();
                if timeout.timed_out() && !barrier.released {
                    return Err(Error::Storage);
                }
                drop(barrier);
            }
        }
        self.inner.load(key)
    }
    fn snapshot(&self, k: &str, r: usize, b: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(k, r, b)
    }
    fn query_read(&self, q: &StorageQuery, b: QueryBounds) -> Result<QueryRead> {
        self.inner.query_read(q, b)
    }
    fn receipt(&self, k: &str) -> Result<Option<Receipt>> {
        self.inner.receipt(k)
    }
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        self.inner.commit(b)
    }
    fn supports_journal(&self) -> bool {
        self.inner.supports_journal()
    }
    fn journal_head(&self, k: &str) -> Result<JournalCursor> {
        self.inner.journal_head(k)
    }
    fn journal(
        &self,
        k: &str,
        a: Option<&JournalCursor>,
        r: usize,
        b: usize,
    ) -> Result<JournalPage> {
        self.inner.journal(k, a, r, b)
    }
}
