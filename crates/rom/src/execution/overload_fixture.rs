//! Private unit-test fixtures; no native-adapter or identity dependency.
use super::Runtime;
use crate::*;
use std::collections::BTreeMap;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

pub(super) const BOUND: Duration = Duration::from_secs(5);
#[derive(Clone, Debug)]
pub(super) struct Record {
    pub writable: bool,
}
impl Resource for Record {
    const KIND: &'static str = "overload-fixture-records";
    fn descriptor() -> Descriptor {
        Descriptor {
            kind: Self::KIND.into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "writable".into(),
                shape: Shape::Bool,
            }],
        }
    }
    fn normalize_field(name: &str, value: Value) -> Result<Value> {
        if name == "writable" && value.is_boolean() {
            Ok(value)
        } else {
            Err(Error::invalid(Self::KIND, name))
        }
    }
    fn encode(&self) -> Value {
        json!({"writable": self.writable})
    }
    fn decode(value: Value) -> Result<Self> {
        Ok(Self {
            writable: value
                .get("writable")
                .and_then(Value::as_bool)
                .ok_or_else(|| Error::invalid(Self::KIND, "writable"))?,
        })
    }
}
pub(super) fn key(id: &str) -> Key {
    Key {
        kind: Record::KIND.into(),
        id: id.into(),
    }
}
pub(super) fn actor() -> Actor {
    Actor::trusted("overload-unit", "owner").expires_at(100)
}
struct Time;
impl Clock for Time {
    fn now(&self) -> u64 {
        20
    }
}
#[derive(Default)]
struct State {
    rows: BTreeMap<Key, Row>,
    receipts: BTreeMap<String, Receipt>,
}
#[derive(Default)]
pub(super) struct MemoryStore {
    ownership: StorageOwnership,
    state: Mutex<State>,
    pub reads: AtomicUsize,
    pub first_load_pause: Mutex<Option<Arc<NativePause>>>,
}
impl MemoryStore {
    pub fn seeded() -> Arc<Self> {
        let store = Arc::new(Self::default());
        {
            let mut state = store.state.lock().unwrap();
            for id in
                std::iter::once("authority".to_string()).chain((0..32).map(|n| format!("row-{n}")))
            {
                let key = key(&id);
                state.rows.insert(
                    key.clone(),
                    Row {
                        key,
                        revision: 1,
                        value: Some(Record { writable: true }.encode()),
                        protected: ProtectedMetadata::default(),
                    },
                );
            }
        }
        store
    }
}
impl Storage for MemoryStore {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.ownership.acquire()
    }
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        Ok(RetryEpochs::default())
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        if descriptors.iter().any(|d| d != &Record::descriptor()) {
            return Err(Error::Storage);
        }
        Ok(())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn supports_reactions(&self) -> bool {
        true
    }
    fn reaction_update(&self, _: WorkUpdate) -> Result<WorkResult> {
        Ok(WorkResult::Idle)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        Ok(vec![])
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let pause = self
            .first_load_pause
            .lock()
            .map_err(|_| Error::Panicked)?
            .take();
        if let Some(pause) = pause {
            pause.wait(0)?;
        }
        Ok(self
            .state
            .lock()
            .map_err(|_| Error::Storage)?
            .rows
            .get(key)
            .cloned())
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        let selected: Vec<_> = self
            .state
            .lock()
            .map_err(|_| Error::Storage)?
            .rows
            .values()
            .filter(|r| r.key.kind == kind)
            .cloned()
            .collect();
        if selected.len() > rows
            || serde_json::to_vec(&selected)
                .map_err(|_| Error::Storage)?
                .len()
                > bytes
        {
            return Err(Error::TooLarge);
        }
        Ok(selected)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| Error::Storage)?
            .receipts
            .get(identity)
            .cloned())
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        // Only plain Resource commands occur in these counter tests.
        if !bundle.effects.is_empty()
            || !bundle.reactions.is_empty()
            || bundle.completed_work.is_some()
        {
            return Err(Error::Unsupported("unit fixture plain bundles only".into()));
        }
        let mut state = self.state.lock().map_err(|_| Error::Storage)?;
        if let Some(prior) = state.receipts.get(&bundle.receipt.identity) {
            return Ok(prior.clone());
        }
        if state.rows.get(&bundle.receipt.row.key).map(|r| r.revision) != bundle.expected {
            return Err(Error::Conflict);
        }
        if bundle.changed {
            state
                .rows
                .insert(bundle.receipt.row.key.clone(), bundle.receipt.row.clone());
        }
        state
            .receipts
            .insert(bundle.receipt.identity.clone(), bundle.receipt.clone());
        Ok(bundle.receipt.clone())
    }
}
struct CurrentGate;
impl ActorGate for CurrentGate {
    fn check(&self, actor: &Actor, read: &mut dyn AuthorizationRead) -> Result<()> {
        if actor.authority != "overload-unit" || actor.subject != "owner" {
            return Err(Error::Denied);
        }
        let row = read.load(&key("authority"))?.ok_or(Error::Denied)?;
        if row.revision != 1
            || !row
                .value
                .and_then(|v| v.get("writable").and_then(Value::as_bool))
                .unwrap_or(false)
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
pub(super) fn runtime(store: Arc<MemoryStore>) -> Runtime {
    Runtime::builder()
        .clock(Arc::new(Time))
        .actor_gate(Arc::new(CurrentGate))
        .resource(
            Record::definition()
                .policy(|a, _, value| {
                    a.authority == "overload-unit" && a.subject == "owner" && value.writable
                })
                .allow_all_fields(),
        )
        .limits(Limits {
            actions: 32,
            io_jobs: 32,
            ..Limits::default()
        })
        .build(store, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
pub(super) struct NativePause {
    released: Mutex<bool>,
    changed: Condvar,
    pub entered: tokio::sync::mpsc::Sender<usize>,
}
impl NativePause {
    pub fn new(capacity: usize) -> (Arc<Self>, tokio::sync::mpsc::Receiver<usize>) {
        let (entered, received) = tokio::sync::mpsc::channel(capacity);
        (
            Arc::new(Self {
                released: Mutex::new(false),
                changed: Condvar::new(),
                entered,
            }),
            received,
        )
    }
    pub fn wait(&self, index: usize) -> Result<()> {
        self.entered.try_send(index).map_err(|_| Error::Storage)?;
        let guard = self.released.lock().map_err(|_| Error::Panicked)?;
        let (guard, timeout) = self
            .changed
            .wait_timeout_while(guard, BOUND, |r| !*r)
            .map_err(|_| Error::Panicked)?;
        if timeout.timed_out() && !*guard {
            return Err(Error::Storage);
        }
        Ok(())
    }
    pub fn release(&self) {
        if let Ok(mut guard) = self.released.lock() {
            *guard = true;
            self.changed.notify_all();
        }
    }
}
pub(super) struct Release(pub Arc<NativePause>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
pub(super) async fn until(
    mut predicate: impl FnMut() -> bool,
) -> std::result::Result<(), tokio::time::error::Elapsed> {
    tokio::time::timeout(BOUND, async {
        while !predicate() {
            tokio::task::yield_now().await;
        }
    })
    .await
}
