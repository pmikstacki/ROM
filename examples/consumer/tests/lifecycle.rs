use rom::{
    Actor, Bundle, Capabilities, Command, Key, Receipt, Resource, Result, Row, Runtime, Storage,
};
use rom_consumer::{Task, task_policy};
use rom_sqlite::Sqlite;
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct Gate {
    started: tokio::sync::Notify,
    open: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            started: tokio::sync::Notify::new(),
            open: Mutex::new(false),
            wake: Condvar::new(),
        })
    }
    fn wait(&self) {
        self.started.notify_one();
        let mut o = self.open.lock().unwrap();
        while !*o {
            o = self.wake.wait(o).unwrap();
        }
    }
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
struct Blocking {
    inner: Sqlite,
    gate: Arc<Gate>,
    block: AtomicBool,
}
impl Storage for Blocking {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        if self.block.swap(false, Ordering::SeqCst) {
            self.gate.wait();
        }
        self.inner.load(key)
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(kind, max_rows, max_bytes)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.inner.receipt(id)
    }
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        self.inner.commit(b)
    }
}
fn actor() -> Actor {
    Actor::trusted("local", "alice")
}
fn task() -> Task {
    Task {
        owner: "alice".into(),
        title: "one".into(),
        done: false,
        note: None,
    }
}
async fn blocked() -> (
    Runtime,
    Arc<Blocking>,
    tokio::task::JoinHandle<Result<rom::Snapshot<Task>>>,
) {
    let store = Arc::new(Blocking {
        inner: Sqlite::open(":memory:").unwrap(),
        gate: Gate::new(),
        block: AtomicBool::new(false),
    });
    let rom = Runtime::builder()
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let runtime = rom.clone();
    let work = tokio::spawn(async move {
        let mut t = task();
        t.done = true;
        runtime
            .execute(
                &actor(),
                Command::replace("t", t)
                    .at_revision(1)
                    .idempotency("replace"),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), store.gate.started.notified())
        .await
        .unwrap();
    (rom, store, work)
}
#[tokio::test]
async fn cancelled_shutdown_can_be_joined_again() {
    let (rom, store, work) = blocked().await;
    let r = rom.clone();
    let mut first = tokio::spawn(async move { r.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut first)
            .await
            .is_err()
    );
    first.abort();
    let _ = first.await;
    let r = rom.clone();
    let mut second = tokio::spawn(async move { r.shutdown().await });
    let premature = tokio::time::timeout(Duration::from_millis(10), &mut second)
        .await
        .is_ok();
    store.gate.release();
    if !premature {
        second.await.unwrap().unwrap();
    }
    let _ = work.await;
    assert!(
        !premature,
        "second shutdown forgot accepted work after first waiter cancellation"
    );
}
#[tokio::test]
async fn concurrent_shutdown_waits_for_same_work() {
    let (rom, store, work) = blocked().await;
    let r = rom.clone();
    let mut first = tokio::spawn(async move { r.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut first)
            .await
            .is_err()
    );
    let r = rom.clone();
    let mut second = tokio::spawn(async move { r.shutdown().await });
    let premature = tokio::time::timeout(Duration::from_millis(10), &mut second)
        .await
        .is_ok();
    store.gate.release();
    first.await.unwrap().unwrap();
    if !premature {
        second.await.unwrap().unwrap();
    }
    let _ = work.await;
    assert!(
        !premature,
        "concurrent shutdown returned while accepted work is running"
    );
}
#[tokio::test(flavor = "current_thread")]
async fn blocking_storage_does_not_block_tokio_heartbeat() {
    let store = Arc::new(Blocking {
        inner: Sqlite::open(":memory:").unwrap(),
        gate: Gate::new(),
        block: AtomicBool::new(false),
    });
    let rom = Runtime::builder()
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let r = rom.clone();
    let read = tokio::spawn(async move { r.read::<Task>(&actor(), "t").await });
    tokio::time::timeout(Duration::from_secs(5), store.gate.started.notified())
        .await
        .unwrap();
    // The gated synchronous driver is still running while the single Tokio worker advances.
    tokio::time::timeout(
        Duration::from_secs(1),
        tokio::time::sleep(Duration::from_millis(2)),
    )
    .await
    .unwrap();
    assert!(!read.is_finished());
    store.gate.release();
    assert_eq!(read.await.unwrap().unwrap().revision, 1);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn cancelled_read_keeps_io_capacity_until_storage_completes() {
    let store = Arc::new(Blocking {
        inner: Sqlite::open(":memory:").unwrap(),
        gate: Gate::new(),
        block: AtomicBool::new(false),
    });
    let rom = Runtime::builder()
        .limits(rom::Limits {
            io_jobs: 1,
            ..rom::Limits::default()
        })
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let r = rom.clone();
    let read = tokio::spawn(async move { r.read::<Task>(&actor(), "t").await });
    tokio::time::timeout(Duration::from_secs(5), store.gate.started.notified())
        .await
        .unwrap();
    read.abort();
    let _ = read.await;
    assert_eq!(rom.available_io_capacity(), 0);
    assert!(matches!(
        rom.query(&actor(), &Task::done_field().equals(false)).await,
        Err(rom::Error::Overloaded)
    ));
    store.gate.release();
    rom.shutdown().await.unwrap();
    assert_eq!(rom.available_io_capacity(), 1);
}
#[tokio::test]
async fn adapter_panic_is_terminal_and_never_success() {
    struct PanicStore(Sqlite);
    impl Storage for PanicStore {
        fn capabilities(&self) -> Capabilities {
            self.0.capabilities()
        }
        fn load(&self, _: &Key) -> Result<Option<Row>> {
            panic!("injected adapter panic")
        }
        fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
            panic!("injected adapter panic")
        }
        fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
            self.0.receipt(id)
        }
        fn commit(&self, b: &Bundle) -> Result<Receipt> {
            self.0.commit(b)
        }
    }
    let rom = Runtime::builder()
        .resource(Task::definition().policy(task_policy))
        .build(
            Arc::new(PanicStore(Sqlite::open(":memory:").unwrap())),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        rom.read::<Task>(&actor(), "t").await,
        Err(rom::Error::Panicked)
    ));
    assert!(matches!(rom.shutdown().await, Err(rom::Error::Panicked)));
    assert!(matches!(
        rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
            .await,
        Err(rom::Error::Closed) | Err(rom::Error::Panicked)
    ));
}
#[derive(Default)]
struct TestClock(std::sync::atomic::AtomicU64);
impl rom::Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[tokio::test]
async fn expired_actor_denied_at_result_and_live_delivery() {
    let clock = Arc::new(TestClock::default());
    let store = Arc::new(Blocking {
        inner: Sqlite::open(":memory:").unwrap(),
        gate: Gate::new(),
        block: AtomicBool::new(false),
    });
    let rom = Runtime::builder()
        .clock(clock.clone())
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = actor().expires_at(10);
    rom.execute(&actor, Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    let mut live = rom
        .live(&actor, Task::done_field().equals(false))
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let r = rom.clone();
    let a = actor.clone();
    let read = tokio::spawn(async move { r.read::<Task>(&a, "t").await });
    tokio::time::timeout(Duration::from_secs(5), store.gate.started.notified())
        .await
        .unwrap();
    clock.0.store(10, Ordering::SeqCst);
    store.gate.release();
    assert!(matches!(read.await.unwrap(), Err(rom::Error::Denied)));
    assert!(matches!(live.changed().await, Err(rom::Error::Denied)));
    assert!(matches!(
        rom.read::<Task>(&actor, "missing").await,
        Err(rom::Error::Denied)
    ));
    assert!(matches!(
        rom.execute(&actor, Command::create("t", task()).idempotency("create"))
            .await,
        Err(rom::Error::Denied)
    ));
    assert_eq!(store.inner.counts().unwrap(), [1, 1, 1, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn expired_actor_cannot_commit_an_already_admitted_proposal() {
    let clock = Arc::new(TestClock::default());
    let store = Arc::new(Blocking {
        inner: Sqlite::open(":memory:").unwrap(),
        gate: Gate::new(),
        block: AtomicBool::new(false),
    });
    let rom = Runtime::builder()
        .clock(clock.clone())
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = actor().expires_at(10);
    rom.execute(&actor, Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    store.block.store(true, Ordering::SeqCst);
    let r = rom.clone();
    let a = actor.clone();
    let work = tokio::spawn(async move {
        let mut value = task();
        value.done = true;
        r.execute(
            &a,
            Command::replace("t", value)
                .at_revision(1)
                .idempotency("replace"),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), store.gate.started.notified())
        .await
        .unwrap();
    clock.0.store(10, Ordering::SeqCst);
    store.gate.release();
    assert!(matches!(work.await.unwrap(), Err(rom::Error::Denied)));
    assert_eq!(store.inner.counts().unwrap(), [1, 1, 1, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn shutdown_races_admission_without_orphan() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(Task::definition().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(17));
    let mut jobs = Vec::new();
    for i in 0..16 {
        let r = rom.clone();
        let b = barrier.clone();
        jobs.push(tokio::spawn(async move {
            b.wait().await;
            r.execute(
                &actor(),
                Command::create(&format!("t{i}"), task()).idempotency("create"),
            )
            .await
        }));
    }
    barrier.wait().await;
    rom.shutdown().await.unwrap();
    for job in jobs {
        assert!(matches!(
            job.await.unwrap(),
            Ok(_) | Err(rom::Error::Closed) | Err(rom::Error::Overloaded)
        ));
    }
    let counts = store.counts().unwrap();
    assert_eq!(counts[0], counts[1]);
    assert_eq!(counts[1], counts[2]);
    assert_eq!(counts[3], 0);
    assert_eq!(rom.available_capacity(), 8);
    assert_eq!(rom.available_io_capacity(), 8);
}
#[tokio::test]
async fn policy_panic_stops_runtime_without_partial_state() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(Task::definition().policy(|_, _, _| panic!("injected policy panic")))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    assert!(matches!(
        rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
            .await,
        Err(rom::Error::Panicked)
    ));
    assert_eq!(store.counts().unwrap(), [0; 4]);
    assert!(matches!(
        rom.read::<Task>(&actor(), "t").await,
        Err(rom::Error::Panicked)
    ));
    assert!(matches!(rom.shutdown().await, Err(rom::Error::Panicked)));
}
