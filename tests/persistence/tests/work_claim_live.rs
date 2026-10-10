//! Bounded persisted claim samples and explicit compatibility for older wrappers.
#[path = "support/work_atomic_batch_fixture.rs"]
mod fixture;
use rom::*;
type CallbackLatch = Arc<(std::sync::Mutex<(bool, bool)>, std::sync::Condvar)>;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
enum Db {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Db {
    fn open(redb: bool, label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-claim-live-{label}-{redb}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(s) => s.clone(),
            Self::Redb(s) => s.clone(),
        }
    }
    fn canonical(&self, label: &str) -> serde_json::Value {
        let path = std::env::temp_dir().join(format!(
            "rom-claim-live-backup-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let backend = match self {
            Self::Sqlite(db) => {
                db.backup_to(&path, rom_backup::BackupLimits::default())
                    .unwrap();
                rom_backup::Backend::Sqlite
            }
            Self::Redb(db) => {
                db.backup_to(&path, rom_backup::BackupLimits::default())
                    .unwrap();
                rom_backup::Backend::Redb
            }
        };
        let (_, snapshot) =
            rom_backup::read(&path, backend, rom_backup::BackupLimits::default()).unwrap();
        snapshot.validate().unwrap();
        serde_json::to_value(snapshot).unwrap()
    }
    fn commits(&self, count: Arc<AtomicUsize>) {
        let observer: Arc<dyn Fn(usize) -> Result<()> + Send + Sync> = Arc::new(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        match self {
            Self::Sqlite(db) => db.on_commit(Some(observer)),
            Self::Redb(db) => db.on_commit(Some(observer)),
        }
    }
}
#[test]
fn native_keyed_claim_read_is_read_only_and_checks_persisted_generation_deadline_and_identity() {
    for redb in [false, true] {
        let db = Db::open(redb, "query");
        let storage = db.storage();
        fixture::seed(storage.as_ref());
        let claims = storage.reaction_claim_prefix(0, 2).unwrap();
        let before = db.canonical("before");
        let commits = Arc::new(AtomicUsize::new(0));
        db.commits(commits.clone());
        assert_eq!(
            storage.reaction_claim_live(&claims[0].key(), 0).unwrap(),
            Some(true)
        );
        assert_eq!(
            storage.reaction_claim_live(&claims[0].key(), 29).unwrap(),
            Some(true)
        );
        assert_eq!(
            storage.reaction_claim_live(&claims[0].key(), 30).unwrap(),
            Some(false)
        );
        assert_eq!(
            storage
                .reaction_claim_live(
                    &ClaimKey {
                        id: "missing".into(),
                        generation: 1
                    },
                    0
                )
                .unwrap(),
            Some(false)
        );
        assert_eq!(
            storage
                .reaction_claim_live(
                    &ClaimKey {
                        id: claims[0].key().id,
                        generation: 0
                    },
                    0
                )
                .unwrap(),
            Some(false)
        );
        assert_eq!(commits.load(Ordering::SeqCst), 0);
        assert_eq!(db.canonical("after"), before);
        let new = storage.reaction_claim_prefix(31, 2).unwrap();
        assert_eq!(new[0].work.generation, 2);
        assert_eq!(
            storage.reaction_claim_live(&claims[0].key(), 0).unwrap(),
            Some(false)
        );
        assert_eq!(
            storage.reaction_claim_live(&new[0].key(), 0).unwrap(),
            Some(true)
        );
        let current = db.canonical("new-generation");
        assert_eq!(
            storage.reaction_update(WorkUpdate::Materialize {
                claim: claims[0].key(),
                now: 0,
                children: vec![]
            }),
            Err(Error::Conflict)
        );
        assert_eq!(db.canonical("rejected-stale-update"), current);
        assert_eq!(
            storage.reaction_claim_live(&claims[1].key(), 0).unwrap(),
            Some(false)
        );
        assert_eq!(db.canonical("new-generation-after"), current);
    }
}

struct HalfForward(Arc<dyn Storage>);
impl Storage for HalfForward {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.0.acquire_owner()
    }
    fn register(&self, d: &[Descriptor]) -> Result<()> {
        self.0.register(d)
    }
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    fn supports_reactions(&self) -> bool {
        self.0.supports_reactions()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.0.load(key)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.0.receipt(id)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.0.commit(bundle)
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        self.0.reaction_update(update)
    }
    fn reaction_claim_prefix(&self, now: u64, count: usize) -> Result<Vec<WorkClaim>> {
        self.0.reaction_claim_prefix(now, count)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.0.snapshot(kind, rows, bytes)
    }
}
#[derive(Clone, Resource)]
#[resource(name = "live-read-source")]
struct Source {
    enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "live-read-target")]
struct Destination {
    enabled: bool,
}
const SET: Action<Destination, bool> = Action::new("set", |row, value| {
    row.enabled = value;
    Ok(vec![])
});
static HALF_CALLS: AtomicUsize = AtomicUsize::new(0);
fn empty(_: &Snapshot<Source>) -> Result<Vec<Target<bool>>> {
    HALF_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(vec![])
}
struct FakeClock(AtomicU64);
impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn runtime(storage: Arc<dyn Storage>, clock: Arc<FakeClock>) -> Runtime {
    Runtime::builder()
        .clock(clock)
        .resource(
            Source::definition()
                .allow_all_fields()
                .policy(|_, _, _| true),
        )
        .resource(
            Destination::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(SET),
        )
        .reaction(Reaction::new(
            "live-read-map",
            1,
            Actor::trusted("host", "live-service").with_kind(PrincipalKind::Service),
            SET,
            empty,
        ))
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
#[tokio::test]
async fn half_forwarded_group_is_refused_without_callbacks_and_native_recovery_keeps_attempts() {
    for redb in [false, true] {
        HALF_CALLS.store(0, Ordering::SeqCst);
        let db = Db::open(redb, "half");
        let clock = Arc::new(FakeClock(AtomicU64::new(0)));
        let original = runtime(Arc::new(HalfForward(db.storage())), clock.clone());
        for id in ["a", "b"] {
            original
                .execute(
                    &Actor::trusted("host", "owner"),
                    Command::create(id, Source { enabled: true }).idempotency(id),
                )
                .await
                .unwrap();
        }
        assert!(matches!(
            original.process_reactions(2).await,
            Err(Error::Unsupported(_))
        ));
        assert_eq!(HALF_CALLS.load(Ordering::SeqCst), 0);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|record| record.attempts == 1
                    && record.generation == 1
                    && matches!(record.state, WorkState::Leased { .. }))
        );
        original.shutdown().await.unwrap();
        drop(original);
        clock.0.store(31, Ordering::SeqCst);
        let recovered = runtime(db.storage(), clock);
        assert_eq!(recovered.process_reactions(2).await.unwrap(), 2);
        assert_eq!(HALF_CALLS.load(Ordering::SeqCst), 2);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|record| record.attempts == 2
                    && record.generation == 2
                    && record.state == WorkState::Done)
        );
        recovered.shutdown().await.unwrap();
    }
}

#[derive(Clone, Resource)]
#[resource(name = "live-policy-source")]
struct PolicySource {
    enabled: bool,
}
const POLICY_SET: Action<PolicySource, bool> = Action::new("set", |row, value| {
    row.enabled = value;
    Ok(vec![])
});
static POLICY_CALLS: AtomicUsize = AtomicUsize::new(0);
static POLICY_WAIT: std::sync::Mutex<Option<CallbackLatch>> = std::sync::Mutex::new(None);
fn policy_mapper(_: &Snapshot<PolicySource>) -> Result<Vec<Target<bool>>> {
    if POLICY_CALLS.fetch_add(1, Ordering::SeqCst) == 1 {
        let pair = POLICY_WAIT.lock().unwrap().as_ref().unwrap().clone();
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
async fn changed_current_source_policy_denies_deferred_historical_output_without_blocking_valid_root()
 {
    for redb in [false, true] {
        POLICY_CALLS.store(0, Ordering::SeqCst);
        let db = Db::open(redb, "policy");
        let runtime = Runtime::builder()
            .clock(Arc::new(FakeClock(AtomicU64::new(0))))
            .resource(
                PolicySource::definition()
                    .allow_all_fields()
                    .policy(|actor, _, row| {
                        actor.principal_kind() != PrincipalKind::Service || row.enabled
                    })
                    .action(POLICY_SET),
            )
            .resource(
                Destination::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true)
                    .action(SET),
            )
            .reaction(Reaction::new(
                "live-policy-map",
                1,
                Actor::trusted("host", "policy-service").with_kind(PrincipalKind::Service),
                SET,
                policy_mapper,
            ))
            .build(db.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let owner = Actor::trusted("host", "policy-owner");
        for id in ["a", "b"] {
            runtime
                .execute(
                    &owner,
                    Command::create(id, PolicySource { enabled: true }).idempotency(id),
                )
                .await
                .unwrap();
        }
        let pair = Arc::new((
            std::sync::Mutex::new((false, false)),
            std::sync::Condvar::new(),
        ));
        *POLICY_WAIT.lock().unwrap() = Some(pair.clone());
        let worker = runtime.clone();
        let processing = tokio::spawn(async move { worker.process_reactions(2).await });
        loop {
            if pair.0.lock().unwrap().0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        runtime
            .execute(
                &owner,
                Command::action("a", POLICY_SET, false)
                    .at_revision(1)
                    .idempotency("deny-current"),
            )
            .await
            .unwrap();
        pair.0.lock().unwrap().1 = true;
        pair.1.notify_all();
        assert_eq!(processing.await.unwrap().unwrap(), 2);
        assert_eq!(POLICY_CALLS.load(Ordering::SeqCst), 2);
        let records = db.storage().reaction_records().unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(
            records
                .iter()
                .filter(|r| r.state == WorkState::Stopped(StopReason::Denied))
                .count(),
            1
        );
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
                .filter(|r| r.state == WorkState::Pending && r.attempts == 0)
                .count(),
            1
        );
        *POLICY_WAIT.lock().unwrap() = None;
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn native_claim_sample_rejects_rewound_time_before_stored_eligibility_floor() {
    for redb in [false, true] {
        let db = Db::open(redb, "eligibility");
        let current = runtime(db.storage(), Arc::new(FakeClock(AtomicU64::new(10))));
        current
            .execute(
                &Actor::trusted("host", "owner"),
                Command::create("a", Source { enabled: true }).idempotency("a"),
            )
            .await
            .unwrap();
        let claim = db.storage().reaction_claim_prefix(10, 1).unwrap().remove(0);
        let before = db.canonical("eligibility-before");
        for now in [0, 9, 40] {
            assert_eq!(
                db.storage().reaction_claim_live(&claim.key(), now).unwrap(),
                Some(false)
            );
        }
        for now in [10, 39] {
            assert_eq!(
                db.storage().reaction_claim_live(&claim.key(), now).unwrap(),
                Some(true)
            );
        }
        assert_eq!(db.canonical("eligibility-after"), before);
        current.shutdown().await.unwrap();
    }
}
