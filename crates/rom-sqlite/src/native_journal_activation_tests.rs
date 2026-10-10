//! Public current-format activation and genuine predecessor witnesses.
use crate::native_work_test_support::{Scratch, bundle, descriptor};
use rom::{Resource, Storage, StorageLimits, WorkUpdate};
use rom_backup::{Backend, BackupLimits, Snapshot};

fn limits() -> BackupLimits {
    BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    }
}
fn assert_current(db: &crate::Sqlite) -> Snapshot {
    let c = db.connection.lock().unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
    crate::snapshot::validate_inventory(&c, 11).unwrap();
    let raw: String = c
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .unwrap();
    let _: rom::storage_support::metadata::MetadataHeaderParts =
        serde_json::from_str(&raw).unwrap();
    assert!(
        !serde_json::from_str::<rom::Value>(&raw)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("journal")
    );
    assert_eq!(db.validation_limits.max_bytes, limits().max_bytes);
    assert_eq!(db.validation_limits.max_records, limits().max_records);
    crate::snapshot::collect_native_snapshot(&c, limits(), true).unwrap()
}
fn compare_restore(db: &crate::Sqlite, mut expected: Snapshot) {
    let original_generation = serde_json::to_value(&expected.state).unwrap()["generation"].clone();
    let old_claims = expected
        .state
        .work
        .records()
        .into_iter()
        .filter_map(|record| {
            matches!(record.state, rom::WorkState::Leased { .. }).then_some(rom::ClaimKey {
                id: record.pending.id,
                generation: record.generation,
            })
        })
        .collect::<Vec<_>>();
    expected.state.prepare_restore().unwrap();
    let mut actual = serde_json::to_value(assert_current(db)).unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    assert_ne!(actual["state"]["generation"], original_generation);
    actual["state"]["generation"] = expected["state"]["generation"].clone();
    assert_eq!(actual, expected);
    for claim in old_claims {
        assert_eq!(
            db.reaction_update(WorkUpdate::DeliveryStarted { claim, now: 0 }),
            Err(rom::Error::Conflict)
        );
    }
}
fn genuine() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.superpowers/rom-010-format10-writer-SX103q/sqlite")
}
#[test]
fn public_fresh_store_has_complete_keyed_journal_inventory() {
    let scratch = Scratch::new();
    let path = scratch.path("fresh11");
    let db = crate::Sqlite::open_with_validation_limits(&path, StorageLimits::default(), limits())
        .unwrap();
    let empty = assert_current(&db);
    assert!(empty.rows.is_empty() && empty.events.is_empty() && empty.receipts.is_empty());
    assert!(
        empty.effects.is_empty() && empty.references.is_empty() && empty.descriptors.is_empty()
    );
    let mut empty_state = serde_json::to_value(&empty.state).unwrap();
    let wanted =
        serde_json::to_value(rom::StorageState::new(StorageLimits::default()).unwrap()).unwrap();
    empty_state["generation"] = wanted["generation"].clone();
    assert_eq!(empty_state, wanted);
    let c = db.connection.lock().unwrap();
    assert_eq!(
        c.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        c.pragma_query_value(None, "synchronous", |r| r.get::<_, u32>(0))
            .unwrap(),
        2
    );
    drop(c);
    db.register(&[descriptor()]).unwrap();
    db.commit(&bundle("public")).unwrap();
    assert_eq!(
        db.journal("native-work", None, 10, 100000)
            .unwrap()
            .events
            .len(),
        1
    );
    assert_eq!(db.work_snapshot(100, 1000000).unwrap().records.len(), 1);
    let before = serde_json::to_value(assert_current(&db)).unwrap();
    db.close().unwrap();
    let db = crate::Sqlite::open_with_validation_limits(&path, StorageLimits::default(), limits())
        .unwrap();
    assert_eq!(serde_json::to_value(assert_current(&db)).unwrap(), before);
}
#[test]
#[ignore = "Requires preserved genuine physical-format10 fixture"]
fn public_current_open_refuses_genuine_format10_without_changes() {
    let scratch = Scratch::new();
    let original = genuine().join("source");
    let bytes = std::fs::read(&original).unwrap();
    let copy = scratch.path("genuine10");
    std::fs::write(&copy, &bytes).unwrap();
    let (_, expected) =
        rom_backup::read(genuine().join("before.rombk"), Backend::Sqlite, limits()).unwrap();
    match crate::Sqlite::open_with_validation_limits(
        &copy,
        expected.state.storage_limits(),
        limits(),
    ) {
        Err(rom::Error::Unsupported(message)) => assert_eq!(message, "SQLite storage format"),
        Err(error) => panic!("expected physical-format refusal, got {error:?}"),
        Ok(_) => panic!("current writer accepted genuine format10"),
    }
    assert_eq!(std::fs::read(original).unwrap(), bytes);
    assert_eq!(std::fs::read(copy).unwrap(), bytes);
}
#[test]
#[ignore = "Requires preserved genuine physical-format10 fixture"]
fn public_upgrade_accepts_genuine_format10_into_fresh11() {
    let scratch = Scratch::new();
    let original = genuine().join("source");
    let bytes = std::fs::read(&original).unwrap();
    let copy = scratch.path("genuine10");
    std::fs::write(&copy, &bytes).unwrap();
    let (_, expected) =
        rom_backup::read(genuine().join("before.rombk"), Backend::Sqlite, limits()).unwrap();
    let db = crate::Sqlite::upgrade_from(
        &copy,
        scratch.path("upgraded11"),
        &expected.descriptors,
        limits(),
    )
    .unwrap();
    compare_restore(&db, expected);
    assert_eq!(std::fs::read(original).unwrap(), bytes);
    assert_eq!(std::fs::read(copy).unwrap(), bytes);
}
#[test]
#[ignore = "Requires preserved genuine physical-format10 archive"]
fn public_restore_from_accepted_archive_creates_physical11() {
    let scratch = Scratch::new();
    let archive = genuine().join("before.rombk");
    let (_, expected) = rom_backup::read(&archive, Backend::Sqlite, limits()).unwrap();
    let db = crate::Sqlite::restore_from(&archive, scratch.path("restored11"), limits()).unwrap();
    compare_restore(&db, expected);
}
#[test]
fn public_current_backup_exports_archive7_storage11() {
    let scratch = Scratch::new();
    let db = crate::Sqlite::open_with_validation_limits(
        scratch.path("current"),
        StorageLimits::default(),
        limits(),
    )
    .unwrap();
    db.register(&[descriptor()]).unwrap();
    db.commit(&bundle("archive")).unwrap();
    let archive = scratch.path("current.rombk");
    let manifest = db.backup_to(&archive, limits()).unwrap();
    let value = serde_json::to_value(manifest).unwrap();
    assert_eq!(value["archive_version"], 7);
    assert_eq!(value["storage_format"], 11);
    let (_, expected) = rom_backup::read(&archive, Backend::Sqlite, limits()).unwrap();
    compare_restore(
        &crate::Sqlite::restore_from(&archive, scratch.path("roundtrip"), limits()).unwrap(),
        expected,
    );
}

#[derive(Clone, Resource)]
#[resource(name = "native-work")]
struct Before {
    amount: u64,
}
#[derive(Clone, Resource)]
#[resource(name = "native-work", version = 2)]
struct After {
    amount: u64,
}
#[test]
fn public_current_maintenance_preserves_canonical_oracle_and_fences_claims() {
    let scratch = Scratch::new();
    let path = scratch.path("source");
    let db = crate::Sqlite::open_with_validation_limits(&path, StorageLimits::default(), limits())
        .unwrap();
    db.register(&[descriptor()]).unwrap();
    db.commit(&bundle("maintenance-a")).unwrap();
    db.commit(&bundle("maintenance-b")).unwrap();
    let old_claim = crate::native_work_test_support::claim(&db).key();
    let expected = assert_current(&db);
    db.close().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let policy = rom_backup::RetentionPolicy::new(expected.state.retry_epochs()).journal_through(1);
    let copy =
        || serde_json::from_value::<Snapshot>(serde_json::to_value(&expected).unwrap()).unwrap();
    let (retained, report) = rom_backup::retain_snapshot(copy(), &policy, limits()).unwrap();
    let (db, actual_report) =
        crate::Sqlite::retain_from(&path, scratch.path("retained"), &policy, limits()).unwrap();
    assert_eq!(actual_report, report);
    compare_restore(&db, retained);
    assert_eq!(
        db.reaction_update(WorkUpdate::DeliveryStarted {
            claim: old_claim.clone(),
            now: 0
        }),
        Err(rom::Error::Conflict)
    );
    let plan = rom_backup::MigrationPlan::new(vec![
        rom_backup::ResourceMigration::new::<Before, After>(|v| Ok(After { amount: v.amount }))
            .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let migrated = rom_backup::migrate_snapshot(copy(), &plan, limits()).unwrap();
    let db = crate::Sqlite::migrate_from(&path, scratch.path("migrated"), &plan, limits()).unwrap();
    compare_restore(&db, migrated);
    assert_eq!(
        db.reaction_update(WorkUpdate::DeliveryStarted {
            claim: old_claim.clone(),
            now: 0
        }),
        Err(rom::Error::Conflict)
    );
    let db = crate::Sqlite::rebuild_indexes_from(&path, scratch.path("rebuilt"), limits()).unwrap();
    compare_restore(&db, expected);
    assert_eq!(
        db.reaction_update(WorkUpdate::DeliveryStarted {
            claim: old_claim,
            now: 0
        }),
        Err(rom::Error::Conflict)
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let destination = scratch.path("existing");
    std::fs::write(&destination, b"sentinel").unwrap();
    assert!(matches!(
        crate::Sqlite::retain_from(&path, &destination, &policy, limits()),
        Err(rom::Error::Conflict)
    ));
    assert!(matches!(
        crate::Sqlite::migrate_from(&path, &destination, &plan, limits()),
        Err(rom::Error::Conflict)
    ));
    assert!(matches!(
        crate::Sqlite::rebuild_indexes_from(&path, &destination, limits()),
        Err(rom::Error::Conflict)
    ));
    assert_eq!(std::fs::read(&destination).unwrap(), b"sentinel");
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
