use rom::*;
use rom_backup::BackupLimits;
use rom_sqlite::Sqlite;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "indexed-items")]
struct Item {
    name: String,
    amount: u64,
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-native-index-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn bundle(id: usize, revision: u64, amount: Option<u64>) -> Bundle {
    Bundle {
        expected: (revision > 1).then_some(revision - 1),
        changed: true,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: format!("{id}-{revision}"),
            fingerprint: format!("{id}-{revision}-{amount:?}"),
            row: Row {
                key: Key {
                    kind: Item::KIND.into(),
                    id: format!("{id:04}"),
                },
                revision,
                value: amount.map(|amount| {
                    Item {
                        name: format!("item {id}"),
                        amount,
                    }
                    .encode()
                }),
                protected: ProtectedMetadata::default(),
            },
        },
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
fn request() -> StorageQuery {
    StorageQuery {
        descriptor: Item::descriptor(),
        spec: QuerySpec::equal("amount", json!(42)),
        semantics: QUERY_SEMANTICS_VERSION,
        selection: SelectionMode::UniformReadAndFields,
    }
}
fn bounds() -> QueryBounds {
    QueryBounds {
        max_rows: 1000,
        max_bytes: 1_000_000,
    }
}
fn seeded(s: &Scratch) -> Sqlite {
    let db = Sqlite::open(s.path("source.db")).unwrap();
    db.register(&[Item::descriptor()]).unwrap();
    for id in 0..128 {
        db.commit(&bundle(id, 1, Some(id as u64))).unwrap();
    }
    db
}
fn native(db: &Sqlite) -> (Vec<Row>, QuerySnapshot, KindAdmission) {
    match db.query_read(&request(), bounds()).unwrap() {
        QueryRead::NativeCandidates {
            rows,
            binding,
            admission,
        } => (rows, binding.snapshot, admission),
        other => panic!("expected native selective read: {other:?}"),
    }
}
#[test]
fn native_commit_replay_rollback_delete_and_reopen_are_coherent() {
    let s = Scratch::new();
    let db = seeded(&s);
    let (rows, initial, admission) = native(&db);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.id, "0042");
    assert_eq!(admission.rows, 128);
    db.commit(&bundle(42, 1, Some(42))).unwrap();
    assert_eq!(native(&db).1, initial);
    db.inject_fault(2);
    assert_eq!(
        db.commit(&bundle(42, 2, Some(99))),
        Err(Error::NotCommitted)
    );
    assert_eq!(native(&db).1, initial);
    let change = bundle(42, 2, Some(99));
    db.commit(&change).unwrap();
    let (after, generation, _) = native(&db);
    assert!(after.is_empty());
    assert_eq!(generation.generation, initial.generation + 1);
    db.commit(&change).unwrap();
    assert_eq!(native(&db).1, generation);
    db.commit(&bundle(0, 2, Some(42))).unwrap();
    assert_eq!(native(&db).0[0].key.id, "0000");
    db.commit(&bundle(0, 3, None)).unwrap();
    assert!(native(&db).0.is_empty());
    assert_eq!(
        db.query_read(
            &request(),
            QueryBounds {
                max_rows: 127,
                ..bounds()
            }
        ),
        Err(Error::TooLarge)
    );
    let snapshot = native(&db).1;
    drop(db);
    let reopened = Sqlite::open(s.path("source.db")).unwrap();
    assert_eq!(native(&reopened).1, snapshot);
}
#[test]
fn backup_restore_and_explicit_rebuild_preserve_logical_data_and_fence_indexes() {
    let s = Scratch::new();
    let db = seeded(&s);
    let before = native(&db);
    let counts = db.counts().unwrap();
    db.backup_to(s.path("archive"), BackupLimits::default())
        .unwrap();
    let restored = Sqlite::restore_from(
        s.path("archive"),
        s.path("restored.db"),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(native(&restored).0, before.0);
    assert_ne!(native(&restored).1.store, before.1.store);
    assert_eq!(restored.counts().unwrap(), counts);
    drop(restored);
    drop(db);
    let raw = rusqlite::Connection::open(s.path("source.db")).unwrap();
    raw.execute("DELETE FROM query_keys WHERE id='0042'", [])
        .unwrap();
    drop(raw);
    assert!(matches!(
        Sqlite::open(s.path("source.db")),
        Err(Error::Storage)
    ));
    let source = std::fs::read(s.path("source.db")).unwrap();
    assert!(matches!(
        Sqlite::rebuild_indexes_from_observed(
            s.path("source.db"),
            s.path("interrupted.db"),
            BackupLimits::default(),
            || Err(Error::NotCommitted)
        ),
        Err(Error::NotCommitted)
    ));
    assert!(!s.path("interrupted.db").exists());
    assert!(
        std::fs::read(s.path("source.db")).unwrap() == source,
        "rebuild modified source"
    );
    let repaired = Sqlite::rebuild_indexes_from(
        s.path("source.db"),
        s.path("repaired.db"),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(native(&repaired).0, before.0);
    assert_ne!(native(&repaired).1.store, before.1.store);
    assert_eq!(repaired.counts().unwrap(), counts);
    assert!(
        std::fs::read(s.path("source.db")).unwrap() == source,
        "rebuild modified source"
    );
    assert!(
        Sqlite::rebuild_indexes_from(
            s.path("source.db"),
            s.path("tiny.db"),
            BackupLimits {
                max_records: 1,
                ..BackupLimits::default()
            }
        )
        .is_err()
    );
    assert!(!s.path("tiny.db").exists());
}

#[test]
fn format_six_upgrade_preserves_retry_epochs_and_does_not_rebind_receipt_origins() {
    let s = Scratch::new();
    let db = seeded(&s);
    drop(db);
    let c = rusqlite::Connection::open(s.path("source.db")).unwrap();
    let text: String = c
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .unwrap();
    let mut state: StorageState = serde_json::from_str(&text).unwrap();
    let epochs = RetryEpochs {
        current: 4,
        admission_floor: 2,
        replay_floor: 0,
    };
    state.apply_retention(epochs, 0, 128, 0).unwrap();
    c.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(&state).unwrap()],
    )
    .unwrap();
    c.execute_batch("DROP TABLE query_keys; DROP TABLE query_kinds; DROP TABLE query_profile; PRAGMA user_version=6;").unwrap();
    drop(c);
    let original = std::fs::read(s.path("source.db")).unwrap();
    assert!(matches!(
        Sqlite::open(s.path("source.db")),
        Err(Error::Unsupported(_))
    ));
    let upgraded = Sqlite::upgrade_from(
        s.path("source.db"),
        s.path("upgraded.db"),
        &[Item::descriptor()],
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(upgraded.retry_epochs().unwrap(), epochs);
    assert_eq!(
        upgraded.receipt("42-1").unwrap().unwrap().replay_version,
        None
    );
    assert_eq!(native(&upgraded).0[0].key.id, "0042");
    assert!(
        std::fs::read(s.path("source.db")).unwrap() == original,
        "upgrade modified source"
    );
}

#[tokio::test]
async fn native_and_reference_resources_keep_same_residual_order_and_live_results() {
    use std::sync::Arc;
    let s = Scratch::new();
    let sqlite = Arc::new(seeded(&s));
    let redb = Arc::new(rom_redb::Redb::open(s.path("reference.redb")).unwrap());
    redb.register(&[Item::descriptor()]).unwrap();
    for id in 0..128 {
        redb.commit(&bundle(id, 1, Some(id as u64))).unwrap();
    }
    let cpus = Runtime::shared_cpu_pool(2).unwrap();
    let make = |storage: Arc<dyn Storage>| {
        Runtime::builder()
            .resource(
                Item::definition()
                    .policy(|_, _, _| true)
                    .read_policy(|_| true)
                    .allow_all_fields(),
            )
            .build(storage, cpus.clone())
            .unwrap()
    };
    let native_runtime = make(sqlite.clone());
    let reference_runtime = make(redb);
    let actor = Actor::trusted("tests", "owner");
    for spec in [
        QuerySpec::equal("amount", json!(42)),
        QuerySpec::all()
            .compare("amount", CompareOp::Ge, json!(120))
            .compare("amount", CompareOp::Lt, json!(124))
            .order_by("name", Direction::Desc)
            .limit(2),
        QuerySpec::all()
            .compare("amount", CompareOp::Ne, json!(42))
            .order_by("amount", Direction::Desc)
            .limit(3),
    ] {
        let left = native_runtime
            .query_spec_projected(&actor, Item::KIND, spec.clone())
            .await
            .unwrap();
        let right = reference_runtime
            .query_spec_projected(&actor, Item::KIND, spec)
            .await
            .unwrap();
        assert_eq!(left, right);
    }
    let mut live = native_runtime
        .live(&actor, Item::amount_field().equals(42))
        .await
        .unwrap();
    assert_eq!(live.changed().await.unwrap()[0].id, "0042");
    native_runtime
        .execute(
            &actor,
            Command::<Item>::delete("0042")
                .at_revision(1)
                .idempotency("delete-live"),
        )
        .await
        .unwrap();
    assert!(live.changed().await.unwrap().is_empty());
    native_runtime.shutdown().await.unwrap();
    reference_runtime.shutdown().await.unwrap();
}
