use rom::*;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
#[derive(Clone, Resource)]
#[resource(name = "sources")]
struct Source {
    enabled: bool,
    title: String,
}
#[derive(Clone, Resource)]
#[resource(name = "mirrors")]
struct Mirror {
    enabled: bool,
}
static FAIL: AtomicUsize = AtomicUsize::new(0);
const FAILSET: Action<Mirror, bool> = Action::new("failset", |r, v| {
    if FAIL
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
        .is_ok()
    {
        return Err(Error::NotCommitted);
    }
    r.enabled = v;
    Ok(vec![])
});
const SET: Action<Mirror, bool> = Action::new("set", |r, v| {
    r.enabled = v;
    Ok(vec![])
});
const SOURCE_SET: Action<Source, bool> = Action::new("set", |r, v| {
    r.enabled = v;
    Ok(vec![])
});
struct TestClock(AtomicU64);
impl Clock for TestClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn service() -> Actor {
    Actor::trusted("host", "copy").with_kind(PrincipalKind::Service)
}
fn actor() -> Actor {
    Actor::trusted("host", "owner")
}
fn copy(s: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok(vec![Target::new(
        "mirror",
        s.value.as_ref().ok_or(Error::Missing)?.enabled,
    )])
}
fn same(s: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok(vec![Target::new(
        s.id.clone(),
        s.value.as_ref().unwrap().enabled,
    )])
}
fn invert(s: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok(vec![Target::new(
        s.id.clone(),
        !s.value.as_ref().unwrap().enabled,
    )])
}
fn fanout(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok((0..3).map(|_| Target::new("source", true)).collect())
}
fn base(clock: Arc<TestClock>) -> Builder {
    Runtime::builder()
        .clock(clock)
        .resource(
            Source::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SOURCE_SET),
        )
        .resource(
            Mirror::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SET)
                .action(FAILSET),
        )
}
fn open(redb: bool, path: &std::path::Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn path(name: &str, redb: bool) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("rom-chain-{}-{name}-{redb}", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}
fn build(builder: Builder, db: Arc<dyn Storage>) -> Runtime {
    builder
        .build(db, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
#[tokio::test]
async fn downstream_failure_preserves_upstream_then_restarts_and_retries_once() {
    for redb in [false, true] {
        let p = path("restart", redb);
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let db = open(redb, &p);
        let rt = build(
            base(clock.clone()).reaction(
                Reaction::new("copy", 1, service(), FAILSET, copy)
                    .depends_on(Source::enabled_field()),
            ),
            db.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
        )
        .await
        .unwrap();
        rt.execute(
            &actor(),
            Command::create(
                "source",
                Source {
                    enabled: true,
                    title: "first".into(),
                },
            )
            .idempotency("upstream"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_reactions(1).await.unwrap(), 1);
        FAIL.store(1, Ordering::SeqCst);
        rt.process_reactions(1).await.unwrap();
        assert!(
            rt.read::<Source>(&actor(), "source")
                .await
                .unwrap()
                .value
                .unwrap()
                .enabled
        );
        assert!(
            !rt.read::<Mirror>(&actor(), "mirror")
                .await
                .unwrap()
                .value
                .unwrap()
                .enabled
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        clock.0.store(2, Ordering::SeqCst);
        let db = open(redb, &p);
        let rt = build(
            base(clock).reaction(
                Reaction::new("copy", 1, service(), FAILSET, copy)
                    .depends_on(Source::enabled_field()),
            ),
            db.clone(),
        );
        assert_eq!(rt.process_reactions(8).await.unwrap(), 1);
        let mirror = rt.read::<Mirror>(&actor(), "mirror").await.unwrap();
        assert!(mirror.value.unwrap().enabled);
        assert_eq!(mirror.revision, 2);
        assert_eq!(rt.process_reactions(8).await.unwrap(), 0);
        assert!(
            db.reaction_records()
                .unwrap()
                .iter()
                .all(|w| w.state == WorkState::Done)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn noops_dependencies_and_oscillations_are_bounded() {
    for redb in [false, true] {
        for (name, map) in [
            (
                "same",
                same as fn(&Snapshot<Source>) -> Result<Vec<Target<bool>>>,
            ),
            ("invert", invert),
            ("fanout", fanout),
        ] {
            let p = path(name, redb);
            let clock = Arc::new(TestClock(AtomicU64::new(0)));
            let db = open(redb, &p);
            let rt = build(
                base(clock)
                    .reaction_limits(ReactionLimits {
                        max_depth: 3,
                        max_fanout: 2,
                        ..Default::default()
                    })
                    .reaction(
                        Reaction::new("echo", 1, service(), SOURCE_SET, map)
                            .depends_on(Source::enabled_field()),
                    ),
                db.clone(),
            );
            rt.execute(
                &actor(),
                Command::create(
                    "source",
                    Source {
                        enabled: false,
                        title: "first".into(),
                    },
                )
                .idempotency("seed"),
            )
            .await
            .unwrap();
            rt.process_reactions(32).await.unwrap();
            let row = rt.read::<Source>(&actor(), "source").await.unwrap();
            let records = db.reaction_records().unwrap();
            match name {
                "same" => {
                    assert_eq!(row.revision, 1);
                    assert_eq!(records.len(), 2);
                    assert!(records.iter().all(|w| w.state == WorkState::Done));
                }
                "invert" => {
                    assert_eq!(row.revision, 4);
                    assert!(
                        records
                            .iter()
                            .any(|w| w.state == WorkState::Stopped(StopReason::Depth))
                    );
                }
                "fanout" => {
                    assert_eq!(row.revision, 1);
                    assert_eq!(records[0].state, WorkState::Stopped(StopReason::Fanout));
                }
                _ => unreachable!(),
            }
            let before = records.len();
            let mut value = row.value.unwrap();
            value.title = "changed unrelated field".into();
            rt.execute(
                &actor(),
                Command::replace("source", value)
                    .at_revision(row.revision)
                    .idempotency("unrelated"),
            )
            .await
            .unwrap();
            assert_eq!(db.reaction_records().unwrap().len(), before);
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(db);
            let _ = std::fs::remove_file(p);
        }
    }
}
#[tokio::test]
async fn revoked_service_stops_without_touching_target() {
    for redb in [false, true] {
        let p = path("revoked", redb);
        let db = open(redb, &p);
        let rt = build(
            base(Arc::new(TestClock(AtomicU64::new(0)))).reaction(Reaction::new(
                "copy",
                1,
                service(),
                SET,
                copy,
            )),
            db.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
        )
        .await
        .unwrap();
        rt.execute(
            &actor(),
            Command::create(
                "source",
                Source {
                    enabled: true,
                    title: "first".into(),
                },
            )
            .idempotency("source"),
        )
        .await
        .unwrap();
        rt.revoke(&service());
        rt.process_reactions(8).await.unwrap();
        assert_eq!(
            db.reaction_records().unwrap()[0].state,
            WorkState::Stopped(StopReason::Denied)
        );
        assert_eq!(
            rt.read::<Mirror>(&actor(), "mirror")
                .await
                .unwrap()
                .revision,
            1
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn lost_target_ack_is_done_atomically_and_restart_does_not_repeat() {
    for redb in [false, true] {
        use std::sync::atomic::AtomicBool;
        let p = path("lost-ack", redb);
        let fail = Arc::new(AtomicBool::new(false));
        let f = fail.clone();
        let hook: Arc<dyn Fn(usize) -> Result<()> + Send + Sync> = Arc::new(move |point| {
            if point == usize::MAX && f.swap(false, Ordering::SeqCst) {
                Err(Error::Unknown)
            } else {
                Ok(())
            }
        });
        let db: Arc<dyn Storage> = if redb {
            let db = Arc::new(rom_redb::Redb::open(&p).unwrap());
            db.on_commit(Some(hook));
            db
        } else {
            let db = Arc::new(rom_sqlite::Sqlite::open(&p).unwrap());
            db.on_commit(Some(hook));
            db
        };
        let clock = Arc::new(TestClock(AtomicU64::new(0)));
        let rt = build(
            base(clock.clone()).reaction(Reaction::new("copy", 1, service(), SET, copy)),
            db.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
        )
        .await
        .unwrap();
        rt.execute(
            &actor(),
            Command::create(
                "source",
                Source {
                    enabled: true,
                    title: "first".into(),
                },
            )
            .idempotency("source"),
        )
        .await
        .unwrap();
        rt.process_reactions(1).await.unwrap();
        fail.store(true, Ordering::SeqCst);
        rt.process_reactions(1).await.unwrap();
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let db = open(redb, &p);
        clock.0.store(31, Ordering::SeqCst);
        let rt = build(
            base(clock).reaction(Reaction::new("copy", 1, service(), SET, copy)),
            db.clone(),
        );
        assert_eq!(rt.process_reactions(16).await.unwrap(), 0);
        assert_eq!(
            rt.read::<Mirror>(&actor(), "mirror")
                .await
                .unwrap()
                .revision,
            2
        );
        assert!(
            db.reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let _ = std::fs::remove_file(p);
    }
}
#[tokio::test]
async fn runtime_worker_is_single_and_shutdown_joins_lifetime() {
    let p = path("worker", false);
    let db = open(false, &p);
    let rt = build(
        base(Arc::new(TestClock(AtomicU64::new(0)))).reaction(Reaction::new(
            "copy",
            1,
            service(),
            SET,
            copy,
        )),
        db.clone(),
    );
    let worker = rt.start_reactions().unwrap();
    assert!(matches!(rt.start_reactions(), Err(Error::Overloaded)));
    rt.shutdown().await.unwrap();
    worker.join().await.unwrap();
    assert_eq!(rt.available_io_capacity(), 8);
    drop(rt);
    drop(db);
    let _ = std::fs::remove_file(p);
}
struct Counted {
    inner: Arc<dyn Storage>,
    calls: [AtomicU64; 4],
}
impl Storage for Counted {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn supports_reactions(&self) -> bool {
        true
    }
    fn load(&self, k: &Key) -> Result<Option<Row>> {
        self.calls[0].fetch_add(1, Ordering::SeqCst);
        self.inner.load(k)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.calls[1].fetch_add(1, Ordering::SeqCst);
        self.inner.receipt(id)
    }
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        self.calls[2].fetch_add(1, Ordering::SeqCst);
        self.inner.commit(b)
    }
    fn reaction_update(&self, u: WorkUpdate) -> Result<WorkResult> {
        self.calls[3].fetch_add(1, Ordering::SeqCst);
        self.inner.reaction_update(u)
    }
    fn snapshot(&self, k: &str, r: usize, b: usize) -> Result<Vec<Row>> {
        self.inner.snapshot(k, r, b)
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.inner.reaction_records()
    }
}
#[tokio::test]
async fn actual_adapter_two_resource_chain_call_counts() {
    let mut samples = vec![];
    for redb in [false, true] {
        let p = path("counts", redb);
        let db = Arc::new(Counted {
            inner: open(redb, &p),
            calls: std::array::from_fn(|_| AtomicU64::new(0)),
        });
        let rt = build(
            base(Arc::new(TestClock(AtomicU64::new(0)))).reaction(Reaction::new(
                "copy",
                1,
                service(),
                SET,
                copy,
            )),
            db.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
        )
        .await
        .unwrap();
        for c in &db.calls {
            c.store(0, Ordering::SeqCst);
        }
        rt.execute(
            &actor(),
            Command::create(
                "source",
                Source {
                    enabled: true,
                    title: "source".into(),
                },
            )
            .idempotency("source"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_reactions(8).await.unwrap(), 2);
        let sample = db.calls.each_ref().map(|c| c.load(Ordering::SeqCst));
        println!(
            "{} chain [load,receipt,commit,reaction_update]={sample:?}",
            if redb { "redb" } else { "SQLite" }
        );
        assert_eq!(sample[2], 2);
        samples.push(sample);
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let _ = std::fs::remove_file(p);
    }
    assert_eq!(samples[0], samples[1]);
}
#[derive(Clone, Resource)]
#[resource(name = "counters")]
struct Counter {
    value: u64,
}
const INCREMENT: Action<Counter, u64> = Action::new("set", |c, v| {
    c.value = v;
    Ok(vec![])
});
fn converge(s: &Snapshot<Counter>) -> Result<Vec<Target<u64>>> {
    let value = s.value.as_ref().unwrap().value;
    Ok(if value < 3 {
        vec![Target::new(s.id.clone(), value + 1)]
    } else {
        vec![]
    })
}
#[tokio::test]
async fn revisiting_same_resource_can_converge() {
    for redb in [false, true] {
        let p = path("converge", redb);
        let db = open(redb, &p);
        let rt = build(
            Runtime::builder()
                .resource(
                    Counter::definition()
                        .allow_all_fields()
                        .policy(|_, _, _| true)
                        .action(INCREMENT),
                )
                .reaction(Reaction::new("converge", 1, service(), INCREMENT, converge)),
            db.clone(),
        );
        rt.execute(
            &actor(),
            Command::create("counter", Counter { value: 0 }).idempotency("seed"),
        )
        .await
        .unwrap();
        assert_eq!(rt.process_reactions(16).await.unwrap(), 7);
        let value = rt.read::<Counter>(&actor(), "counter").await.unwrap();
        assert_eq!(value.value.unwrap().value, 3);
        assert_eq!(value.revision, 4);
        assert!(
            db.reaction_records()
                .unwrap()
                .iter()
                .all(|w| w.state == WorkState::Done)
        );
        rt.shutdown().await.unwrap();
        drop(rt);
        drop(db);
        let _ = std::fs::remove_file(p);
    }
}
static MAP_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MAP_GATE: (std::sync::Mutex<bool>, std::sync::Condvar) =
    (std::sync::Mutex::new(false), std::sync::Condvar::new());
fn blocked_map(s: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    MAP_STARTED.store(true, Ordering::SeqCst);
    let mut release = MAP_GATE.0.lock().unwrap();
    while !*release {
        release = MAP_GATE.1.wait(release).unwrap();
    }
    copy(s)
}
#[tokio::test]
async fn cancelled_batch_is_drained_and_materialized_work_survives_shutdown() {
    let p = path("cancel", false);
    let db = open(false, &p);
    let rt = build(
        base(Arc::new(TestClock(AtomicU64::new(0)))).reaction(Reaction::new(
            "copy",
            1,
            service(),
            SET,
            blocked_map,
        )),
        db.clone(),
    );
    rt.execute(
        &actor(),
        Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
    )
    .await
    .unwrap();
    rt.execute(
        &actor(),
        Command::create(
            "source",
            Source {
                enabled: true,
                title: "source".into(),
            },
        )
        .idempotency("source"),
    )
    .await
    .unwrap();
    let background = rt.clone();
    let waiter = tokio::spawn(async move { background.process_reactions(1).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !MAP_STARTED.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    waiter.abort();
    let background = rt.clone();
    let shutdown = tokio::spawn(async move { background.shutdown().await });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    *MAP_GATE.0.lock().unwrap() = true;
    MAP_GATE.1.notify_all();
    shutdown.await.unwrap().unwrap();
    assert_eq!(db.reaction_records().unwrap().len(), 2);
    assert!(
        db.reaction_records()
            .unwrap()
            .iter()
            .any(|w| w.state == WorkState::Pending)
    );
    drop(rt);
    drop(db);
    let db = open(false, &p);
    let rt = build(
        base(Arc::new(TestClock(AtomicU64::new(0)))).reaction(Reaction::new(
            "copy",
            1,
            service(),
            SET,
            copy,
        )),
        db.clone(),
    );
    assert_eq!(rt.process_reactions(8).await.unwrap(), 1);
    assert!(
        rt.read::<Mirror>(&actor(), "mirror")
            .await
            .unwrap()
            .value
            .unwrap()
            .enabled
    );
    rt.shutdown().await.unwrap();
    drop(rt);
    drop(db);
    let _ = std::fs::remove_file(p);
}
static PROTECTED_MAP_CALLS: AtomicUsize = AtomicUsize::new(0);
fn protected_copy(s: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    PROTECTED_MAP_CALLS.fetch_add(1, Ordering::SeqCst);
    copy(s)
}
#[tokio::test]
async fn mapper_requires_all_historical_and_current_source_fields() {
    for redb in [false, true] {
        for historical_hidden in [false, true] {
            PROTECTED_MAP_CALLS.store(0, Ordering::SeqCst);
            let p = path(&format!("protected-{historical_hidden}"), redb);
            let db = open(redb, &p);
            let source = Source::definition().policy(|_, _, _| true).field_policy(
                |actor, access, name, value| {
                    actor.principal_kind() != PrincipalKind::Service
                        || !matches!(access, Access::Read)
                        || name != "title"
                        || value.title != "hidden"
                },
            );
            let rt = build(
                Runtime::builder()
                    .resource(source)
                    .resource(
                        Mirror::definition()
                            .allow_all_fields()
                            .policy(|_, _, _| true)
                            .action(SET),
                    )
                    .reaction(
                        Reaction::new("copy", 1, service(), SET, protected_copy)
                            .depends_on(Source::enabled_field()),
                    ),
                db.clone(),
            );
            rt.execute(
                &actor(),
                Command::create("mirror", Mirror { enabled: false }).idempotency("seed"),
            )
            .await
            .unwrap();
            rt.execute(
                &actor(),
                Command::create(
                    "source",
                    Source {
                        enabled: true,
                        title: if historical_hidden {
                            "hidden"
                        } else {
                            "visible"
                        }
                        .into(),
                    },
                )
                .idempotency("source"),
            )
            .await
            .unwrap();
            rt.execute(
                &actor(),
                Command::replace(
                    "source",
                    Source {
                        enabled: true,
                        title: if historical_hidden {
                            "visible"
                        } else {
                            "hidden"
                        }
                        .into(),
                    },
                )
                .at_revision(1)
                .idempotency("visibility"),
            )
            .await
            .unwrap();
            rt.process_reactions(8).await.unwrap();
            assert_eq!(
                PROTECTED_MAP_CALLS.load(Ordering::SeqCst),
                0,
                "callback observed protected source fields"
            );
            assert_eq!(
                db.reaction_records().unwrap()[0].state,
                WorkState::Stopped(StopReason::Denied)
            );
            assert_eq!(
                rt.read::<Mirror>(&actor(), "mirror")
                    .await
                    .unwrap()
                    .revision,
                1
            );
            rt.shutdown().await.unwrap();
            drop(rt);
            drop(db);
            let _ = std::fs::remove_file(p);
        }
    }
}
