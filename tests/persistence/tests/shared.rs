use rom::{
    Actor, Bundle, Command, Error, Intent, Key, Receipt, Resource, Result, Row, Runtime, Storage,
    json,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
};

type Observer = Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
trait Inspect: Storage {
    fn counts(&self) -> [u64; 4];
    fn effects(&self) -> Vec<(String, Intent)>;
    fn events(&self) -> Vec<Row>;
    fn observer(&self, observer: Option<Observer>);
}
macro_rules! inspect {
    ($type:ty) => {
        impl Inspect for $type {
            fn counts(&self) -> [u64; 4] {
                self.counts().unwrap()
            }
            fn effects(&self) -> Vec<(String, Intent)> {
                self.intentions().unwrap()
            }
            fn events(&self) -> Vec<Row> {
                self.event_rows().unwrap()
            }
            fn observer(&self, observer: Option<Observer>) {
                self.on_commit(observer)
            }
        }
    };
}
inspect!(rom_sqlite::Sqlite);
inspect!(rom_redb::Redb);
#[derive(Clone, Copy, Debug)]
enum Backend {
    Sqlite,
    Redb,
}
impl Backend {
    fn name(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Redb => "redb",
        }
    }
    fn open(self, path: &Path) -> Arc<dyn Inspect> {
        let store = self.open_unregistered(path);
        let mut other = Record::descriptor();
        other.kind = "other".into();
        store
            .register(&[Record::descriptor(), PrivateRecord::descriptor(), other])
            .unwrap();
        store
    }
    fn open_unregistered(self, path: &Path) -> Arc<dyn Inspect> {
        match self {
            Self::Sqlite => Arc::new(rom_sqlite::Sqlite::open(path).unwrap()),
            Self::Redb => Arc::new(rom_redb::Redb::open(path).unwrap()),
        }
    }
}
const BACKENDS: [Backend; 2] = [Backend::Sqlite, Backend::Redb];

#[derive(Clone, Resource)]
#[resource(name = "private-records")]
struct PrivateRecord {
    owner: String,
    secret: String,
}
fn private_runtime(storage: Arc<dyn Storage>) -> Runtime {
    Runtime::builder()
        .resource(
            PrivateRecord::definition()
                .policy(|actor, _, row| actor.subject == row.owner)
                .field_policy(|_, access, field, _| {
                    matches!(access, rom::Access::Write) || field != "secret"
                }),
        )
        .build(storage, Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
}
#[tokio::test]
async fn shared_deleted_history_retains_private_authorization_across_restart() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        let runtime = private_runtime(store.clone());
        let alice = Actor::trusted("local", "alice");
        let mallory = Actor::trusted("local", "mallory");
        let created = runtime
            .invoke_projected(
                &alice,
                Command::create(
                    "private-alice-id",
                    PrivateRecord {
                        owner: "alice".into(),
                        secret: "NEVER-DISCLOSE".into(),
                    },
                )
                .idempotency("create")
                .into(),
            )
            .await
            .unwrap();
        assert!(
            !serde_json::to_string(&created)
                .unwrap()
                .contains("NEVER-DISCLOSE")
        );
        assert!(
            runtime
                .journal(&mallory, "private-records", None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        let delete: rom::Invocation = Command::<PrivateRecord>::delete("private-alice-id")
            .at_revision(1)
            .idempotency("delete")
            .into();
        let outcome = runtime.invoke(&alice, delete.clone()).await.unwrap();
        assert_eq!(
            serde_json::to_value(&outcome).unwrap(),
            json!({
                "key":{"kind":"private-records","id":"private-alice-id"},"revision":2,"value":null
            })
        );
        assert!(
            runtime
                .journal(&mallory, "private-records", None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        let mut subscription = runtime
            .subscribe(&mallory, "private-records", None)
            .await
            .unwrap();
        assert!(subscription.next().await.unwrap().events.is_empty());
        drop(subscription);
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(store);
        let runtime = private_runtime(backend.open(&scratch.path()));
        let replay = runtime.invoke(&alice, delete).await.unwrap();
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&outcome).unwrap()
        );
        let owner_history = runtime
            .journal(&alice, "private-records", None)
            .await
            .unwrap();
        assert_eq!(owner_history.events.len(), 1);
        assert_eq!(owner_history.events[0].view.revision, 2);
        assert!(owner_history.events[0].view.value.is_none());
        assert!(
            !serde_json::to_string(&owner_history)
                .unwrap()
                .contains("NEVER-DISCLOSE")
        );
        assert!(
            runtime
                .journal(&mallory, "private-records", None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        runtime
            .invoke_projected(
                &mallory,
                Command::create(
                    "private-alice-id",
                    PrivateRecord {
                        owner: "mallory".into(),
                        secret: "NEW-PRIVATE-VALUE".into(),
                    },
                )
                .at_revision(2)
                .idempotency("recreate")
                .into(),
            )
            .await
            .unwrap();
        let new_owner_history = runtime
            .journal(&mallory, "private-records", None)
            .await
            .unwrap();
        assert_eq!(new_owner_history.events.len(), 1);
        assert_eq!(new_owner_history.events[0].view.revision, 3);
        assert!(
            !serde_json::to_string(&new_owner_history)
                .unwrap()
                .contains("NEW-PRIVATE-VALUE")
        );
        assert!(
            runtime
                .journal(&alice, "private-records", None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        runtime.shutdown().await.unwrap();
    }
}
struct Scratch(PathBuf);
#[tokio::test]
async fn shared_legacy_tombstones_without_policy_context_fail_closed() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        let mut legacy = create("legacy");
        legacy.receipt.row.key.kind = "private-records".into();
        legacy.receipt.row.value = Some(json!({"owner":"alice","secret":"legacy-secret"}));
        store.commit(&legacy).unwrap();
        legacy.expected = Some(1);
        legacy.receipt.identity = "legacy-delete".into();
        legacy.receipt.row.revision = 2;
        legacy.receipt.row.value = None;
        store.commit(&legacy).unwrap();
        let runtime = private_runtime(store);
        assert!(
            runtime
                .journal(&Actor::trusted("local", "alice"), "private-records", None)
                .await
                .unwrap()
                .events
                .is_empty()
        );
        runtime.shutdown().await.unwrap();
    }
}
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rom-storage-conformance-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> PathBuf {
        self.0.join("database")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn create(id: &str) -> Bundle {
    Bundle {
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
        expected: None,
        receipt: Receipt {
            replay_version: None,
            identity: format!("create-{id}"),
            fingerprint: format!("create-{id}-false"),
            row: Row {
                protected: Default::default(),
                key: Key {
                    kind: "records".into(),
                    id: id.into(),
                },
                revision: 1,
                value: Some(json!({"done":false})),
            },
        },
        changed: true,
        effects: vec![],
    }
}
fn update() -> Bundle {
    let mut b = create("one");
    b.expected = Some(1);
    b.receipt.identity = "update-one".into();
    b.receipt.fingerprint = "update-one-true".into();
    b.receipt.row.revision = 2;
    b.receipt.row.value = Some(json!({"done":true}));
    b.effects = vec![
        Intent::new("mail", json!({"value":1})),
        Intent::new("audit", json!({"value":2})),
    ];
    b
}
fn assert_bundle(store: &dyn Inspect) {
    let b = update();
    assert_eq!(
        store.load(&b.receipt.row.key).unwrap(),
        Some(b.receipt.row.clone())
    );
    assert_eq!(
        store.receipt(&b.receipt.identity).unwrap(),
        Some(b.receipt.clone())
    );
    assert_eq!(store.counts(), [1, 2, 2, 2]);
    assert_eq!(
        store.events(),
        vec![create("one").receipt.row, b.receipt.row]
    );
    assert_eq!(
        store.effects(),
        b.effects
            .into_iter()
            .map(|i| (b.receipt.identity.clone(), i))
            .collect::<Vec<_>>()
    );
}
#[test]
fn shared_atomic_bundle_and_noop_identity_validation() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        store.commit(&create("one")).unwrap();
        let b = update();
        assert_eq!(store.commit(&b).unwrap(), b.receipt);
        assert_bundle(&*store);
        assert_eq!(store.commit(&b).unwrap(), b.receipt);
        assert_bundle(&*store);
        let mut conflict = b.clone();
        conflict.receipt.fingerprint = "different".into();
        assert_eq!(store.commit(&conflict), Err(Error::IdentityMismatch));
        let mut noop = b.clone();
        noop.expected = Some(2);
        noop.changed = false;
        noop.effects.clear();
        noop.receipt.identity = "noop".into();
        noop.receipt.fingerprint = "noop".into();
        store.commit(&noop).unwrap();
        assert_eq!(store.counts(), [1, 2, 3, 2]);
        let mut invalid = noop.clone();
        invalid.receipt.identity = "invalid".into();
        invalid.effects.push(Intent::new("bad", json!(null)));
        assert_eq!(store.commit(&invalid), Err(Error::NotCommitted));
        invalid.effects.clear();
        invalid.receipt.row.value = Some(json!({"done":false}));
        assert_eq!(store.commit(&invalid), Err(Error::NotCommitted));
        assert_eq!(store.counts(), [1, 2, 3, 2]);
    }
}
#[test]
fn shared_rollback_after_every_write_and_before_commit() {
    for backend in BACKENDS {
        for fail in [1, 2, 3, 4, 5, 6, 0] {
            let scratch = Scratch::new();
            let store = backend.open(&scratch.path());
            let initial = create("one");
            store.commit(&initial).unwrap();
            store.observer(Some(Arc::new(move |point| {
                if point == fail {
                    Err(Error::NotCommitted)
                } else {
                    Ok(())
                }
            })));
            assert_eq!(
                store.commit(&update()),
                Err(Error::NotCommitted),
                "{backend:?} write {fail}"
            );
            assert_eq!(
                store.load(&initial.receipt.row.key).unwrap(),
                Some(initial.receipt.row.clone())
            );
            assert_eq!(store.receipt("update-one").unwrap(), None);
            assert_eq!(store.events(), vec![initial.receipt.row]);
            assert_eq!(store.counts(), [1, 1, 1, 0]);
            assert!(store.effects().is_empty());
            store.observer(None);
            store.commit(&update()).unwrap();
            assert_bundle(&*store);
        }
    }
}
#[test]
fn shared_lost_commit_acknowledgement_reopen_resolves_original_identity() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        {
            let store = backend.open(&scratch.path());
            store.commit(&create("one")).unwrap();
            store.observer(Some(Arc::new(|point| {
                if point == usize::MAX {
                    Err(Error::NotCommitted)
                } else {
                    Ok(())
                }
            })));
            assert_eq!(store.commit(&update()), Err(Error::Unknown));
        }
        let store = backend.open(&scratch.path());
        assert_bundle(&*store);
        assert_eq!(store.commit(&update()).unwrap(), update().receipt);
        assert_bundle(&*store);
    }
}
#[test]
fn shared_revision_race_arbitrates_one_transition() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        store.commit(&create("one")).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|i| {
                    let store = store.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        let mut b = update();
                        b.receipt.identity = format!("racer-{i}");
                        b.receipt.fingerprint = format!("racer-{i}");
                        barrier.wait();
                        store.commit(&b)
                    })
                })
                .collect();
            barrier.wait();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == Err(Error::Conflict))
                .count(),
            1
        );
        assert_eq!(store.counts(), [1, 2, 2, 2]);
    }
}
#[test]
fn shared_same_identity_race_returns_one_receipt() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        store.commit(&create("one")).unwrap();
        let barrier = Arc::new(Barrier::new(3));
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|_| {
                    let store = store.clone();
                    let barrier = barrier.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        store.commit(&update())
                    })
                })
                .collect();
            barrier.wait();
            for handle in handles {
                assert_eq!(handle.join().unwrap().unwrap(), update().receipt);
            }
        });
        assert_bundle(&*store);
    }
}
#[test]
fn shared_bounded_snapshot_exact_limit_overflow_and_kind_isolation() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        let a = create("a");
        let z = create("z");
        store.commit(&z).unwrap();
        store.commit(&a).unwrap();
        let bytes = serde_json::to_vec(&a.receipt.row).unwrap().len()
            + serde_json::to_vec(&z.receipt.row).unwrap().len();
        assert_eq!(
            store.snapshot("records", 2, bytes).unwrap(),
            vec![a.receipt.row, z.receipt.row]
        );
        assert_eq!(store.snapshot("records", 1, bytes), Err(Error::TooLarge));
        assert_eq!(
            store.snapshot("records", 2, bytes - 1),
            Err(Error::TooLarge)
        );
        assert_eq!(store.snapshot("records", 0, bytes), Err(Error::TooLarge));
        assert!(store.snapshot("absent", 0, 0).unwrap().is_empty());
        let mut foreign = create("a");
        foreign.receipt.identity = "other".into();
        foreign.receipt.row.key.kind = "other".into();
        store.commit(&foreign).unwrap();
        assert_eq!(store.snapshot("records", 2, bytes).unwrap().len(), 2);
    }
}
#[derive(Clone, Resource)]
#[resource(name = "records")]
struct Record {
    done: bool,
}
#[tokio::test]
async fn shared_core_typed_actions_work_without_application_repositories() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        let storage: Arc<dyn Storage> = store.clone();
        let runtime = Runtime::builder()
            .resource(
                Record::definition()
                    .allow_all_fields()
                    .policy(|_, _, _| true),
            )
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let actor = Actor::trusted("host", "owner");
        let create = Command::create("one", Record { done: false }).idempotency("create");
        assert_eq!(
            runtime
                .execute(&actor, create.clone())
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(runtime.execute(&actor, create).await.unwrap().revision, 1);
        assert_eq!(
            runtime
                .execute(
                    &actor,
                    Command::replace("one", Record { done: true })
                        .at_revision(1)
                        .idempotency("update")
                )
                .await
                .unwrap()
                .revision,
            2
        );
        assert_eq!(store.counts(), [1, 2, 2, 0]);
        runtime.shutdown().await.unwrap();
    }
}
#[test]
fn shared_actual_subprocess_exit_at_every_write_and_commit_boundary() {
    for backend in BACKENDS {
        for point in [1, 2, 3, 4, 5, 6, 0, usize::MAX] {
            let scratch = Scratch::new();
            {
                let store = backend.open(&scratch.path());
                store.commit(&create("one")).unwrap();
            }
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "crash_child", "--ignored", "--nocapture"])
                .env("ROM_TEST_BACKEND", backend.name())
                .env("ROM_TEST_PATH", scratch.path())
                .env("ROM_TEST_POINT", point.to_string())
                .output()
                .unwrap();
            assert_eq!(
                status.status.code(),
                Some(86),
                "{backend:?} point {point}: {}",
                String::from_utf8_lossy(&status.stderr)
            );
            let store = backend.open(&scratch.path());
            if point == usize::MAX {
                assert_bundle(&*store);
            } else {
                assert_eq!(store.counts(), [1, 1, 1, 0]);
                assert_eq!(
                    store.load(&create("one").receipt.row.key).unwrap(),
                    Some(create("one").receipt.row)
                );
                assert_eq!(store.receipt("update-one").unwrap(), None);
            }
            assert_eq!(store.commit(&update()).unwrap(), update().receipt);
            assert_bundle(&*store);
        }
    }
}
#[test]
#[ignore = "subprocess fixture invoked by shared_actual_subprocess_exit_at_every_write_and_commit_boundary"]
fn crash_child() {
    let backend = match std::env::var("ROM_TEST_BACKEND").unwrap().as_str() {
        "sqlite" => Backend::Sqlite,
        "redb" => Backend::Redb,
        _ => panic!("unknown backend"),
    };
    let path = PathBuf::from(std::env::var_os("ROM_TEST_PATH").unwrap());
    let point: usize = std::env::var("ROM_TEST_POINT").unwrap().parse().unwrap();
    let store = backend.open(&path);
    store.observer(Some(Arc::new(move |at| {
        if at == point {
            std::process::exit(86);
        }
        Ok(())
    })));
    store.commit(&update()).unwrap();
    panic!("crash checkpoint was not reached");
}

#[test]
fn shared_open_rejects_oversized_encoded_row_before_deserializing() {
    use redb::{Database, TableDefinition};
    for backend in BACKENDS {
        let scratch = Scratch::new();
        drop(backend.open_unregistered(&scratch.path()));
        let invalid = "x".repeat(1024);
        match backend {
            Backend::Sqlite => {
                let c = rusqlite::Connection::open(scratch.path()).unwrap();
                c.execute(
                    "INSERT INTO resources(kind,id,revision,data) VALUES ('records','bad',1,?)",
                    [&invalid],
                )
                .unwrap();
            }
            Backend::Redb => {
                let db = Database::open(scratch.path()).unwrap();
                let tx = db.begin_write().unwrap();
                {
                    let mut table = tx
                        .open_table(TableDefinition::<(&str, &str), &str>::new("resources"))
                        .unwrap();
                    table.insert(("records", "bad"), invalid.as_str()).unwrap();
                }
                tx.commit().unwrap();
            }
        }
        for (max_bytes, expected) in [(10, Error::TooLarge), (100_000, Error::Storage)] {
            let limits = rom_backup::BackupLimits {
                max_bytes,
                ..Default::default()
            };
            let result = match backend {
                Backend::Sqlite => rom_sqlite::Sqlite::open_with_validation_limits(
                    scratch.path(),
                    Default::default(),
                    limits,
                )
                .map(|_| ()),
                Backend::Redb => rom_redb::Redb::open_with_validation_limits(
                    scratch.path(),
                    Default::default(),
                    limits,
                )
                .map(|_| ()),
            };
            assert_eq!(result, Err(expected));
        }
    }
}
#[test]
fn redb_unknown_format_rejected_without_application_table_writes() {
    use redb::{Database, ReadableDatabase, TableDefinition};
    let scratch = Scratch::new();
    let meta = TableDefinition::<&str, u64>::new("rom_metadata");
    {
        let db = Database::create(scratch.path()).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(meta).unwrap().insert("format", 999).unwrap();
        tx.commit().unwrap();
    }
    assert!(matches!(
        rom_redb::Redb::open(scratch.path()),
        Err(Error::Unsupported(_))
    ));
    let db = Database::open(scratch.path()).unwrap();
    let tx = db.begin_read().unwrap();
    assert_eq!(tx.list_tables().unwrap().count(), 1);
    assert_eq!(
        tx.open_table(meta)
            .unwrap()
            .get("format")
            .unwrap()
            .unwrap()
            .value(),
        999
    );
}
#[test]
fn shared_missing_format_marker_on_nonempty_database_is_not_implicitly_migrated() {
    use redb::{Database, TableDefinition};
    for backend in BACKENDS {
        let scratch = Scratch::new();
        match backend {
            Backend::Sqlite => {
                let c = rusqlite::Connection::open(scratch.path()).unwrap();
                c.execute("CREATE TABLE legacy(value TEXT)", []).unwrap();
                assert!(matches!(
                    rom_sqlite::Sqlite::open(scratch.path()),
                    Err(Error::Unsupported(_))
                ));
            }
            Backend::Redb => {
                let db = Database::create(scratch.path()).unwrap();
                let tx = db.begin_write().unwrap();
                tx.open_table(TableDefinition::<&str, &str>::new("legacy"))
                    .unwrap()
                    .insert("key", "value")
                    .unwrap();
                tx.commit().unwrap();
                drop(db);
                assert!(matches!(
                    rom_redb::Redb::open(scratch.path()),
                    Err(Error::Unsupported(_))
                ));
            }
        }
    }
}
#[test]
fn shared_delete_tombstone_and_invalid_revision_preserve_contract() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path());
        store.commit(&create("one")).unwrap();
        let mut b = update();
        b.receipt.row.revision = 9;
        assert_eq!(store.commit(&b), Err(Error::NotCommitted));
        assert_eq!(store.counts(), [1, 1, 1, 0]);
        b.receipt.row.revision = 2;
        b.receipt.row.value = None;
        b.effects.clear();
        assert_eq!(store.commit(&b).unwrap(), b.receipt);
        assert_eq!(
            store.load(&b.receipt.row.key).unwrap(),
            Some(b.receipt.row.clone())
        );
        assert_eq!(
            store.snapshot("records", 1, usize::MAX).unwrap(),
            vec![b.receipt.row]
        );
        assert_eq!(store.counts(), [1, 2, 2, 0]);
    }
}
