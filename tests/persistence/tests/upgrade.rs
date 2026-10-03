use rom::*;
use rom_backup::BackupLimits;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "upgrade-nodes")]
struct Node {
    label: String,
    target: Option<ResourceRef<Node>>,
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-native-upgrade-{}-{}",
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

struct Legacy {
    state: StorageState,
    bundles: Vec<Bundle>,
}
fn create(id: &str, target: Option<&str>) -> Bundle {
    let row = Row {
        key: Key {
            kind: Node::KIND.into(),
            id: id.into(),
        },
        revision: 1,
        value: Some(
            Node {
                label: id.into(),
                target: target.map(|id| ResourceRef::new(id).unwrap()),
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    Bundle {
        expected: None,
        receipt: Receipt {
            replay_version: None,
            identity: format!("create-{id}"),
            fingerprint: format!("node-{id}"),
            row,
        },
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
impl Legacy {
    fn new() -> Self {
        let mut state = StorageState::new(StorageLimits::default()).unwrap();
        let parent = create("parent", None);
        let mut child = create("child", Some("parent"));
        child.receipt.row.protected.source_provenance = Some(SourceProvenance {
            source: "config".into(),
            version: "v1".into(),
            generation: 9,
            field_origins: [("label".into(), "environment".into())].into(),
        });
        child
            .effects
            .push(Intent::new("audit", json!({"message":"retained"})));
        child.reaction_limits = Some(ReactionLimits::default());
        child.reactions.push(PendingWork {
            id: "pending-child".into(),
            cause: Cause {
                root: "root-child".into(),
                parent: None,
                depth: 1,
                started_at: 0,
                path: vec!["pending-child".into()],
            },
            definition: "audit-child".into(),
            version: 1,
            service_key: "local-service".into(),
            payload: WorkPayload::Notification {
                source: child.receipt.row.clone(),
                payload: json!({"message":"deliver later"}),
            },
        });
        for b in [&parent, &child] {
            state.bundle(b).unwrap();
        }
        Self {
            state,
            bundles: vec![parent, child],
        }
    }
    fn write(&self, redb: bool, path: &Path) {
        if redb {
            self.write_redb(path);
        } else {
            self.write_sqlite(path);
        }
    }
    fn write_sqlite(&self, path: &Path) {
        let mut c = rusqlite::Connection::open(path).unwrap();
        let tx = c.transaction().unwrap();
        tx.execute_batch("CREATE TABLE resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));
CREATE TABLE receipts(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE events(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE effects(identity TEXT NOT NULL,ordinal INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(identity,ordinal));
CREATE TABLE rom_state(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL);
PRAGMA user_version=3;").unwrap();
        tx.execute(
            "INSERT INTO rom_state VALUES(1,?)",
            [serde_json::to_string(&self.state).unwrap()],
        )
        .unwrap();
        for b in &self.bundles {
            insert_sqlite_bundle(&tx, b);
        }
        tx.commit().unwrap();
    }
    fn write_redb(&self, path: &Path) {
        use redb::TableDefinition;
        let db = redb::Database::create(path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(TableDefinition::<&str, u64>::new("rom_metadata"))
            .unwrap()
            .insert("format", 3)
            .unwrap();
        tx.open_table(TableDefinition::<&str, &str>::new("rom_state"))
            .unwrap()
            .insert(
                "state",
                serde_json::to_string(&self.state).unwrap().as_str(),
            )
            .unwrap();
        {
            let mut rows = tx
                .open_table(TableDefinition::<(&str, &str), &str>::new("resources"))
                .unwrap();
            let mut receipts = tx
                .open_table(TableDefinition::<&str, &str>::new("receipts"))
                .unwrap();
            let mut events = tx
                .open_table(TableDefinition::<&str, &str>::new("events"))
                .unwrap();
            let mut effects = tx
                .open_table(TableDefinition::<(&str, u64), &str>::new("effects"))
                .unwrap();
            for b in &self.bundles {
                let row = &b.receipt.row;
                let data = serde_json::to_string(row).unwrap();
                rows.insert((row.key.kind.as_str(), row.key.id.as_str()), data.as_str())
                    .unwrap();
                receipts
                    .insert(
                        b.receipt.identity.as_str(),
                        serde_json::to_string(&b.receipt).unwrap().as_str(),
                    )
                    .unwrap();
                events
                    .insert(b.receipt.identity.as_str(), data.as_str())
                    .unwrap();
                for (i, effect) in b.effects.iter().enumerate() {
                    effects
                        .insert(
                            (b.receipt.identity.as_str(), i as u64),
                            serde_json::to_string(effect).unwrap().as_str(),
                        )
                        .unwrap();
                }
            }
        }
        tx.commit().unwrap();
    }
}
fn insert_sqlite_bundle(tx: &rusqlite::Transaction<'_>, b: &Bundle) {
    let row = &b.receipt.row;
    let data = serde_json::to_string(row).unwrap();
    tx.execute(
        "INSERT INTO resources VALUES(?,?,?,?)",
        rusqlite::params![
            row.key.kind,
            row.key.id,
            i64::try_from(row.revision).unwrap(),
            data
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO receipts VALUES(?,?)",
        rusqlite::params![
            b.receipt.identity,
            serde_json::to_string(&b.receipt).unwrap()
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO events VALUES(?,?)",
        rusqlite::params![b.receipt.identity, data],
    )
    .unwrap();
    for (i, effect) in b.effects.iter().enumerate() {
        tx.execute(
            "INSERT INTO effects VALUES(?,?,?)",
            rusqlite::params![
                b.receipt.identity,
                i as i64,
                serde_json::to_string(effect).unwrap()
            ],
        )
        .unwrap();
    }
}
fn upgrade(
    redb: bool,
    source: &Path,
    destination: &Path,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Box<dyn Storage>> {
    if redb {
        rom_redb::Redb::upgrade_from(source, destination, descriptors, limits)
            .map(|s| Box::new(s) as Box<dyn Storage>)
    } else {
        rom_sqlite::Sqlite::upgrade_from(source, destination, descriptors, limits)
            .map(|s| Box::new(s) as Box<dyn Storage>)
    }
}
fn remove_parent() -> Bundle {
    let mut b = create("parent", None);
    b.expected = Some(1);
    b.receipt.identity = "delete-parent".into();
    b.receipt.fingerprint = "delete-parent".into();
    b.receipt.row.revision = 2;
    b.receipt.row.value = None;
    b
}

#[test]
fn explicit_native_upgrade_preserves_durable_records_and_enforces_references() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let destination = scratch.path("destination");
        let fixture = Legacy::new();
        fixture.write(redb, &source);
        let ordinary_open = if redb {
            rom_redb::Redb::open(&source).map(|_| ())
        } else {
            rom_sqlite::Sqlite::open(&source).map(|_| ())
        };
        assert!(matches!(ordinary_open, Err(Error::Unsupported(_))));
        // Capture the upgrade baseline separately from the ordinary-open assertion.
        let original = std::fs::read(&source).unwrap();
        let upgraded = upgrade(
            redb,
            &source,
            &destination,
            &[Node::descriptor()],
            BackupLimits::default(),
        )
        .unwrap();
        assert!(
            std::fs::read(&source).unwrap() == original,
            "source must remain byte-identical"
        );
        for bundle in &fixture.bundles {
            let mut expected_receipt = bundle.receipt.clone();
            expected_receipt.replay_version = Some(1);
            assert_eq!(
                upgraded.load(&bundle.receipt.row.key).unwrap(),
                Some(bundle.receipt.row.clone())
            );
            assert_eq!(
                upgraded.receipt(&bundle.receipt.identity).unwrap(),
                Some(expected_receipt.clone())
            );
            assert_eq!(
                upgraded.commit(bundle).unwrap(),
                expected_receipt,
                "receipt replay must not reapply the mutation"
            );
        }
        assert_eq!(
            upgraded
                .journal(Node::KIND, None, 100, 100_000)
                .unwrap()
                .events
                .len(),
            2
        );
        let work = upgraded.reaction_records().unwrap();
        assert_eq!(work.len(), 1);
        assert_eq!(work[0].pending, fixture.bundles[1].reactions[0]);
        assert_eq!(upgraded.commit(&remove_parent()), Err(Error::Conflict));
        drop(upgraded);
        let intentions = if redb {
            rom_redb::Redb::open(&destination)
                .unwrap()
                .intentions()
                .unwrap()
        } else {
            rom_sqlite::Sqlite::open(&destination)
                .unwrap()
                .intentions()
                .unwrap()
        };
        let expected_intentions: Vec<_> = fixture
            .bundles
            .iter()
            .flat_map(|bundle| {
                bundle
                    .effects
                    .iter()
                    .cloned()
                    .map(|effect| (bundle.receipt.identity.clone(), effect))
            })
            .collect();
        assert_eq!(intentions, expected_intentions);
    }
}

#[derive(Clone, Copy, Debug)]
enum InvalidSource {
    MissingCatalog,
    WrongShape,
    DanglingTarget,
    MalformedRow,
    ByteLimit,
    RecordLimit,
}
fn corrupt_row(redb: bool, source: &Path) {
    if redb {
        let db = redb::Database::open(source).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<(&str, &str), &str>::new(
            "resources",
        ))
        .unwrap()
        .insert((Node::KIND, "child"), "not JSON")
        .unwrap();
        tx.commit().unwrap();
    } else {
        let c = rusqlite::Connection::open(source).unwrap();
        c.execute("UPDATE resources SET data='not JSON' WHERE id='child'", [])
            .unwrap();
    }
}
#[test]
fn invalid_native_upgrade_leaves_source_unchanged_and_destination_absent() {
    for redb in [false, true] {
        for case in [
            InvalidSource::MissingCatalog,
            InvalidSource::WrongShape,
            InvalidSource::DanglingTarget,
            InvalidSource::MalformedRow,
            InvalidSource::ByteLimit,
            InvalidSource::RecordLimit,
        ] {
            let scratch = Scratch::new();
            let source = scratch.path("source");
            let destination = scratch.path("destination");
            let mut fixture = Legacy::new();
            if matches!(case, InvalidSource::DanglingTarget) {
                fixture.bundles[1].receipt.row.value = Some(
                    Node {
                        label: "child".into(),
                        target: Some(ResourceRef::new("missing").unwrap()),
                    }
                    .encode(),
                );
                fixture.state = StorageState::new(StorageLimits::default()).unwrap();
                for b in &fixture.bundles {
                    fixture.state.bundle(b).unwrap();
                }
            }
            fixture.write(redb, &source);
            if matches!(case, InvalidSource::MalformedRow) {
                corrupt_row(redb, &source);
            }
            let original = std::fs::read(&source).unwrap();
            let mut descriptors = vec![Node::descriptor()];
            let mut limits = BackupLimits::default();
            match case {
                InvalidSource::MissingCatalog => descriptors.clear(),
                InvalidSource::WrongShape => {
                    descriptors[0]
                        .fields
                        .iter_mut()
                        .find(|f| f.name == "label")
                        .unwrap()
                        .shape = Shape::Bool
                }
                InvalidSource::ByteLimit => limits.max_bytes = 1,
                InvalidSource::RecordLimit => limits.max_records = 1,
                _ => {}
            }
            let result = upgrade(redb, &source, &destination, &descriptors, limits);
            assert!(result.is_err(), "must reject {case:?} on redb={redb}");
            if matches!(case, InvalidSource::ByteLimit | InvalidSource::RecordLimit) {
                assert!(matches!(result, Err(Error::TooLarge)));
            }
            assert!(
                !destination.exists(),
                "rejected {case:?} must not publish destination"
            );
            assert!(
                std::fs::read(&source).unwrap() == original,
                "rejected {case:?} must not modify source"
            );
        }
    }
}

#[test]
fn native_upgrade_refuses_existing_destination_and_never_creates_missing_source() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let destination = scratch.path("destination");
        assert!(
            upgrade(
                redb,
                &source,
                &destination,
                &[Node::descriptor()],
                BackupLimits::default()
            )
            .is_err()
        );
        assert!(!source.exists());
        assert!(!destination.exists());
        Legacy::new().write(redb, &source);
        let original = std::fs::read(&source).unwrap();
        assert!(matches!(
            upgrade(
                redb,
                &source,
                &source,
                &[Node::descriptor()],
                BackupLimits::default()
            ),
            Err(Error::Conflict)
        ));
        std::fs::write(&destination, b"existing destination").unwrap();
        assert!(matches!(
            upgrade(
                redb,
                &source,
                &destination,
                &[Node::descriptor()],
                BackupLimits::default()
            ),
            Err(Error::Conflict)
        ));
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"existing destination"
        );
        assert!(
            std::fs::read(&source).unwrap() == original,
            "upgrade must not modify source bytes"
        );
    }
}

#[test]
#[ignore = "actual child-process fixture used by native_upgrade_interruption_before_publication_is_retryable"]
fn upgrade_child_exits_before_publication() {
    let source = PathBuf::from(std::env::var_os("ROM_UPGRADE_SOURCE").unwrap());
    let destination = PathBuf::from(std::env::var_os("ROM_UPGRADE_DESTINATION").unwrap());
    let redb = std::env::var("ROM_UPGRADE_REDB").unwrap() == "true";
    let before_publish = || -> Result<()> { std::process::exit(86) };
    if redb {
        rom_redb::Redb::upgrade_from_observed(
            &source,
            &destination,
            &[Node::descriptor()],
            BackupLimits::default(),
            before_publish,
        )
        .unwrap();
    } else {
        rom_sqlite::Sqlite::upgrade_from_observed(
            &source,
            &destination,
            &[Node::descriptor()],
            BackupLimits::default(),
            before_publish,
        )
        .unwrap();
    }
    panic!("publication interruption checkpoint was not reached");
}

#[test]
fn native_upgrade_interruption_before_publication_is_retryable() {
    use std::os::unix::fs::PermissionsExt;
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let destination = scratch.path("destination");
        Legacy::new().write(redb, &source);
        let original = std::fs::read(&source).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "upgrade_child_exits_before_publication",
                "--ignored",
                "--nocapture",
            ])
            .env("ROM_UPGRADE_SOURCE", &source)
            .env("ROM_UPGRADE_DESTINATION", &destination)
            .env("ROM_UPGRADE_REDB", redb.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(
            std::fs::read(&source).unwrap() == original,
            "upgrade must not modify source bytes"
        );
        assert!(!destination.exists());
        let stages: Vec<_> = std::fs::read_dir(&scratch.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".rom-maintenance-")
            })
            .collect();
        assert!(
            !stages.is_empty(),
            "actual process exit leaves private unpublished staging data"
        );
        for stage in stages {
            assert_eq!(
                std::fs::metadata(&stage).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(stage.join("data"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let upgraded = upgrade(
            redb,
            &source,
            &destination,
            &[Node::descriptor()],
            BackupLimits::default(),
        )
        .unwrap();
        assert_eq!(upgraded.snapshot(Node::KIND, 10, 100_000).unwrap().len(), 2);
        assert_eq!(upgraded.commit(&remove_parent()), Err(Error::Conflict));
        assert!(
            std::fs::read(&source).unwrap() == original,
            "upgrade must not modify source bytes"
        );
    }
}

#[test]
fn ordinary_open_rejects_legacy_format_without_modifying_source() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        Legacy::new().write(redb, &source);
        let original = std::fs::read(&source).unwrap();
        let result = if redb {
            rom_redb::Redb::open(&source).map(|_| ())
        } else {
            rom_sqlite::Sqlite::open(&source).map(|_| ())
        };
        assert!(matches!(result, Err(Error::Unsupported(_))));
        assert!(
            std::fs::read(&source).unwrap() == original,
            "ordinary rejected open must not modify source; redb={redb}"
        );
    }
}

#[test]
#[ignore = "actual process fixture for a dirty legacy redb header"]
fn legacy_redb_child_exits_with_writable_handle() {
    let source = PathBuf::from(std::env::var_os("ROM_UPGRADE_SOURCE").unwrap());
    let _db = redb::Database::open(&source).unwrap();
    std::process::exit(86);
}

#[test]
fn ordinary_open_rejects_dirty_legacy_redb_without_repairing_source() {
    let scratch = Scratch::new();
    let source = scratch.path("source");
    Legacy::new().write(true, &source);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "legacy_redb_child_exits_with_writable_handle",
            "--ignored",
        ])
        .env("ROM_UPGRADE_SOURCE", &source)
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(86),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(
        redb::ReadOnlyDatabase::open(&source).is_err(),
        "fixture must require recovery"
    );
    let original = std::fs::read(&source).unwrap();
    assert!(matches!(
        rom_redb::Redb::open(&source),
        Err(Error::Unsupported(_))
    ));
    assert!(
        std::fs::read(&source).unwrap() == original,
        "rejected ordinary open must not repair dirty legacy source"
    );
    let destination = scratch.path("destination");
    let upgraded = rom_redb::Redb::upgrade_from(
        &source,
        &destination,
        &[Node::descriptor()],
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(upgraded.snapshot(Node::KIND, 10, 100_000).unwrap().len(), 2);
    assert_eq!(upgraded.commit(&remove_parent()), Err(Error::Conflict));
    assert!(
        std::fs::read(&source).unwrap() == original,
        "dirty source recovery must use a private copy"
    );
}

#[test]
fn sqlite_upgrade_reads_committed_wal_without_changing_source() {
    let scratch = Scratch::new();
    let source = scratch.path("source");
    let destination = scratch.path("destination");
    let mut fixture = Legacy::new();
    fixture.write(false, &source);
    let mut connection = rusqlite::Connection::open(&source).unwrap();
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let bundle = create("wal-only", Some("parent"));
    fixture.state.bundle(&bundle).unwrap();
    let tx = connection.transaction().unwrap();
    insert_sqlite_bundle(&tx, &bundle);
    tx.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(&fixture.state).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    let wal = scratch.path("source-wal");
    let original = std::fs::read(&source).unwrap();
    let original_wal = std::fs::read(&wal).unwrap();
    assert!(!original_wal.is_empty());
    let upgraded = rom_sqlite::Sqlite::upgrade_from(
        &source,
        &destination,
        &[Node::descriptor()],
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(
        upgraded.load(&bundle.receipt.row.key).unwrap(),
        Some(bundle.receipt.row.clone())
    );
    assert_eq!(
        upgraded
            .journal(Node::KIND, None, 10, 100_000)
            .unwrap()
            .events
            .len(),
        3
    );
    assert!(
        std::fs::read(&source).unwrap() == original,
        "upgrade must not checkpoint or modify the main file"
    );
    assert!(
        std::fs::read(&wal).unwrap() == original_wal,
        "upgrade must not modify the source WAL"
    );
    drop(connection);
}

#[test]
fn native_upgrade_fences_active_work_claims_without_losing_attempts() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let destination = scratch.path("destination");
        let mut fixture = Legacy::new();
        let WorkResult::Claimed(old) = fixture
            .state
            .work
            .apply(WorkUpdate::Claim { now: 0 })
            .unwrap()
        else {
            panic!("fixture must lease pending work")
        };
        fixture
            .state
            .work
            .apply(WorkUpdate::DeliveryStarted {
                claim: old.key(),
                now: 0,
            })
            .unwrap();
        let old_cursor = fixture.state.journal_head(Node::KIND);
        fixture.write(redb, &source);
        let original = std::fs::read(&source).unwrap();
        let upgraded = upgrade(
            redb,
            &source,
            &destination,
            &[Node::descriptor()],
            BackupLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            upgraded.journal(Node::KIND, Some(&old_cursor), 10, 100_000),
            Err(Error::HistoryGap)
        ));
        assert_eq!(
            upgraded.reaction_update(WorkUpdate::DeliveryFinished {
                claim: old.key(),
                now: 0,
                outcome: DeliveryOutcome::Accepted
            }),
            Err(Error::Conflict)
        );
        let WorkResult::Claimed(recovered) = upgraded
            .reaction_update(WorkUpdate::Claim { now: 0 })
            .unwrap()
        else {
            panic!("upgraded work must be recoverable")
        };
        assert_eq!(recovered.work.pending, old.work.pending);
        assert_eq!(recovered.work.attempts, old.work.attempts + 1);
        assert_eq!(recovered.work.delivery, Some(DeliveryOutcome::Unknown));
        assert!(recovered.work.generation > old.work.generation);
        assert!(
            std::fs::read(&source).unwrap() == original,
            "claim recovery applies only to the new database"
        );
    }
}
