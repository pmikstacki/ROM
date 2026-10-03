use rom::*;
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "migration-nodes")]
struct Before {
    label: String,
    quantity: String,
    target: Option<ResourceRef<Before>>,
}
#[derive(Clone, Resource)]
#[resource(name = "migration-nodes", version = 2)]
struct After {
    title: String,
    quantity: u64,
    target: Option<ResourceRef<After>>,
    enabled: bool,
}
fn convert(old: Before) -> Result<After> {
    Ok(After {
        title: old.label,
        quantity: old.quantity.parse().map_err(|_| Error::Storage)?,
        target: old
            .target
            .map(|reference| ResourceRef::new(reference.id()))
            .transpose()?,
        enabled: true,
    })
}
fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(convert).unwrap(),
    ])
    .unwrap()
    .validate_work(|work| {
        if work.definition != "notify" || work.version != 1 {
            return Err(Error::Unsupported("notification contract".into()));
        }
        let WorkPayload::Notification { source, payload } = &work.payload else {
            return Err(Error::Storage);
        };
        After::decode(source.value.clone().ok_or(Error::Storage)?)?;
        if payload != &json!({"channel":"audit"}) {
            return Err(Error::Storage);
        }
        Ok(())
    })
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-schema-migration-{}-{}",
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
enum Db {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
}
impl Db {
    fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
    fn storage(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(db) => db,
            Self::Redb(db) => db,
        }
    }
    fn migrate(
        redb: bool,
        source: &Path,
        destination: &Path,
        plan: &MigrationPlan,
        limits: BackupLimits,
    ) -> Result<Self> {
        if redb {
            rom_redb::Redb::migrate_from(source, destination, plan, limits).map(Self::Redb)
        } else {
            rom_sqlite::Sqlite::migrate_from(source, destination, plan, limits).map(Self::Sqlite)
        }
    }
    fn backup(&self, path: &Path) {
        match self {
            Self::Sqlite(db) => db.backup_to(path, BackupLimits::default()),
            Self::Redb(db) => db.backup_to(path, BackupLimits::default()),
        }
        .unwrap();
    }
    fn restore(redb: bool, archive: &Path, destination: &Path) -> Self {
        if redb {
            Self::Redb(
                rom_redb::Redb::restore_from(archive, destination, BackupLimits::default())
                    .unwrap(),
            )
        } else {
            Self::Sqlite(
                rom_sqlite::Sqlite::restore_from(archive, destination, BackupLimits::default())
                    .unwrap(),
            )
        }
    }
}
fn bundle(id: &str, target: Option<&str>, quantity: &str) -> Bundle {
    Bundle {
        expected: None,
        receipt: Receipt {
            replay_version: None,
            identity: format!("create-{id}"),
            fingerprint: format!("before-{id}"),
            row: Row {
                key: Key {
                    kind: Before::KIND.into(),
                    id: id.into(),
                },
                revision: 1,
                value: Some(
                    Before {
                        label: id.into(),
                        quantity: quantity.into(),
                        target: target.map(|id| ResourceRef::new(id).unwrap()),
                    }
                    .encode(),
                ),
                protected: ProtectedMetadata {
                    deletion_authorization: None,
                    source_provenance: Some(SourceProvenance {
                        source: "fixture".into(),
                        version: "v1".into(),
                        generation: 1,
                        field_origins: [("label".into(), "fixture".into())].into(),
                    }),
                },
            },
        },
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
struct Seed {
    child: Bundle,
    old_claim: ClaimKey,
    old_cursor: JournalCursor,
}
fn seed(redb: bool, path: &Path, quantity: &str) -> Seed {
    let db = Db::open(redb, path);
    let storage = db.storage();
    storage.register(&[Before::descriptor()]).unwrap();
    storage.commit(&bundle("parent", None, "1")).unwrap();
    let mut child = bundle("child", Some("parent"), quantity);
    child
        .effects
        .push(Intent::new("audit", json!({"message":"unchanged effect"})));
    child.reaction_limits = Some(ReactionLimits::default());
    child.reactions.push(PendingWork {
        id: "notify-child".into(),
        definition: "notify".into(),
        version: 1,
        service_key: "service".into(),
        cause: Cause {
            root: "chain".into(),
            parent: None,
            depth: 1,
            started_at: 0,
            path: vec!["notify-child".into()],
        },
        payload: WorkPayload::Notification {
            source: child.receipt.row.clone(),
            payload: json!({"channel":"audit"}),
        },
    });
    storage.commit(&child).unwrap();
    let mut deleted = bundle("deleted", None, "3");
    deleted.receipt.row.protected.deletion_authorization = deleted.receipt.row.value.take();
    storage.commit(&deleted).unwrap();
    let WorkResult::Claimed(claim) = storage
        .reaction_update(WorkUpdate::Claim { now: 0 })
        .unwrap()
    else {
        panic!("claim required")
    };
    storage
        .reaction_update(WorkUpdate::DeliveryStarted {
            claim: claim.key(),
            now: 0,
        })
        .unwrap();
    Seed {
        child,
        old_claim: claim.key(),
        old_cursor: storage.journal_head(Before::KIND).unwrap(),
    }
}
fn remove_parent() -> Bundle {
    let mut b = bundle("parent", None, "1");
    b.expected = Some(1);
    b.receipt.identity = "delete-parent".into();
    b.receipt.row.revision = 2;
    b.receipt.row.value = None;
    b
}
fn assert_after(db: &Db, seed: &Seed) {
    let storage = db.storage();
    storage.register(&[After::descriptor()]).unwrap();
    assert!(storage.register(&[Before::descriptor()]).is_err());
    let child = storage.load(&seed.child.receipt.row.key).unwrap().unwrap();
    let value = After::decode(child.value.clone().unwrap()).unwrap();
    assert_eq!(
        (value.title.as_str(), value.quantity, value.enabled),
        ("child", 7, true)
    );
    assert_eq!(child.revision, 1);
    assert_eq!(child.protected, seed.child.receipt.row.protected);
    let receipt = storage
        .receipt(&seed.child.receipt.identity)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.row, child);
    assert_eq!(receipt.replay_version, Some(1));
    assert_eq!(receipt.fingerprint, seed.child.receipt.fingerprint);
    assert_eq!(
        storage.commit(&seed.child).unwrap(),
        receipt,
        "native replay returns migrated receipt without reapplying"
    );
    assert!(matches!(
        storage.journal(Before::KIND, Some(&seed.old_cursor), 10, 100_000),
        Err(Error::HistoryGap)
    ));
    let journal = storage.journal(Before::KIND, None, 10, 100_000).unwrap();
    assert_eq!(journal.events.len(), 3);
    for event in journal.events {
        if let Some(value) = event.row.value {
            After::decode(value).unwrap();
        }
    }
    let tomb = storage
        .load(&Key {
            kind: Before::KIND.into(),
            id: "deleted".into(),
        })
        .unwrap()
        .unwrap();
    assert!(tomb.value.is_none());
    assert_eq!(
        After::decode(tomb.protected.deletion_authorization.unwrap())
            .unwrap()
            .quantity,
        3
    );
    assert_eq!(storage.commit(&remove_parent()), Err(Error::Conflict));
    assert_eq!(
        storage.reaction_update(WorkUpdate::DeliveryFinished {
            claim: seed.old_claim.clone(),
            now: 0,
            outcome: DeliveryOutcome::Accepted
        }),
        Err(Error::Conflict)
    );
    let work = storage.reaction_records().unwrap();
    assert_eq!(work[0].attempts, 1);
    assert_eq!(work[0].state, WorkState::Pending);
    assert_eq!(work[0].delivery, Some(DeliveryOutcome::Unknown));
    let WorkPayload::Notification { source, payload } = &work[0].pending.payload else {
        panic!("notification")
    };
    assert_eq!(
        After::decode(source.value.clone().unwrap())
            .unwrap()
            .quantity,
        7
    );
    assert_eq!(payload, &json!({"channel":"audit"}));
}
#[test]
fn native_schema_migration_preserves_history_references_work_and_backup() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let dest = dir.path("destination");
        let seed = seed(redb, &source, "7");
        let original = std::fs::read(&source).unwrap();
        let db = Db::migrate(redb, &source, &dest, &plan(), BackupLimits::default()).unwrap();
        assert_after(&db, &seed);
        let archive = dir.path("archive");
        db.backup(&archive);
        let backend = if redb {
            rom_backup::Backend::Redb
        } else {
            rom_backup::Backend::Sqlite
        };
        let (_, snapshot) = rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
        assert_eq!(snapshot.effects.len(), 1);
        assert_eq!(snapshot.effects[0].intent, seed.child.effects[0]);
        assert_eq!(snapshot.effects[0].identity, seed.child.receipt.identity);
        drop(db);
        let restored = Db::restore(redb, &archive, &dir.path("restored"));
        assert_after(&restored, &seed);
        assert!(
            std::fs::read(&source).unwrap() == original,
            "source bytes unchanged"
        );
    }
}
#[test]
fn rejected_migration_never_publishes_or_modifies_source() {
    for redb in [false, true] {
        for case in [
            "conversion",
            "panic",
            "work",
            "dangling",
            "limit",
            "destination",
        ] {
            let dir = Scratch::new();
            let source = dir.path("source");
            let dest = dir.path("destination");
            seed(
                redb,
                &source,
                if case == "conversion" {
                    "not a number"
                } else {
                    "7"
                },
            );
            let original = std::fs::read(&source).unwrap();
            let selected = match case {
                "dangling" => MigrationPlan::new(vec![
                    ResourceMigration::new::<Before, After>(|old| {
                        let mut after = convert(old)?;
                        after.target = Some(ResourceRef::new("absent")?);
                        Ok(after)
                    })
                    .unwrap(),
                ])
                .unwrap()
                .validate_work(|_| Ok(())),
                "panic" => MigrationPlan::new(vec![
                    ResourceMigration::new::<Before, After>(|_| panic!("converter failed"))
                        .unwrap(),
                ])
                .unwrap(),
                "work" => MigrationPlan::new(vec![
                    ResourceMigration::new::<Before, After>(convert).unwrap(),
                ])
                .unwrap(),
                _ => plan(),
            };
            let mut limits = BackupLimits::default();
            if case == "limit" {
                limits.max_bytes = 1;
            }
            if case == "destination" {
                std::fs::write(&dest, b"existing").unwrap();
            }
            let result = Db::migrate(redb, &source, &dest, &selected, limits);
            assert!(result.is_err(), "case {case}");
            if case == "panic" {
                assert!(matches!(result, Err(Error::Panicked)));
            }
            if case == "destination" {
                assert_eq!(std::fs::read(&dest).unwrap(), b"existing");
            } else {
                assert!(!dest.exists());
            }
            assert!(
                std::fs::read(&source).unwrap() == original,
                "source unchanged after {case}"
            );
        }
    }
}
#[test]
#[ignore = "subprocess fixture invoked by native_schema_migration_interruption_is_retryable"]
fn migration_child_exits_before_publication() {
    let source = PathBuf::from(std::env::var_os("ROM_MIGRATE_SOURCE").unwrap());
    let dest = PathBuf::from(std::env::var_os("ROM_MIGRATE_DEST").unwrap());
    let before = || -> Result<()> { std::process::exit(86) };
    if std::env::var("ROM_MIGRATE_REDB").unwrap() == "true" {
        rom_redb::Redb::migrate_from_observed(
            &source,
            &dest,
            &plan(),
            BackupLimits::default(),
            before,
        )
        .unwrap();
    } else {
        rom_sqlite::Sqlite::migrate_from_observed(
            &source,
            &dest,
            &plan(),
            BackupLimits::default(),
            before,
        )
        .unwrap();
    }
    panic!("interruption checkpoint not reached");
}
#[test]
fn native_schema_migration_interruption_is_retryable() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let dest = dir.path("destination");
        let seed = seed(redb, &source, "7");
        let original = std::fs::read(&source).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "migration_child_exits_before_publication",
                "--ignored",
            ])
            .env("ROM_MIGRATE_SOURCE", &source)
            .env("ROM_MIGRATE_DEST", &dest)
            .env("ROM_MIGRATE_REDB", redb.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(!dest.exists());
        assert!(std::fs::read(&source).unwrap() == original);
        let db = Db::migrate(redb, &source, &dest, &plan(), BackupLimits::default()).unwrap();
        assert_after(&db, &seed);
    }
}

fn mark_format_four(redb: bool, path: &Path, erase_catalog: bool) {
    if redb {
        let db = redb::Database::open(path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<&str, u64>::new("rom_metadata"))
            .unwrap()
            .insert("format", 4)
            .unwrap();
        if erase_catalog {
            tx.open_table(redb::TableDefinition::<&str, &str>::new("rom_schemas"))
                .unwrap()
                .retain(|_, _| false)
                .unwrap();
        }
        tx.commit().unwrap();
    } else {
        let c = rusqlite::Connection::open(path).unwrap();
        c.pragma_update(None, "user_version", 4).unwrap();
        if erase_catalog {
            c.execute("DELETE FROM schemas", []).unwrap();
        }
    }
}
fn upgrade_four(
    redb: bool,
    source: &Path,
    destination: &Path,
    descriptors: &[Descriptor],
) -> Result<Db> {
    if redb {
        rom_redb::Redb::upgrade_from(source, destination, descriptors, BackupLimits::default())
            .map(Db::Redb)
    } else {
        rom_sqlite::Sqlite::upgrade_from(source, destination, descriptors, BackupLimits::default())
            .map(Db::Sqlite)
    }
}
#[test]
fn format_four_requires_explicit_upgrade_or_migration_without_source_writes() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let fixture = seed(redb, &source, "7");
        mark_format_four(redb, &source, false);
        let original = std::fs::read(&source).unwrap();
        let ordinary = if redb {
            rom_redb::Redb::open(&source).map(|_| ())
        } else {
            rom_sqlite::Sqlite::open(&source).map(|_| ())
        };
        assert!(matches!(ordinary, Err(Error::Unsupported(_))));
        assert!(std::fs::read(&source).unwrap() == original);
        let upgraded = upgrade_four(
            redb,
            &source,
            &dir.path("upgraded"),
            &[Before::descriptor()],
        )
        .unwrap();
        let receipt = upgraded
            .storage()
            .receipt(&fixture.child.receipt.identity)
            .unwrap()
            .unwrap();
        assert_eq!(receipt.replay_version, Some(1));
        assert_eq!(receipt.row, fixture.child.receipt.row);
        assert_eq!(
            upgraded.storage().commit(&remove_parent()),
            Err(Error::Conflict)
        );
        drop(upgraded);
        let migrated = Db::migrate(
            redb,
            &source,
            &dir.path("migrated"),
            &plan(),
            BackupLimits::default(),
        )
        .unwrap();
        assert_after(&migrated, &fixture);
        assert!(std::fs::read(&source).unwrap() == original);
    }
}
#[test]
fn format_four_upgrade_requires_exact_valid_catalog() {
    for redb in [false, true] {
        for empty in [false, true] {
            let dir = Scratch::new();
            let source = dir.path("source");
            seed(redb, &source, "7");
            mark_format_four(redb, &source, empty);
            let original = std::fs::read(&source).unwrap();
            let descriptors = if empty {
                vec![Before::descriptor()]
            } else {
                vec![After::descriptor()]
            };
            assert!(upgrade_four(redb, &source, &dir.path("destination"), &descriptors).is_err());
            assert!(!dir.path("destination").exists());
            assert!(std::fs::read(&source).unwrap() == original);
        }
    }
}
