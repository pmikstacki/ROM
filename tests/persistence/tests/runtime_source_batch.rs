//! Draft runtime RED fixtures; install as tests/persistence/tests/runtime_source_batch.rs.
//! Uses public APIs; current singleton worker should fail the new grouping assertions.
use rom::*;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
type Observer = Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
type CallbackLatch = Arc<(Mutex<(bool, bool)>, Condvar)>;
#[derive(Clone, Resource)]
#[resource(name = "runtime-batch-sources")]
struct Source {
    enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "runtime-batch-targets")]
struct TargetResource {
    enabled: bool,
}
const SET: Action<TargetResource, bool> = Action::new("set", |resource, value| {
    resource.enabled = value;
    Ok(vec![])
});
const SOURCE_SET: Action<Source, bool> = Action::new("set", |resource, value| {
    resource.enabled = value;
    Ok(vec![])
});
struct FakeClock(AtomicU64);
impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
enum Db {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Db {
    fn open(redb: bool, label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "rom-runtime-batch-{label}-{redb}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(dir.join("db")).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(dir.join("db")).unwrap()))
        }
    }
    fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(s) => s.clone(),
            Self::Redb(s) => s.clone(),
        }
    }
    fn observe(&self, observer: Option<Observer>) {
        match self {
            Self::Sqlite(s) => s.on_commit(observer),
            Self::Redb(s) => s.on_commit(observer),
        }
    }
}
fn owner() -> Actor {
    Actor::trusted("host", "batch-owner")
}
fn service() -> Actor {
    Actor::trusted("host", "batch-service").with_kind(PrincipalKind::Service)
}
async fn setup(
    redb: bool,
    label: &str,
    clock: Arc<FakeClock>,
    map: fn(&Snapshot<Source>) -> Result<Vec<Target<bool>>>,
) -> (Db, Runtime) {
    setup_diagnostics(redb, label, clock, map, None).await
}
async fn setup_diagnostics(
    redb: bool,
    label: &str,
    clock: Arc<FakeClock>,
    map: fn(&Snapshot<Source>) -> Result<Vec<Target<bool>>>,
    sink: Option<DiagnosticSink>,
) -> (Db, Runtime) {
    let db = Db::open(redb, label);
    let mut builder = Runtime::builder();
    if let Some(sink) = sink {
        builder = builder.diagnostics(sink);
    }
    let runtime = builder
        .clock(clock)
        .resource(
            Source::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SOURCE_SET),
        )
        .resource(
            TargetResource::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SET),
        )
        .reaction(Reaction::new("batch-map", 1, service(), SET, map))
        .build(db.storage(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create("target", TargetResource { enabled: false }).idempotency("target"),
        )
        .await
        .unwrap();
    for id in ["a", "b"] {
        runtime
            .execute(
                &owner(),
                Command::create(id, Source { enabled: true }).idempotency(&format!("source-{id}")),
            )
            .await
            .unwrap();
    }
    (db, runtime)
}
static PREFETCH_STORAGE: Mutex<Option<Arc<dyn Storage>>> = Mutex::new(None);
static PREFETCH_SEEN: AtomicUsize = AtomicUsize::new(0);
static PREFETCH_CALLS: AtomicUsize = AtomicUsize::new(0);
fn observed_empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    let leased = PREFETCH_STORAGE
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .reaction_records()?
        .iter()
        .filter(|record| matches!(record.state, WorkState::Leased { .. }))
        .count();
    if PREFETCH_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
        PREFETCH_SEEN.store(leased, Ordering::SeqCst);
    }
    Ok(vec![])
}
#[tokio::test]
async fn two_sources_are_durably_claimed_before_sequential_mappers_and_finish_in_two_commits() {
    for redb in [false, true] {
        PREFETCH_CALLS.store(0, Ordering::SeqCst);
        PREFETCH_SEEN.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "empty",
            Arc::new(FakeClock(AtomicU64::new(0))),
            observed_empty,
        )
        .await;
        *PREFETCH_STORAGE.lock().unwrap() = Some(db.storage());
        let commits = Arc::new(AtomicUsize::new(0));
        let observed = commits.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == usize::MAX {
                observed.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(PREFETCH_CALLS.load(Ordering::SeqCst), 2);
        assert_eq!(PREFETCH_SEEN.load(Ordering::SeqCst), 2);
        assert_eq!(commits.load(Ordering::SeqCst), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
        *PREFETCH_STORAGE.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}
static EXPIRY_CLOCK: Mutex<Option<Arc<FakeClock>>> = Mutex::new(None);
static EXPIRY_CALLS: AtomicUsize = AtomicUsize::new(0);
static EXPIRED_ONCE: AtomicBool = AtomicBool::new(false);
fn expires_reserved_leases(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    EXPIRY_CALLS.fetch_add(1, Ordering::SeqCst);
    if !EXPIRED_ONCE.swap(true, Ordering::SeqCst) {
        EXPIRY_CLOCK
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .0
            .store(31, Ordering::SeqCst);
    }
    Ok(vec![])
}
#[tokio::test]
async fn expired_prefetched_claim_gets_no_callback_and_recovers_without_attempt_reset() {
    for redb in [false, true] {
        EXPIRY_CALLS.store(0, Ordering::SeqCst);
        EXPIRED_ONCE.store(false, Ordering::SeqCst);
        let clock = Arc::new(FakeClock(AtomicU64::new(0)));
        *EXPIRY_CLOCK.lock().unwrap() = Some(clock.clone());
        let (db, runtime) = setup(redb, "expiry", clock, expires_reserved_leases).await;
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 1);
        assert_eq!(EXPIRY_CALLS.load(Ordering::SeqCst), 1);
        let records = db.storage().reaction_records().unwrap();
        assert!(records.iter().all(|r| r.attempts == 1
            && r.generation == 1
            && matches!(r.state, WorkState::Leased { .. })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(EXPIRY_CALLS.load(Ordering::SeqCst), 3);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done && r.attempts == 2 && r.generation == 2)
        );
        *EXPIRY_CLOCK.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}
static UNKNOWN_CALLS: AtomicUsize = AtomicUsize::new(0);
fn unknown_probe(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    UNKNOWN_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![])
}
#[tokio::test]
async fn unknown_claim_group_dispatches_no_mapper() {
    for redb in [false, true] {
        UNKNOWN_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "unknown-claim",
            Arc::new(FakeClock(AtomicU64::new(0))),
            unknown_probe,
        )
        .await;
        db.observe(Some(Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(runtime.process_reactions(2).await, Err(Error::Unknown));
        assert_eq!(UNKNOWN_CALLS.load(Ordering::SeqCst), 0);
        db.observe(None);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.attempts == 1 && matches!(r.state, WorkState::Leased { .. }))
        );
        runtime.shutdown().await.unwrap();
    }
}
static REVOKE_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
static REVOKE_CALLS: AtomicUsize = AtomicUsize::new(0);
fn revokes_before_result_flush(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    REVOKE_CALLS.fetch_add(1, Ordering::SeqCst);
    REVOKE_RUNTIME
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .revoke(&service());
    Ok(vec![])
}
#[tokio::test]
async fn mapper_revocation_prevents_deferred_done_and_later_reserved_dispatch() {
    for redb in [false, true] {
        REVOKE_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "revocation",
            Arc::new(FakeClock(AtomicU64::new(0))),
            revokes_before_result_flush,
        )
        .await;
        *REVOKE_RUNTIME.lock().unwrap() = Some(runtime.clone());
        runtime.process_reactions(2).await.unwrap();
        assert_eq!(REVOKE_CALLS.load(Ordering::SeqCst), 1);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Stopped(StopReason::Denied))
        );
        *REVOKE_RUNTIME.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}
static DRAIN_STARTED: AtomicBool = AtomicBool::new(false);
static DRAIN_CALLS: AtomicUsize = AtomicUsize::new(0);
static DRAIN_GATE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());
fn blocks_first_reserved_mapper(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    DRAIN_CALLS.fetch_add(1, Ordering::SeqCst);
    DRAIN_STARTED.store(true, Ordering::SeqCst);
    let mut open = DRAIN_GATE.0.lock().unwrap();
    while !*open {
        open = DRAIN_GATE.1.wait(open).unwrap();
    }
    Ok(vec![])
}
#[tokio::test]
async fn cancelled_observer_and_shutdown_drain_all_already_reserved_claims() {
    for redb in [false, true] {
        DRAIN_STARTED.store(false, Ordering::SeqCst);
        DRAIN_CALLS.store(0, Ordering::SeqCst);
        *DRAIN_GATE.0.lock().unwrap() = false;
        let (db, runtime) = setup(
            redb,
            "drain",
            Arc::new(FakeClock(AtomicU64::new(0))),
            blocks_first_reserved_mapper,
        )
        .await;
        let background = runtime.clone();
        let observer = tokio::spawn(async move { background.process_reactions(2).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !DRAIN_STARTED.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        observer.abort();
        let background = runtime.clone();
        let shutdown = tokio::spawn(async move { background.shutdown().await });
        tokio::task::yield_now().await;
        assert!(!shutdown.is_finished());
        *DRAIN_GATE.0.lock().unwrap() = true;
        DRAIN_GATE.1.notify_all();
        shutdown.await.unwrap().unwrap();
        assert_eq!(DRAIN_CALLS.load(Ordering::SeqCst), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
    }
}

static CHILD_STORAGE: Mutex<Option<Arc<dyn Storage>>> = Mutex::new(None);
static CHILD_CALLS: AtomicUsize = AtomicUsize::new(0);
static CHILD_BARRIER_SEEN: AtomicBool = AtomicBool::new(false);
fn creates_child_then_observes_reserved_fairness(
    _: &Snapshot<Source>,
) -> Result<Vec<Target<bool>>> {
    if CHILD_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
        return Ok(vec![Target::new("target", true)]);
    }
    let records = CHILD_STORAGE
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .reaction_records()?;
    CHILD_BARRIER_SEEN.store(
        records.iter().any(|r| {
            matches!(r.pending.payload, WorkPayload::Action(_)) && r.state == WorkState::Pending
        }) && records
            .iter()
            .filter(|r| {
                matches!(r.pending.payload, WorkPayload::Source(_)) && r.state == WorkState::Done
            })
            .count()
            == 1,
        Ordering::SeqCst,
    );
    Ok(vec![])
}
#[tokio::test]
async fn child_materialization_is_acknowledged_before_reserved_source_resumes_without_timeout() {
    for redb in [false, true] {
        CHILD_CALLS.store(0, Ordering::SeqCst);
        CHILD_BARRIER_SEEN.store(false, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "children",
            Arc::new(FakeClock(AtomicU64::new(0))),
            creates_child_then_observes_reserved_fairness,
        )
        .await;
        *CHILD_STORAGE.lock().unwrap() = Some(db.storage());
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(CHILD_CALLS.load(Ordering::SeqCst), 2);
        assert!(CHILD_BARRIER_SEEN.load(Ordering::SeqCst));
        let records = db.storage().reaction_records().unwrap();
        assert!(
            records
                .iter()
                .filter(|r| matches!(r.pending.payload, WorkPayload::Source(_)))
                .all(|r| r.state == WorkState::Done && r.attempts == 1)
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| matches!(r.pending.payload, WorkPayload::Action(_))
                    && r.state == WorkState::Pending)
                .count(),
            1
        );
        assert_eq!(runtime.process_reactions(1).await.unwrap(), 1);
        let target = runtime
            .read::<TargetResource>(&owner(), "target")
            .await
            .unwrap();
        assert!(target.value.unwrap().enabled);
        assert_eq!(target.revision, 2);
        *CHILD_STORAGE.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}

static HISTORICAL_REVISIONS: Mutex<Vec<u64>> = Mutex::new(vec![]);
fn records_historical_revision(snapshot: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    if snapshot.id == "a" {
        HISTORICAL_REVISIONS.lock().unwrap().push(snapshot.revision);
    }
    Ok(vec![])
}
#[tokio::test]
async fn newer_authorized_source_revision_does_not_reject_its_historical_event() {
    for redb in [false, true] {
        HISTORICAL_REVISIONS.lock().unwrap().clear();
        let (db, runtime) = setup(
            redb,
            "historical",
            Arc::new(FakeClock(AtomicU64::new(0))),
            records_historical_revision,
        )
        .await;
        runtime
            .execute(
                &owner(),
                Command::action("a", SOURCE_SET, false)
                    .at_revision(1)
                    .idempotency("source-a-change"),
            )
            .await
            .unwrap();
        assert_eq!(runtime.process_reactions(8).await.unwrap(), 3);
        let mut revisions = HISTORICAL_REVISIONS.lock().unwrap().clone();
        revisions.sort();
        assert_eq!(revisions, vec![1, 2]);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
        runtime.shutdown().await.unwrap();
    }
}

static OUTPUT_UNKNOWN_CALLS: AtomicUsize = AtomicUsize::new(0);
fn unknown_empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    OUTPUT_UNKNOWN_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![])
}
#[tokio::test]
async fn unknown_materialization_ack_never_repeats_mapper_or_blindly_republishes_results() {
    for redb in [false, true] {
        OUTPUT_UNKNOWN_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "results-unknown",
            Arc::new(FakeClock(AtomicU64::new(0))),
            unknown_empty,
        )
        .await;
        let acknowledgements = Arc::new(AtomicUsize::new(0));
        let observed = acknowledgements.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == usize::MAX && observed.fetch_add(1, Ordering::SeqCst) == 1 {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(acknowledgements.load(Ordering::SeqCst), 2);
        assert_eq!(OUTPUT_UNKNOWN_CALLS.load(Ordering::SeqCst), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done)
        );
        db.observe(None);
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 0);
        assert_eq!(OUTPUT_UNKNOWN_CALLS.load(Ordering::SeqCst), 2);
        runtime.shutdown().await.unwrap();
    }
}

static OUTPUT_ROLLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
fn rollback_empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    OUTPUT_ROLLBACK_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![])
}
#[tokio::test]
async fn confirmed_materialization_rollback_publishes_obtained_outputs_without_mapper_repeat() {
    for redb in [false, true] {
        OUTPUT_ROLLBACK_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "results-rollback",
            Arc::new(FakeClock(AtomicU64::new(0))),
            rollback_empty,
        )
        .await;
        let precommits = Arc::new(AtomicUsize::new(0));
        let observed = precommits.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == 0 && observed.fetch_add(1, Ordering::SeqCst) == 1 {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(precommits.load(Ordering::SeqCst), 4);
        assert_eq!(OUTPUT_ROLLBACK_CALLS.load(Ordering::SeqCst), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done && r.attempts == 1)
        );
        db.observe(None);
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 0);
        assert_eq!(OUTPUT_ROLLBACK_CALLS.load(Ordering::SeqCst), 2);
        runtime.shutdown().await.unwrap();
    }
}

static ERROR_BARRIER_CALLS: AtomicUsize = AtomicUsize::new(0);
fn middle_error(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    if ERROR_BARRIER_CALLS.fetch_add(1, Ordering::SeqCst) == 1 {
        Err(Error::Storage)
    } else {
        Ok(vec![])
    }
}
#[tokio::test]
async fn mapper_error_flushes_prior_obtained_result_before_retry_publication() {
    for redb in [false, true] {
        ERROR_BARRIER_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "error-barrier",
            Arc::new(FakeClock(AtomicU64::new(0))),
            middle_error,
        )
        .await;
        let records = db.storage();
        let observed = Arc::new(AtomicUsize::new(0));
        let checks = observed.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == usize::MAX {
                // Native guard is held here; do not recursively read this same DB.
                checks.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert_eq!(ERROR_BARRIER_CALLS.load(Ordering::SeqCst), 2);
        let work = records.reaction_records().unwrap();
        assert_eq!(
            work.iter().filter(|r| r.state == WorkState::Done).count(),
            1
        );
        assert_eq!(
            work.iter()
                .filter(|r| r.state == WorkState::Pending && r.due > 0)
                .count(),
            1
        );
        assert_eq!(observed.load(Ordering::SeqCst), 3);
        db.observe(None);
        runtime.shutdown().await.unwrap();
    }
}

fn diagnostic_empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    Ok(vec![])
}
#[tokio::test]
async fn deferred_success_diagnostics_are_unpublished_until_materialization_acknowledges() {
    for redb in [false, true] {
        let (sink, mut reader) = Diagnostics::bounded(
            DiagnosticOptions::new(DiagnosticKey::new([93; 32]), [94; 16]).capacity(512),
        )
        .unwrap();
        let (db, runtime) = setup_diagnostics(
            redb,
            "diagnostics",
            Arc::new(FakeClock(AtomicU64::new(0))),
            diagnostic_empty,
            Some(sink),
        )
        .await;
        while reader.try_recv().is_some() {}
        let reader = Arc::new(Mutex::new(reader));
        let before_ack = Arc::new(AtomicBool::new(false));
        let seen = before_ack.clone();
        let observed_reader = reader.clone();
        let precommits = Arc::new(AtomicUsize::new(0));
        let observed_precommits = precommits.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == 0 && observed_precommits.fetch_add(1, Ordering::SeqCst) == 1 {
                while let Some(event) = observed_reader.lock().unwrap().try_recv() {
                    assert!(
                        !(event.stage == DiagnosticStage::Work
                            && event.outcome == DiagnosticOutcome::Succeeded)
                    );
                    assert!(
                        !(event.stage == DiagnosticStage::Materialize
                            && event.outcome == DiagnosticOutcome::Succeeded)
                    );
                }
                seen.store(true, Ordering::SeqCst);
            }
            Ok(())
        })));
        assert_eq!(runtime.process_reactions(2).await.unwrap(), 2);
        assert!(before_ack.load(Ordering::SeqCst));
        let mut succeeded = 0;
        while let Some(event) = reader.lock().unwrap().try_recv() {
            if event.stage == DiagnosticStage::Work && event.outcome == DiagnosticOutcome::Succeeded
            {
                succeeded += 1;
            }
        }
        assert_eq!(succeeded, 2);
        db.observe(None);
        runtime.shutdown().await.unwrap();
    }
}

static HARD_BARRIER_CALLS: AtomicUsize = AtomicUsize::new(0);
fn hard_barrier(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    if HARD_BARRIER_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
        Ok(vec![Target::new("target", true)])
    } else {
        Ok(vec![])
    }
}
#[tokio::test]
async fn persistent_first_barrier_publication_failure_still_drains_reserved_second_source() {
    for redb in [false, true] {
        HARD_BARRIER_CALLS.store(0, Ordering::SeqCst);
        let (db, runtime) = setup(
            redb,
            "hard-barrier",
            Arc::new(FakeClock(AtomicU64::new(0))),
            hard_barrier,
        )
        .await;
        let precommits = Arc::new(AtomicUsize::new(0));
        let observed = precommits.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == 0 && observed.fetch_add(1, Ordering::SeqCst) > 0 {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(runtime.process_reactions(2).await, Err(Error::NotCommitted));
        assert_eq!(HARD_BARRIER_CALLS.load(Ordering::SeqCst), 2);
        assert_eq!(db.storage().reaction_records().unwrap().len(), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|record| matches!(record.state, WorkState::Leased { .. })
                    && record.attempts == 1)
        );
        db.observe(None);
        runtime.shutdown().await.unwrap();
    }
}

static PARTIAL_DIAGNOSTIC_CALLS: AtomicUsize = AtomicUsize::new(0);
fn partial_diagnostic_empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    PARTIAL_DIAGNOSTIC_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![])
}
#[tokio::test]
async fn singleton_fallback_partial_failure_keeps_acknowledged_success_diagnostics_exact() {
    for redb in [false, true] {
        PARTIAL_DIAGNOSTIC_CALLS.store(0, Ordering::SeqCst);
        let (sink, mut reader) = Diagnostics::bounded(
            DiagnosticOptions::new(DiagnosticKey::new([95; 32]), [96; 16]).capacity(512),
        )
        .unwrap();
        let (db, runtime) = setup_diagnostics(
            redb,
            "partial-diagnostics",
            Arc::new(FakeClock(AtomicU64::new(0))),
            partial_diagnostic_empty,
            Some(sink),
        )
        .await;
        while reader.try_recv().is_some() {}
        let precommits = Arc::new(AtomicUsize::new(0));
        let observed = precommits.clone();
        db.observe(Some(Arc::new(move |point| {
            if point == 0 {
                let ordinal = observed.fetch_add(1, Ordering::SeqCst);
                if ordinal == 1 || ordinal == 3 {
                    return Err(Error::Storage);
                }
            }
            Ok(())
        })));
        assert_eq!(runtime.process_reactions(2).await, Err(Error::NotCommitted));
        assert_eq!(PARTIAL_DIAGNOSTIC_CALLS.load(Ordering::SeqCst), 2);
        let records = db.storage().reaction_records().unwrap();
        assert_eq!(
            records
                .iter()
                .filter(|r| r.state == WorkState::Done)
                .count(),
            1
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| matches!(r.state, WorkState::Leased { .. }))
                .count(),
            1
        );
        let mut successes = 0;
        let mut failures = 0;
        while let Some(event) = reader.try_recv() {
            if event.stage == DiagnosticStage::Work {
                if event.outcome == DiagnosticOutcome::Succeeded {
                    successes += 1;
                }
                if event.outcome == DiagnosticOutcome::NotCommitted {
                    failures += 1;
                }
            }
        }
        assert_eq!((successes, failures), (1, 1));
        db.observe(None);
        runtime.shutdown().await.unwrap();
    }
}

static REWIND_CALLS: AtomicUsize = AtomicUsize::new(0);
static REWIND_STATE: Mutex<Option<CallbackLatch>> = Mutex::new(None);
fn rewind_mapper(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    if REWIND_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
        let pair = REWIND_STATE.lock().unwrap().as_ref().unwrap().clone();
        let mut state = pair.0.lock().unwrap();
        state.0 = true;
        pair.1.notify_all();
        while !state.1 {
            state = pair.1.wait(state).unwrap();
        }
    }
    Ok(vec![])
}
#[tokio::test]
async fn reclaimed_generation_then_clock_rewind_suppresses_stale_reserved_callback() {
    for redb in [false, true] {
        REWIND_CALLS.store(0, Ordering::SeqCst);
        let clock = Arc::new(FakeClock(AtomicU64::new(0)));
        let (db, runtime) = setup(redb, "clock-rewind", clock.clone(), rewind_mapper).await;
        let pair = Arc::new((Mutex::new((false, false)), Condvar::new()));
        *REWIND_STATE.lock().unwrap() = Some(pair.clone());
        let worker = runtime.clone();
        let processing = tokio::spawn(async move { worker.process_reactions(2).await });
        loop {
            if pair.0.lock().unwrap().0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        clock.0.store(31, Ordering::SeqCst);
        let replacements = db.storage().reaction_claim_prefix(31, 2).unwrap();
        assert_eq!(replacements.len(), 2);
        assert!(
            replacements
                .iter()
                .all(|claim| claim.work.generation == 2 && claim.work.attempts == 2)
        );
        let before = db.storage().reaction_records().unwrap();
        clock.0.store(0, Ordering::SeqCst);
        pair.0.lock().unwrap().1 = true;
        pair.1.notify_all();
        let result = processing.await.unwrap();
        // The first mapper entered under generation1 before the reclaim. The
        // second was already stale at dispatch and must never be called.
        assert_eq!(REWIND_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(result.unwrap(), 1);
        assert_eq!(db.storage().reaction_records().unwrap(), before);
        *REWIND_STATE.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}
