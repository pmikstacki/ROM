//! Throwaway ROM-owned deduplication experiment; no public cache/plugin API.
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};

#[derive(Clone, Copy, Debug, Serialize)]
enum Mode {
    Baseline,
    Flight,
    Moka,
    Quick,
}
impl Mode {
    const ALL: [Self; 4] = [Self::Baseline, Self::Flight, Self::Moka, Self::Quick];
    fn flight(self) -> bool {
        !matches!(self, Self::Baseline)
    }
}
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
struct Key {
    tenant: String,
    principal: String,
    operation: String,
    id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Input {
    target: String,
    amount: i64,
    payload: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Outcome {
    value: i64,
    revision: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Error {
    Conflict,
    Denied,
    Busy,
    Unknown,
    TooLarge,
}
type Reply = Result<Outcome, Error>;
#[derive(Clone)]
struct Cached {
    input: Input,
    outcome: Outcome,
    until: u64,
}
enum Cache {
    None,
    Moka(moka::sync::Cache<Key, Arc<Cached>>),
    Quick(quick_cache::sync::Cache<Key, Arc<Cached>>),
}
impl Cache {
    fn new(mode: Mode, capacity: usize) -> Self {
        match mode {
            Mode::Moka => Self::Moka(moka::sync::Cache::new(capacity as u64)),
            Mode::Quick => Self::Quick(quick_cache::sync::Cache::new(capacity)),
            _ => Self::None,
        }
    }
    fn get(&self, key: &Key, now: u64) -> Option<Arc<Cached>> {
        let value = match self {
            Self::None => None,
            Self::Moka(c) => c.get(key),
            Self::Quick(c) => c.get(key),
        };
        if value.as_ref().is_some_and(|v| v.until <= now) {
            self.remove(key);
            None
        } else {
            value
        }
    }
    fn insert(&self, key: Key, value: Cached) {
        let value = Arc::new(value);
        match self {
            Self::None => {}
            Self::Moka(c) => c.insert(key, value),
            Self::Quick(c) => c.insert(key, value),
        }
    }
    fn remove(&self, key: &Key) {
        match self {
            Self::None => {}
            Self::Moka(c) => c.invalidate(key),
            Self::Quick(c) => {
                c.remove(key);
            }
        }
    }
}
struct Flight {
    input: Input,
    receiver: watch::Receiver<Option<Reply>>,
}
#[derive(Default)]
struct Stats {
    attempts: AtomicU64,
    commits: AtomicU64,
    hits: AtomicU64,
    joins: AtomicU64,
}
struct Gate {
    entered: Semaphore,
    release: Semaphore,
    joined: Semaphore,
}
impl Gate {
    #[cfg(test)]
    fn new() -> Self {
        Self {
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            joined: Semaphore::new(0),
        }
    }
}
struct Shared {
    mode: Mode,
    path: PathBuf,
    cache: Cache,
    flights: Mutex<HashMap<Key, Flight>>,
    permits: Arc<Semaphore>,
    callers: Semaphore,
    connection: Mutex<Connection>,
    denied: Mutex<HashSet<String>>,
    stats: Stats,
    start: Instant,
    offset: AtomicU64,
    ttl_ms: u64,
    unknown: AtomicBool,
    panic_worker: AtomicBool,
    gate: Mutex<Option<Arc<Gate>>>,
}
impl Shared {
    fn now(&self) -> u64 {
        (self.start.elapsed().as_millis() as u64)
            .saturating_add(self.offset.load(Ordering::Relaxed))
    }
    fn deliver(&self, key: &Key, result: Reply) -> Reply {
        if self.denied.lock().unwrap().contains(&key.principal) {
            Err(Error::Denied)
        } else {
            result
        }
    }
}
struct Job {
    key: Key,
    input: Input,
    sender: watch::Sender<Option<Reply>>,
    _permit: OwnedSemaphorePermit,
}
#[derive(Clone)]
struct Runtime {
    shared: Arc<Shared>,
    sender: mpsc::UnboundedSender<Job>,
}
impl Runtime {
    fn new(path: &Path, mode: Mode, bound: usize, capacity: usize, ttl_ms: u64) -> Self {
        initialize(path);
        let shared = Arc::new(Shared {
            mode,
            path: path.into(),
            cache: Cache::new(mode, capacity),
            flights: Mutex::new(HashMap::new()),
            permits: Arc::new(Semaphore::new(bound)),
            callers: Semaphore::new(bound.saturating_mul(32)),
            connection: Mutex::new(open(path)),
            denied: Mutex::new(HashSet::new()),
            stats: Stats::default(),
            start: Instant::now(),
            offset: AtomicU64::new(0),
            ttl_ms,
            unknown: AtomicBool::new(false),
            panic_worker: AtomicBool::new(false),
            gate: Mutex::new(None),
        });
        let (sender, mut receiver) = mpsc::unbounded_channel::<Job>();
        let state = shared.clone();
        // Supervisor owns admitted jobs. Closing all Runtime handles drains accepted jobs.
        // The queue is unbounded mechanically, but every entry owns one of `bound` permits.
        tokio::spawn(async move {
            let mut jobs = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    job = receiver.recv() => match job {
                        Some(job) => { let state = state.clone(); jobs.spawn(async move { execute(state,job).await }); },
                        None => break,
                    },
                    _ = jobs.join_next(), if !jobs.is_empty() => {},
                }
            }
            while jobs.join_next().await.is_some() {}
        });
        Self { shared, sender }
    }
    // Resource authors call an ordinary action; experiment mode stays inside runtime construction.
    async fn call(&self, key: Key, input: Input) -> Reply {
        let _caller = self.shared.callers.try_acquire().map_err(|_| Error::Busy)?;
        if [
            &key.tenant,
            &key.principal,
            &key.operation,
            &key.id,
            &input.target,
        ]
        .iter()
        .any(|s| s.len() > 256)
            || input.payload.len() > 1024 * 1024
        {
            return Err(Error::TooLarge);
        }
        if self.shared.denied.lock().unwrap().contains(&key.principal) {
            return Err(Error::Denied);
        }
        let mut receiver = {
            let mut flights = self.shared.flights.lock().unwrap();
            if let Some(cached) = self.shared.cache.get(&key, self.shared.now()) {
                self.shared.stats.hits.fetch_add(1, Ordering::Relaxed);
                return self.shared.deliver(
                    &key,
                    if cached.input == input {
                        Ok(cached.outcome.clone())
                    } else {
                        Err(Error::Conflict)
                    },
                );
            }
            if let Some(flight) = flights.get(&key) {
                if flight.input != input {
                    return Err(Error::Conflict);
                }
                self.shared.stats.joins.fetch_add(1, Ordering::Relaxed);
                if let Some(gate) = self.shared.gate.lock().unwrap().as_ref() {
                    gate.joined.add_permits(1);
                }
                flight.receiver.clone()
            } else {
                let permit = self
                    .shared
                    .permits
                    .clone()
                    .try_acquire_owned()
                    .map_err(|_| Error::Busy)?;
                let (sender, receiver) = watch::channel(None);
                if self.shared.mode.flight() {
                    flights.insert(
                        key.clone(),
                        Flight {
                            input: input.clone(),
                            receiver: receiver.clone(),
                        },
                    );
                }
                self.sender
                    .send(Job {
                        key: key.clone(),
                        input,
                        sender,
                        _permit: permit,
                    })
                    .map_err(|_| Error::Unknown)?;
                receiver
            }
        };
        loop {
            let result = receiver.borrow_and_update().clone();
            if let Some(result) = result {
                return self.shared.deliver(&key, result);
            }
            if receiver.changed().await.is_err() {
                return Err(Error::Unknown);
            }
        }
    }
}
async fn execute(shared: Arc<Shared>, job: Job) {
    let state = shared.clone();
    let key = job.key.clone();
    let input = job.input.clone();
    // The outer supervisor owns cleanup even when the action task unwinds.
    let result = tokio::spawn(async move {
        let gate = state.gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.entered.add_permits(1);
            gate.release.acquire().await.unwrap().forget();
        }
        assert!(
            !state.panic_worker.swap(false, Ordering::SeqCst),
            "injected action panic"
        );
        tokio::task::spawn_blocking(move || durable(&state, &key, &input))
            .await
            .unwrap_or(Err(Error::Unknown))
    })
    .await
    .unwrap_or(Err(Error::Unknown));
    {
        let mut flights = shared.flights.lock().unwrap();
        if let Ok(outcome) = &result {
            shared.cache.insert(
                job.key.clone(),
                Cached {
                    input: job.input,
                    outcome: outcome.clone(),
                    until: shared.now().saturating_add(shared.ttl_ms),
                },
            );
        }
        if shared.mode.flight() {
            flights.remove(&job.key);
        }
        job.sender.send_replace(Some(result));
    }
}
fn open(path: &Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.busy_timeout(Duration::from_secs(30)).unwrap();
    conn.pragma_update(None, "synchronous", "FULL").unwrap();
    conn
}
fn initialize(path: &Path) {
    let conn = open(path);
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.execute_batch("CREATE TABLE IF NOT EXISTS resources(tenant TEXT,target TEXT,value INTEGER,revision INTEGER,PRIMARY KEY(tenant,target)); CREATE TABLE IF NOT EXISTS receipts(key TEXT PRIMARY KEY,input TEXT,outcome TEXT); CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY, key TEXT UNIQUE,revision INTEGER,amount INTEGER);").unwrap();
}
fn durable(shared: &Shared, key: &Key, input: &Input) -> Reply {
    shared.stats.attempts.fetch_add(1, Ordering::Relaxed);
    let action = || -> rusqlite::Result<Reply> {
        let mut conn = shared.connection.lock().unwrap();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let identity = serde_json::to_string(key).unwrap();
        // Typed input is serialized once canonically in struct field order, no caller JSON fingerprints.
        let canonical = serde_json::to_string(input).unwrap();
        let old = tx
            .query_row(
                "SELECT input,outcome FROM receipts WHERE key=?",
                [&identity],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((old_input, outcome)) = old {
            return Ok(if old_input == canonical {
                Ok(serde_json::from_str(&outcome).unwrap())
            } else {
                Err(Error::Conflict)
            });
        }
        tx.execute(
            "INSERT OR IGNORE INTO resources VALUES(?,?,0,0)",
            params![key.tenant, input.target],
        )?;
        let (value, revision): (i64, i64) = tx.query_row(
            "SELECT value,revision FROM resources WHERE tenant=? AND target=?",
            params![key.tenant, input.target],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let outcome = Outcome {
            value: value + input.amount,
            revision: revision + 1,
        };
        let changed = tx.execute(
            "UPDATE resources SET value=?,revision=? WHERE tenant=? AND target=? AND revision=?",
            params![
                outcome.value,
                outcome.revision,
                key.tenant,
                input.target,
                revision
            ],
        )?;
        assert_eq!(changed, 1);
        tx.execute(
            "INSERT INTO receipts VALUES(?,?,?)",
            params![
                identity,
                canonical,
                serde_json::to_string(&outcome).unwrap()
            ],
        )?;
        tx.execute(
            "INSERT INTO events(key,revision,amount) VALUES(?,?,?)",
            params![identity, outcome.revision, input.amount],
        )?;
        tx.commit()?;
        shared.stats.commits.fetch_add(1, Ordering::Relaxed);
        // Fault lies after a real COMMIT; this result must never populate the success cache.
        Ok(if shared.unknown.swap(false, Ordering::SeqCst) {
            Err(Error::Unknown)
        } else {
            Ok(outcome)
        })
    };
    action().unwrap_or(Err(Error::Unknown))
}
fn key(id: impl ToString) -> Key {
    Key {
        tenant: "tenant".into(),
        principal: "alice".into(),
        operation: "increment/v1".into(),
        id: id.to_string(),
    }
}
fn input(size: usize) -> Input {
    Input {
        target: "counter".into(),
        amount: 1,
        payload: "x".repeat(size),
    }
}
fn counts(path: &Path) -> (i64, i64, i64) {
    let c = open(path);
    (
        c.query_row("SELECT count(*) FROM receipts", [], |r| r.get(0))
            .unwrap(),
        c.query_row("SELECT count(*) FROM events", [], |r| r.get(0))
            .unwrap(),
        c.query_row("SELECT coalesce(sum(value),0) FROM resources", [], |r| {
            r.get(0)
        })
        .unwrap(),
    )
}
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    if std::env::args().any(|a| a == "bench") {
        benchmark().await;
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = if std::env::args().nth(1).as_deref() == Some("persist-demo") {
        PathBuf::from(
            std::env::args()
                .nth(2)
                .expect("persist-demo requires scratch database path"),
        )
    } else {
        dir.path().join("PROTOTYPE-wipe-me.sqlite")
    };
    let runtime = Runtime::new(&path, Mode::Moka, 64, 1024, 60_000);
    let (a, b) = tokio::join!(
        runtime.call(key("demo"), input(128)),
        runtime.call(key("demo"), input(128))
    );
    println!(
        "First={a:?}; duplicate={b:?}; durable receipts/events/value={:?}",
        counts(&runtime.shared.path)
    );
}

mod bench;
use bench::benchmark;
#[cfg(test)]
mod tests;
