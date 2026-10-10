use super::*;
use redb::ReadableTable;
use rom_backup::{MigrationPlan, ResourceMigration, RetentionPolicy};

#[derive(Clone, Resource)]
#[resource(name = "journal-items", version = 2)]
struct After {
    title: String,
}
fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Item, After>(|before| {
            Ok(After {
                title: format!("{}!", before.title),
            })
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()))
}
fn populated() -> (Redb, std::path::PathBuf) {
    let (mut storage, path) = candidate(8);
    storage.validation_limits = BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    };
    storage
        .register(&[Item::descriptor(), Link::descriptor()])
        .unwrap();
    storage.commit(&bundle_with_work("one")).unwrap();
    storage.commit(&bundle("two")).unwrap();
    let mut link = bundle("link");
    link.receipt.row.key.kind = "journal-links".into();
    link.receipt.row.value = Some(rom::json!({"target":"one"}));
    storage.commit(&link).unwrap();
    assert!(matches!(
        storage.reaction_update(rom::WorkUpdate::Claim { now: 0 }),
        Ok(rom::WorkResult::Claimed(_))
    ));
    (storage, path)
}
fn snapshot(storage: &Redb) -> rom_backup::Snapshot {
    crate::maintenance::snapshot_in_format(
        &storage.db.begin_read().unwrap(),
        storage.validation_limits,
        crate::maintenance::NativeFormat::Exact(11),
    )
    .unwrap()
}

#[test]
fn candidate_migration_rebuilds_journal_and_preserves_source() {
    let (storage, path) = populated();
    let limits = storage.validation_limits;
    let source_generation = storage.journal_head("journal-items").unwrap().generation;
    let mut expected = rom_backup::migrate_snapshot(snapshot(&storage), &plan(), limits).unwrap();
    expected.state.prepare_restore().unwrap();
    let expected_limits = expected.state.storage_limits();
    drop(storage);
    let source = path.join("db");
    let before = std::fs::read(&source).unwrap();
    let migrated = Redb::migrate_from(&source, path.join("migrated"), &plan(), limits)
        .expect("candidate source must use bounded migration branch");
    assert_fenced_conversion(
        canonical(&migrated),
        serde_json::to_value(expected).unwrap(),
        &source_generation,
    );
    assert_eq!(migrated.native_format, 11);
    assert_eq!(migrated.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(migrated.validation_limits.max_records, limits.max_records);
    snapshot(&migrated)
        .state
        .check_limits(&expected_limits)
        .unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), before);
    drop(migrated);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn candidate_retention_rebuilds_journal_and_preserves_source() {
    let (storage, path) = populated();
    let limits = storage.validation_limits;
    let source_generation = storage.journal_head("journal-items").unwrap().generation;
    let policy = RetentionPolicy::new(rom::RetryEpochs {
        current: 1,
        admission_floor: 1,
        replay_floor: 0,
    })
    .journal_through(1);
    let (mut expected, expected_report) =
        rom_backup::retain_snapshot(snapshot(&storage), &policy, limits).unwrap();
    expected.state.prepare_restore().unwrap();
    let expected_limits = expected.state.storage_limits();
    drop(storage);
    let source = path.join("db");
    let before = std::fs::read(&source).unwrap();
    let (retained, report) = Redb::retain_from(&source, path.join("retained"), &policy, limits)
        .expect("candidate source must use bounded retention branch");
    assert_eq!(report, expected_report);
    assert_eq!(report.events_removed, 1);
    assert_fenced_conversion(
        canonical(&retained),
        serde_json::to_value(expected).unwrap(),
        &source_generation,
    );
    assert_eq!(retained.native_format, 11);
    assert_eq!(retained.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(retained.validation_limits.max_records, limits.max_records);
    snapshot(&retained)
        .state
        .check_limits(&expected_limits)
        .unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), before);
    drop(retained);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn candidate_maintenance_refuses_existing_destinations_and_interrupted_publication() {
    let (storage, path) = populated();
    let limits = storage.validation_limits;
    drop(storage);
    let source = path.join("db");
    let before = std::fs::read(&source).unwrap();
    let policy = RetentionPolicy::new(rom::RetryEpochs::default()).journal_through(1);
    let existing = path.join("existing");
    std::fs::write(&existing, b"preserved").unwrap();
    assert!(Redb::migrate_from(&source, &existing, &plan(), limits).is_err());
    assert!(Redb::retain_from(&source, &existing, &policy, limits).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), b"preserved");
    let migration = path.join("migration-interrupted");
    assert!(matches!(
        Redb::migrate_from_observed(&source, &migration, &plan(), limits, || Err(Error::Storage)),
        Err(Error::Storage)
    ));
    assert!(!migration.exists());
    let retention = path.join("retention-interrupted");
    assert!(matches!(
        Redb::retain_from_observed(&source, &retention, &policy, limits, || Err(Error::Storage)),
        Err(Error::Storage)
    ));
    assert!(!retention.exists());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn candidate_maintenance_admits_aggregate_native_raw_bytes_before_callbacks() {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let guarded = MigrationPlan::new(vec![
        ResourceMigration::new::<Item, After>(|before| {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(After {
                title: before.title,
            })
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let (storage, path) = populated();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::POSITIONS)
        .unwrap()
        .insert(1, "{".repeat(32768).as_str())
        .unwrap();
    tx.commit().unwrap();
    drop(storage);
    let source = path.join("db");
    let before = std::fs::read(&source).unwrap();
    let limits = BackupLimits {
        max_bytes: 16384,
        max_records: 4096,
    };
    let migration = path.join("migration-rejected");
    assert!(matches!(
        Redb::migrate_from(&source, &migration, &guarded, limits),
        Err(Error::TooLarge)
    ));
    assert_eq!(CALLS.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(!migration.exists());
    let retention = path.join("retention-rejected");
    let policy = RetentionPolicy::new(rom::RetryEpochs::default());
    assert!(matches!(
        Redb::retain_from(&source, &retention, &policy, limits),
        Err(Error::TooLarge)
    ));
    assert!(!retention.exists());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn candidate_maintenance_rejects_individually_fitting_native_values_in_aggregate() {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let guarded = MigrationPlan::new(vec![
        ResourceMigration::new::<Item, After>(|before| {
            CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(After {
                title: before.title,
            })
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let (storage, path) = populated();
    let tx = storage.db.begin_write().unwrap();
    {
        let mut positions = tx.open_table(crate::format::POSITIONS).unwrap();
        for position in [1, 2] {
            let raw = positions.get(position).unwrap().unwrap().value().to_owned();
            assert!(raw.len() < 6144);
            // Trailing JSON whitespace preserves a valid exact identity/byte fact.
            let padded = format!("{raw}{}", " ".repeat(6144 - raw.len()));
            assert_eq!(padded.len(), 6144);
            assert!(padded.len() + 8 < 8192);
            positions.insert(position, padded.as_str()).unwrap();
        }
    }
    tx.commit().unwrap();
    // The modified source is still a fully valid native image under its profile.
    snapshot(&storage).validate().unwrap();
    drop(storage);
    let source = path.join("db");
    let before = std::fs::read(&source).unwrap();
    let limits = BackupLimits {
        max_bytes: 8192,
        max_records: 4096,
    };
    let migration = path.join("aggregate-migration-rejected");
    assert!(matches!(
        Redb::migrate_from(&source, &migration, &guarded, limits),
        Err(Error::TooLarge)
    ));
    assert_eq!(CALLS.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(!migration.exists());
    let retention = path.join("aggregate-retention-rejected");
    assert!(matches!(
        Redb::retain_from(
            &source,
            &retention,
            &RetentionPolicy::new(rom::RetryEpochs::default()),
            limits
        ),
        Err(Error::TooLarge)
    ));
    assert!(!retention.exists());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
#[ignore = "explicit >128MiB physical-budget gate; root must admit memory/disk envelope"]
fn candidate_valid_inventory_above_default_cap_preserves_custom_profile() {
    static CONVERTERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    static PUBLISHES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    assert_eq!(
        std::env::var("ROM_REDB_LARGE_NATIVE_BUDGET").as_deref(),
        Ok("1")
    );
    let (mut storage, path) = populated();
    let limits = BackupLimits {
        max_bytes: 192 * 1024 * 1024,
        max_records: 4096,
    };
    storage.validation_limits = limits;
    let storage_limits = snapshot(&storage).state.storage_limits();
    let expected = snapshot(&storage);
    let source_generation = expected.state.journal_head("journal-items").generation;
    {
        let tx = storage.db.begin_write().unwrap();
        {
            let mut state = tx.open_table(crate::format::STATE).unwrap();
            let mut raw = state.get("metadata").unwrap().unwrap().value().to_owned();
            // Valid JSON whitespace enlarges actual physical inventory without
            // multiplying large canonical payloads during maintenance collection.
            raw.extend(std::iter::repeat_n(' ', 129 * 1024 * 1024 - raw.len()));
            assert!(raw.len() > BackupLimits::default().max_bytes);
            state.insert("metadata", raw.as_str()).unwrap();
        }
        tx.commit().unwrap();
    }
    assert_eq!(
        serde_json::to_value(snapshot(&storage)).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    drop(storage);
    let source = path.join("db");
    // redb rounds a 129MiB leaf to a 256MiB allocation and grows its
    // logical backing beyond 512MiB; the admitted image remains finitely bounded.
    assert!(std::fs::metadata(&source).unwrap().len() < 1024 * 1024 * 1024);
    let before = super::file_witness::capture(&source).unwrap();
    assert!(matches!(
        Redb::open_with_validation_limits(&source, storage_limits.clone(), BackupLimits::default()),
        Err(Error::TooLarge)
    ));
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    let guarded = MigrationPlan::new(vec![
        ResourceMigration::new::<Item, After>(|before| {
            CONVERTERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(After {
                title: before.title,
            })
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let default_migration = path.join("large-default-migration-rejected");
    assert!(matches!(
        Redb::migrate_from_observed(
            &source,
            &default_migration,
            &guarded,
            BackupLimits::default(),
            || {
                PUBLISHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        ),
        Err(Error::TooLarge)
    ));
    assert_eq!(CONVERTERS.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(PUBLISHES.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(!default_migration.exists());
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    let policy = RetentionPolicy::new(rom::RetryEpochs::default()).journal_through(1);
    let default_retention = path.join("large-default-retention-rejected");
    assert!(matches!(
        Redb::retain_from_observed(
            &source,
            &default_retention,
            &policy,
            BackupLimits::default(),
            || {
                PUBLISHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }
        ),
        Err(Error::TooLarge)
    ));
    assert_eq!(CONVERTERS.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(PUBLISHES.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(!default_retention.exists());
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    let reopen_source = path.join("large-custom-reopen-copy");
    std::fs::copy(&source, &reopen_source).unwrap();
    assert!(std::fs::metadata(&reopen_source).unwrap().len() < 1024 * 1024 * 1024);
    assert_eq!(
        super::file_witness::capture(&reopen_source).unwrap(),
        before
    );
    let reopened =
        Redb::open_with_validation_limits(&reopen_source, storage_limits.clone(), limits).unwrap();
    assert_eq!(
        serde_json::to_value(snapshot(&reopened)).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    assert_eq!(reopened.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(reopened.validation_limits.max_records, limits.max_records);
    snapshot(&reopened)
        .state
        .check_limits(&storage_limits)
        .unwrap();
    drop(reopened);
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    let migration_input = serde_json::from_value(serde_json::to_value(&expected).unwrap()).unwrap();
    let mut expected_migration =
        rom_backup::migrate_snapshot(migration_input, &plan(), limits).unwrap();
    expected_migration.state.prepare_restore().unwrap();
    let migrated_limits = expected_migration.state.storage_limits();
    let migrated =
        Redb::migrate_from(&source, path.join("large-migrated"), &plan(), limits).unwrap();
    assert_eq!(migrated.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(migrated.validation_limits.max_records, limits.max_records);
    assert_fenced_conversion(
        canonical(&migrated),
        serde_json::to_value(expected_migration).unwrap(),
        &source_generation,
    );
    snapshot(&migrated)
        .state
        .check_limits(&migrated_limits)
        .unwrap();
    drop(migrated);
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    let (mut expected_retention, expected_report) =
        rom_backup::retain_snapshot(expected, &policy, limits).unwrap();
    expected_retention.state.prepare_restore().unwrap();
    let retained_limits = expected_retention.state.storage_limits();
    let (retained, report) =
        Redb::retain_from(&source, path.join("large-retained"), &policy, limits).unwrap();
    assert_eq!(retained.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(retained.validation_limits.max_records, limits.max_records);
    assert_eq!(report, expected_report);
    assert_fenced_conversion(
        canonical(&retained),
        serde_json::to_value(expected_retention).unwrap(),
        &source_generation,
    );
    snapshot(&retained)
        .state
        .check_limits(&retained_limits)
        .unwrap();
    drop(retained);
    assert_eq!(super::file_witness::capture(&source).unwrap(), before);
    // Keep the real backing images for the explicitly admitted fixture audit.
    eprintln!(
        "retained large redb fixture: {}",
        path.file_name().unwrap().to_string_lossy()
    );
}

#[test]
fn rejected_native_startup_preserves_unused_backing_and_source_bytes() {
    for native_format in [10, 11] {
        let path = directory("rejected-startup-preservation");
        let source = path.join("db");
        let limits = StorageLimits::default();
        let open = |validation| {
            Redb::open_owned_in_format(
                rom_backup::NativeOwnership::acquire(
                    &source,
                    rom_backup::NativeAccess::OpenOrCreate,
                )
                .unwrap(),
                limits.clone(),
                validation,
                native_format,
            )
        };
        drop(open(BackupLimits::default()).unwrap());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&source)
            .unwrap();
        file.set_len(file.metadata().unwrap().len() + 1024 * 1024)
            .unwrap();
        drop(file);
        // Extra unused backing is a legal source, not a malformed table fixture.
        crate::maintenance::read_snapshot(
            &source,
            BackupLimits::default(),
            crate::maintenance::NativeFormat::Exact(native_format),
        )
        .unwrap()
        .validate()
        .unwrap();
        let before = super::file_witness::capture(&source).unwrap();
        assert!(matches!(
            open(BackupLimits {
                max_bytes: 1,
                max_records: 4096
            }),
            Err(Error::TooLarge)
        ));
        assert_eq!(super::file_witness::capture(&source).unwrap(), before);
        std::fs::remove_dir_all(path).unwrap();
    }
}
