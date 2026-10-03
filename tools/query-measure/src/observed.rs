//! Measurement-only Storage delegation; all runtime authority stays in ROM.
use rom::*;
use rom_sqlite::{QueryExecution, QueryMetrics, Sqlite};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    Automatic,
    Reference,
    Native,
}
impl Mode {
    pub(crate) const ALL: [Self; 3] = [Self::Automatic, Self::Reference, Self::Native];
    pub(crate) fn order(position: usize) -> [Self; 3] {
        use Mode::{Automatic as A, Native as N, Reference as R};
        [
            [A, R, N],
            [A, N, R],
            [R, A, N],
            [R, N, A],
            [N, A, R],
            [N, R, A],
        ][position % 6]
    }
    fn execution(self) -> QueryExecution {
        match self {
            Self::Automatic => QueryExecution::Automatic,
            Self::Reference => QueryExecution::Reference,
            Self::Native => QueryExecution::Native,
        }
    }
}
pub(crate) struct Metrics {
    pub adapter: QueryMetrics,
    pub storage_elapsed_ns: u128,
}
pub(crate) struct Observed {
    pub store: Arc<Sqlite>,
    mode: AtomicU8,
    metrics: Mutex<Option<Metrics>>,
}
impl Observed {
    pub(crate) fn new(store: Arc<Sqlite>) -> Self {
        Self {
            store,
            mode: AtomicU8::new(0),
            metrics: Mutex::new(None),
        }
    }
    pub(crate) fn mode(&self, mode: Mode) -> Result<()> {
        *self.metrics.lock().map_err(|_| Error::Panicked)? = None;
        self.mode.store(
            match mode {
                Mode::Automatic => 0,
                Mode::Reference => 1,
                Mode::Native => 2,
            },
            Ordering::SeqCst,
        );
        Ok(())
    }
    pub(crate) fn take(&self) -> Result<Metrics> {
        self.metrics
            .lock()
            .map_err(|_| Error::Panicked)?
            .take()
            .ok_or(Error::Storage)
    }
}
impl Storage for Observed {
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.store.register(descriptors)
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        self.store.retry_epochs()
    }
    fn capabilities(&self) -> Capabilities {
        self.store.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.store.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.store.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.store.receipt(id)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.store.commit(bundle)
    }
    fn supports_reactions(&self) -> bool {
        self.store.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.store.reaction_update(update)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.store.reaction_records()
    }
    fn supports_journal(&self) -> bool {
        self.store.supports_journal()
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.store.journal_head(kind)
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        rows: usize,
        bytes: usize,
    ) -> Result<JournalPage> {
        self.store.journal(kind, after, rows, bytes)
    }
    fn query_read(&self, request: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        let mode = match self.mode.load(Ordering::SeqCst) {
            0 => Mode::Automatic,
            1 => Mode::Reference,
            2 => Mode::Native,
            _ => return Err(Error::Storage),
        };
        let start = Instant::now();
        let observation = self
            .store
            .query_read_observed(request, bounds, mode.execution())?;
        let elapsed = start.elapsed().as_nanos();
        let mut metrics = self.metrics.lock().map_err(|_| Error::Panicked)?;
        if metrics.is_some() {
            return Err(Error::Storage);
        } // Sequential trials must contain exactly one storage read.
        *metrics = Some(Metrics {
            adapter: observation.metrics,
            storage_elapsed_ns: elapsed,
        });
        Ok(observation.read)
    }
}
