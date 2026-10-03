use crate::model::*;
use rom::*;
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "PROTOTYPE-compensation-wipe-me-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub struct Time(pub AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl Time {
    pub fn advance(&self) {
        self.0.fetch_add(10, Ordering::SeqCst);
    }
}
#[derive(Default)]
pub struct SimulatedProvider {
    pub script: VecDeque<DeliveryOutcome>,
    pub deliveries: Vec<Value>,
    pub effects: BTreeSet<String>,
}
#[derive(Default)]
pub struct Metrics {
    bundles: AtomicU64,
    acknowledgments: AtomicU64,
    events: AtomicU64,
    loads: AtomicU64,
    snapshots: AtomicU64,
    receipts: AtomicU64,
    work_updates: AtomicU64,
    work_claims: AtomicU64,
    inspections: AtomicU64,
}
#[derive(Clone)]
pub struct Options {
    pub failures: Arc<AtomicUsize>,
    pub metrics: Arc<Metrics>,
    pub clock: Arc<Time>,
    pub provider: Arc<Mutex<SimulatedProvider>>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            failures: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(Metrics::default()),
            clock: Arc::new(Time(AtomicU64::new(1000))),
            provider: Arc::new(Mutex::new(SimulatedProvider::default())),
        }
    }
}
pub struct Fixture {
    pub runtime: Runtime,
    pub store: Arc<dyn Storage>,
    pub sqlite: Option<Arc<rom_sqlite::Sqlite>>,
    pub options: Options,
}
struct FaultStorage {
    inner: Arc<dyn Storage>,
    failures: Arc<AtomicUsize>,
    metrics: Arc<Metrics>,
}
impl Storage for FaultStorage {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.metrics.loads.fetch_add(1, Ordering::Relaxed);
        self.inner.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.metrics.snapshots.fetch_add(1, Ordering::Relaxed);
        self.inner.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, key: &str) -> Result<Option<Receipt>> {
        self.metrics.receipts.fetch_add(1, Ordering::Relaxed);
        self.inner.receipt(key)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.metrics.bundles.fetch_add(1, Ordering::Relaxed);
        if bundle.receipt.row.key.kind == Inventory::KIND
            && self
                .failures
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
        {
            return Err(Error::NotCommitted);
        }
        let receipt = self.inner.commit(bundle)?;
        self.metrics.acknowledgments.fetch_add(1, Ordering::Relaxed);
        if bundle.changed {
            self.metrics.events.fetch_add(1, Ordering::Relaxed);
        }
        Ok(receipt)
    }
    fn supports_reactions(&self) -> bool {
        self.inner.supports_reactions()
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.metrics.work_updates.fetch_add(1, Ordering::Relaxed);
        let result = self.inner.reaction_update(update)?;
        if matches!(result, WorkResult::Claimed(_)) {
            self.metrics.work_claims.fetch_add(1, Ordering::Relaxed);
        }
        Ok(result)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.metrics.inspections.fetch_add(1, Ordering::Relaxed);
        self.inner.reaction_records()
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
impl Fixture {
    pub fn open(backend: &str, path: &Path, options: Options) -> Self {
        let (store, sqlite): (Arc<dyn Storage>, _) = match backend {
            "sqlite" => {
                let store = Arc::new(rom_sqlite::Sqlite::open(path).unwrap());
                (store.clone(), Some(store))
            }
            "redb" => (Arc::new(rom_redb::Redb::open(path).unwrap()), None),
            _ => panic!("unknown backend"),
        };
        let store: Arc<dyn Storage> = Arc::new(FaultStorage {
            inner: store,
            failures: options.failures.clone(),
            metrics: options.metrics.clone(),
        });
        let provider_state = options.provider.clone();
        let runtime=registry().clock(options.clock.clone()).reaction_limits(ReactionLimits{max_depth:4,max_work:64,max_attempts:3,max_fanout:4,..Default::default()})
            .channel(EXTERNAL,provider(),move|delivery|{let state=provider_state.clone();async move{
                let mut state=state.lock().unwrap();let outcome=state.script.pop_front().unwrap_or(DeliveryOutcome::Accepted);
                if matches!(outcome,DeliveryOutcome::Unknown|DeliveryOutcome::Accepted){state.effects.insert(delivery.id.clone());}
                state.deliveries.push(json!({"id":delivery.id,"attempt":delivery.attempt,"outcome":outcome,"provider":"simulated_deduplicating_receiver"}));outcome
            }})
            .build(store.clone(),Runtime::shared_cpu_pool(2).unwrap()).unwrap();
        Self {
            runtime,
            store,
            sqlite,
            options,
        }
    }
    pub async fn create<R: Resource>(
        &self,
        id: &str,
        value: R,
        trace: &mut Vec<Value>,
    ) -> Snapshot<R> {
        let row = self
            .runtime
            .execute(
                &author(),
                Command::create(id, value).idempotency(&format!("seed-{}-{id}", R::KIND)),
            )
            .await
            .unwrap();
        record("create", &row, trace);
        row
    }
    pub async fn act<R: Resource, I: Input>(
        &self,
        id: &str,
        action: Action<R, I>,
        input: I,
        key: &str,
        trace: &mut Vec<Value>,
    ) -> Snapshot<R> {
        let current = self.runtime.read::<R>(&author(), id).await.unwrap();
        let row = self
            .runtime
            .execute(
                &author(),
                Command::action(id, action, input)
                    .at_revision(current.revision)
                    .idempotency(key),
            )
            .await
            .unwrap();
        record(key, &row, trace);
        row
    }
    pub async fn read<R: Resource>(&self, id: &str) -> Snapshot<R> {
        self.runtime.read(&author(), id).await.unwrap()
    }
    pub async fn drain(&self, trace: &mut Vec<Value>) {
        for _ in 0..16 {
            if self.runtime.process_work(32).await.unwrap() == 0 {
                trace.push(json!({"step":"drain_currently_due_work","work":self.work()}));
                return;
            }
        }
        panic!("probe step bound exhausted");
    }
    pub fn work(&self) -> Value {
        json!(self.store.reaction_records().unwrap().iter().map(|w|json!({"id":w.pending.id,"definition":w.pending.definition,"depth":w.pending.cause.depth,"attempts":w.attempts,"state":w.state,"delivery":w.delivery})).collect::<Vec<_>>())
    }
    pub fn stopped(&self, reason: StopReason) -> bool {
        self.store
            .reaction_records()
            .unwrap()
            .iter()
            .any(|w| w.state == WorkState::Stopped(reason.clone()))
    }
    pub fn costs(&self) -> Value {
        let records = self.store.reaction_records().unwrap();
        let m = &self.options.metrics;
        json!({"scope":"whole_case_including_probe_reads_and_clean_reopens", "bundle_calls":m.bundles.load(Ordering::Relaxed), "bundle_acknowledgments":m.acknowledgments.load(Ordering::Relaxed), "changed_events_acknowledged":m.events.load(Ordering::Relaxed), "row_loads":m.loads.load(Ordering::Relaxed), "resource_snapshot_scans":m.snapshots.load(Ordering::Relaxed), "receipt_lookups":m.receipts.load(Ordering::Relaxed), "work_update_calls":m.work_updates.load(Ordering::Relaxed), "claimed_work_steps":m.work_claims.load(Ordering::Relaxed), "ledger_inspections":m.inspections.load(Ordering::Relaxed), "retry_claims":records.iter().map(|r|r.attempts.saturating_sub(1) as u64).sum::<u64>(), "work_records":records.len(), "limits":{"fanout":4,"depth":4,"work_per_root":64,"attempts":3,"drain_iterations":16,"steps_per_drain_iteration":32}})
    }
    pub async fn finish(self) {
        self.runtime.shutdown().await.unwrap();
        drop(self);
    }
}
pub fn record<R: Resource>(step: &str, row: &Snapshot<R>, trace: &mut Vec<Value>) {
    trace.push(json!({"step":step,"kind":R::KIND,"id":row.id,"revision":row.revision,"value":row.value.as_ref().map(Resource::encode)}));
}
pub fn evidence(
    backend: &str,
    case: &str,
    verdict: &str,
    trace: Vec<Value>,
    checks: Value,
) -> Value {
    json!({"case":case,"backend":backend,"passed":true,"verdict":verdict,"external_outcomes":"simulated","trace":trace,"checks":checks})
}

pub async fn finish_result(f: Fixture, mut result: Value) -> Value {
    result["cost"] = f.costs();
    f.finish().await;
    result
}
