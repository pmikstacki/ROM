//! Real candidate maintenance compared with the canonical maintenance oracle.
use crate::native_work_test_support::{Scratch, bundle, descriptor};
use rom::{ClaimKey, Resource, Storage, WorkUpdate};
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration, RetentionPolicy, Snapshot};
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
fn limits() -> BackupLimits {
    BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    }
}
fn source() -> (Scratch, std::path::PathBuf, Snapshot, Vec<u8>, ClaimKey) {
    let scratch = Scratch::new();
    let path = scratch.path("candidate-source");
    let db =
        crate::Sqlite::open_with_validation_limits(&path, rom::StorageLimits::default(), limits())
            .unwrap();
    db.register(&[descriptor()]).unwrap();
    db.commit(&bundle("maintenance-a")).unwrap();
    db.commit(&bundle("maintenance-b")).unwrap();
    let old_claim = crate::native_work_test_support::claim(&db).key();
    let snapshot =
        crate::snapshot::collect_native_snapshot(&db.connection.lock().unwrap(), limits(), true)
            .unwrap();
    db.close().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    (scratch, path, snapshot, bytes, old_claim)
}
fn compare(db: &crate::Sqlite, expected: Snapshot) {
    compare_profile(db, expected, limits());
}
fn compare_profile(db: &crate::Sqlite, mut expected: Snapshot, profile: BackupLimits) {
    let original_generation = serde_json::to_value(&expected.state).unwrap()["generation"].clone();
    expected.state.prepare_restore().unwrap();
    let actual =
        crate::snapshot::collect_native_snapshot(&db.connection.lock().unwrap(), profile, true)
            .unwrap();
    let mut actual = serde_json::to_value(actual).unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    assert_ne!(actual["state"]["generation"], original_generation);
    assert_eq!(db.validation_limits.max_bytes, profile.max_bytes);
    assert_eq!(db.validation_limits.max_records, profile.max_records);
    actual["state"]["generation"] = expected["state"]["generation"].clone();
    assert_eq!(actual, expected);
    assert_eq!(
        db.connection
            .lock()
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        11
    );
}
#[test]
fn candidate_offline_retention_rebuilds_native_positions() {
    let (scratch, path, snapshot, bytes, old_claim) = source();
    let limits = limits();
    let policy = RetentionPolicy::new(snapshot.state.retry_epochs()).journal_through(1);
    let (expected, report) = rom_backup::retain_snapshot(snapshot, &policy, limits).unwrap();
    let (db, actual_report) = crate::Sqlite::retain_journal_candidate(
        &path,
        &scratch.path("retained"),
        &policy,
        limits,
        || Ok(()),
    )
    .unwrap();
    assert_eq!(actual_report, report);
    compare(&db, expected);
    assert_eq!(
        db.reaction_update(WorkUpdate::DeliveryStarted {
            claim: old_claim,
            now: 0
        }),
        Err(rom::Error::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
#[test]
fn candidate_offline_migration_rebuilds_native_positions() {
    let (scratch, path, snapshot, bytes, old_claim) = source();
    let limits = limits();
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| Ok(After { amount: v.amount })).unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let expected = rom_backup::migrate_snapshot(snapshot, &plan, limits).unwrap();
    let db = crate::Sqlite::migrate_journal_candidate(
        &path,
        &scratch.path("migrated"),
        &plan,
        limits,
        || Ok(()),
    )
    .unwrap();
    compare(&db, expected);
    assert_eq!(
        db.reaction_update(WorkUpdate::DeliveryStarted {
            claim: old_claim,
            now: 0
        }),
        Err(rom::Error::Conflict)
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn candidate_maintenance_failures_preserve_source_and_fresh_destination() {
    let (scratch, path, snapshot, bytes, _old_claim) = source();
    let limits = BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    };
    let policy = RetentionPolicy::new(snapshot.state.retry_epochs()).journal_through(1);
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| Ok(After { amount: v.amount })).unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let retention = scratch.path("retention-failed");
    assert!(matches!(
        crate::Sqlite::retain_journal_candidate(&path, &retention, &policy, limits, || Err(
            rom::Error::NotCommitted
        )),
        Err(rom::Error::NotCommitted)
    ));
    assert!(!retention.exists());
    let migration = scratch.path("migration-failed");
    assert!(matches!(
        crate::Sqlite::migrate_journal_candidate(&path, &migration, &plan, limits, || Err(
            rom::Error::NotCommitted
        )),
        Err(rom::Error::NotCommitted)
    ));
    assert!(!migration.exists());
    let denied = scratch.path("too-large");
    assert!(matches!(
        crate::Sqlite::retain_journal_candidate(
            &path,
            &denied,
            &policy,
            BackupLimits {
                max_bytes: 1,
                max_records: 4096
            },
            || Ok(())
        ),
        Err(rom::Error::TooLarge)
    ));
    assert!(!denied.exists());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| Ok(After { amount: v.amount })).unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()))
}
#[test]
fn candidate_both_maintenance_routes_refuse_existing_destination() {
    let (scratch, path, snapshot, bytes, _) = source();
    let destination = scratch.path("existing");
    let sentinel = b"existing destination must survive";
    std::fs::write(&destination, sentinel).unwrap();
    let policy = RetentionPolicy::new(snapshot.state.retry_epochs()).journal_through(1);
    assert!(matches!(
        crate::Sqlite::retain_journal_candidate(&path, &destination, &policy, limits(), || Ok(())),
        Err(rom::Error::Conflict)
    ));
    assert!(matches!(
        crate::Sqlite::migrate_journal_candidate(&path, &destination, &plan(), limits(), || Ok(())),
        Err(rom::Error::Conflict)
    ));
    assert_eq!(std::fs::read(&destination).unwrap(), sentinel);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
static MIGRATION_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn counted_conversion(v: Before) -> rom::Result<After> {
    MIGRATION_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(After { amount: v.amount })
}
#[test]
fn candidate_aggregate_malformed_inventory_is_rejected_before_callbacks() {
    let (scratch, path, snapshot, _, _) = source();
    let c = rusqlite::Connection::open(&path).unwrap();
    let budget = BackupLimits {
        max_bytes: 16 * 1024,
        max_records: 4096,
    };
    crate::native_journal::admit_inventory(&c, budget).unwrap();
    let malformed = "!".repeat(9000);
    assert!(malformed.len() < budget.max_bytes);
    assert_eq!(
        c.execute("UPDATE work_records SET data=?", [&malformed])
            .unwrap(),
        2
    );
    drop(c);
    let bytes = std::fs::read(&path).unwrap();
    let policy = RetentionPolicy::new(snapshot.state.retry_epochs());
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(counted_conversion).unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    MIGRATION_CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
    let published = std::cell::Cell::new(0);
    let retained = scratch.path("aggregate-retained");
    assert!(matches!(
        crate::Sqlite::retain_journal_candidate(&path, &retained, &policy, budget, || {
            published.set(published.get() + 1);
            Ok(())
        }),
        Err(rom::Error::TooLarge)
    ));
    let migrated = scratch.path("aggregate-migrated");
    assert!(matches!(
        crate::Sqlite::migrate_journal_candidate(&path, &migrated, &plan, budget, || {
            published.set(published.get() + 1);
            Ok(())
        }),
        Err(rom::Error::TooLarge)
    ));
    assert_eq!(MIGRATION_CALLS.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(published.get(), 0);
    assert!(!retained.exists());
    assert!(!migrated.exists());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn wal_source(
    candidate: bool,
) -> (
    Scratch,
    std::path::PathBuf,
    Snapshot,
    ClaimKey,
    rusqlite::Connection,
) {
    let scratch = Scratch::new();
    let path = scratch.path("wal-source");
    let db = if candidate {
        crate::Sqlite::open_with_validation_limits(&path, rom::StorageLimits::default(), limits())
            .unwrap()
    } else {
        let owner =
            rom_backup::NativeOwnership::acquire(&path, rom_backup::NativeAccess::OpenOrCreate)
                .unwrap();
        crate::Sqlite::open_connection_for_layout(
            &path,
            rom::StorageLimits::default(),
            limits(),
            Some(owner),
            false,
        )
        .unwrap()
    };
    db.register(&[descriptor()]).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let hold = rusqlite::Connection::open(&path).unwrap();
    hold.execute_batch("PRAGMA wal_autocheckpoint=0").unwrap();
    db.commit(&bundle("wal-only-a")).unwrap();
    db.commit(&bundle("wal-only-b")).unwrap();
    let old_claim = crate::native_work_test_support::claim(&db).key();
    let snapshot = crate::snapshot::collect_native_snapshot(
        &db.connection.lock().unwrap(),
        limits(),
        candidate,
    )
    .unwrap();
    assert_eq!(
        hold.query_row("SELECT COUNT(*) FROM resources", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    db.close().unwrap();
    // A separate live connection prevents the last-close WAL checkpoint.
    let main = rusqlite::Connection::open_with_flags(
        format!("file:{}?immutable=1", path.display()),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    assert_eq!(
        main.query_row("SELECT COUNT(*) FROM resources", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(main);
    (scratch, path, snapshot, old_claim, hold)
}
#[test]
fn candidate_maintenance_and_format10_conversion_preserve_committed_wal() {
    for candidate in [false, true] {
        let (scratch, path, snapshot, old_claim, _hold) = wal_source(candidate);
        let wal = path.with_file_name("wal-source-wal");
        let main_bytes = std::fs::read(&path).unwrap();
        let wal_bytes = std::fs::read(&wal).unwrap();
        assert!(wal_bytes.len() > 32);
        let preserved = || {
            assert_eq!(std::fs::read(&path).unwrap(), main_bytes);
            assert_eq!(std::fs::read(&wal).unwrap(), wal_bytes);
        };
        if candidate {
            let policy = RetentionPolicy::new(snapshot.state.retry_epochs()).journal_through(1);
            let (expected, _) = rom_backup::retain_snapshot(
                crate::snapshot::read_snapshot(&path, limits(), |c, limits| {
                    crate::snapshot::collect_native_snapshot(c, limits, true)
                })
                .unwrap(),
                &policy,
                limits(),
            )
            .unwrap();
            let (db, _) = crate::Sqlite::retain_journal_candidate(
                &path,
                &scratch.path("wal-retained"),
                &policy,
                limits(),
                || Ok(()),
            )
            .unwrap();
            compare(&db, expected);
            assert_eq!(
                db.reaction_update(WorkUpdate::DeliveryStarted {
                    claim: old_claim.clone(),
                    now: 0
                }),
                Err(rom::Error::Conflict)
            );
            preserved();
            let plan = plan();
            let expected = rom_backup::migrate_snapshot(snapshot, &plan, limits()).unwrap();
            let db = crate::Sqlite::migrate_journal_candidate(
                &path,
                &scratch.path("wal-migrated"),
                &plan,
                limits(),
                || Ok(()),
            )
            .unwrap();
            compare(&db, expected);
            preserved();
            let rejected = scratch.path("wal-rejected");
            assert!(matches!(
                crate::Sqlite::migrate_journal_candidate(
                    &path,
                    &rejected,
                    &plan,
                    limits(),
                    || Err(rom::Error::NotCommitted)
                ),
                Err(rom::Error::NotCommitted)
            ));
            assert!(!rejected.exists());
            preserved();
        } else {
            let db = crate::Sqlite::upgrade_journal_candidate(
                &path,
                &scratch.path("wal-converted"),
                limits(),
                || Ok(()),
            )
            .unwrap();
            compare(&db, snapshot);
            assert_eq!(
                db.reaction_update(WorkUpdate::DeliveryStarted {
                    claim: old_claim,
                    now: 0
                }),
                Err(rom::Error::Conflict)
            );
            preserved();
            let rejected = scratch.path("wal-rejected");
            assert!(matches!(
                crate::Sqlite::upgrade_journal_candidate(&path, &rejected, limits(), || Err(
                    rom::Error::NotCommitted
                )),
                Err(rom::Error::NotCommitted)
            ));
            assert!(!rejected.exists());
            preserved();
        }
    }
}

#[test]
#[ignore = "Requires root admission of <=1 GiB memory and bounded owned disk envelope"]
fn candidate_valid_padded_header_requires_larger_than_default_profile() {
    let (scratch, path, snapshot, _, _) = source();
    let c = rusqlite::Connection::open(&path).unwrap();
    let mut header: String = c
        .query_row("SELECT data FROM rom_state", [], |r| r.get(0))
        .unwrap();
    header.extend(std::iter::repeat_n(' ', 129 * 1024 * 1024 - header.len()));
    assert_eq!(header.len(), 129 * 1024 * 1024);
    c.execute("UPDATE rom_state SET data=?", [&header]).unwrap();
    drop(header);
    drop(c);
    let original = std::fs::read(&path).unwrap();
    let profile = BackupLimits {
        max_bytes: 192 * 1024 * 1024,
        max_records: 4096,
    };
    let owner =
        rom_backup::NativeOwnership::acquire(&path, rom_backup::NativeAccess::Existing).unwrap();
    assert!(matches!(
        crate::Sqlite::open_connection_for_layout(
            &path,
            rom::StorageLimits::default(),
            BackupLimits::default(),
            Some(owner),
            true
        ),
        Err(rom::Error::TooLarge)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let calls = std::cell::Cell::new(0);
    MIGRATION_CALLS.store(0, std::sync::atomic::Ordering::SeqCst);
    let rejecting_plan = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(counted_conversion).unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    let rejected = scratch.path("default-retention");
    let policy = RetentionPolicy::new(snapshot.state.retry_epochs()).journal_through(1);
    assert!(matches!(
        crate::Sqlite::retain_journal_candidate(
            &path,
            &rejected,
            &policy,
            BackupLimits::default(),
            || {
                calls.set(calls.get() + 1);
                Ok(())
            }
        ),
        Err(rom::Error::TooLarge)
    ));
    assert!(!rejected.exists());
    let rejected = scratch.path("default-migration");
    assert!(matches!(
        crate::Sqlite::migrate_journal_candidate(
            &path,
            &rejected,
            &rejecting_plan,
            BackupLimits::default(),
            || {
                calls.set(calls.get() + 1);
                Ok(())
            }
        ),
        Err(rom::Error::TooLarge)
    ));
    assert!(!rejected.exists());
    assert_eq!(calls.get(), 0);
    assert_eq!(MIGRATION_CALLS.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let owner =
        rom_backup::NativeOwnership::acquire(&path, rom_backup::NativeAccess::Existing).unwrap();
    let db = crate::Sqlite::open_connection_for_layout(
        &path,
        rom::StorageLimits::default(),
        profile,
        Some(owner),
        true,
    )
    .unwrap();
    assert_eq!(db.validation_limits.max_bytes, profile.max_bytes);
    assert_eq!(db.validation_limits.max_records, profile.max_records);
    let reopened =
        crate::snapshot::collect_native_snapshot(&db.connection.lock().unwrap(), profile, true)
            .unwrap();
    assert_eq!(
        serde_json::to_value(reopened).unwrap(),
        serde_json::to_value(&snapshot).unwrap()
    );
    db.close().unwrap();
    let expected_source = crate::snapshot::read_snapshot(&path, profile, |c, p| {
        crate::snapshot::collect_native_snapshot(c, p, true)
    })
    .unwrap();
    let (expected, _) = rom_backup::retain_snapshot(expected_source, &policy, profile).unwrap();
    let (db, _) = crate::Sqlite::retain_journal_candidate(
        &path,
        &scratch.path("large-retained"),
        &policy,
        profile,
        || Ok(()),
    )
    .unwrap();
    compare_profile(&db, expected, profile);
    let expected_source = crate::snapshot::read_snapshot(&path, profile, |c, p| {
        crate::snapshot::collect_native_snapshot(c, p, true)
    })
    .unwrap();
    let expected = rom_backup::migrate_snapshot(expected_source, &plan(), profile).unwrap();
    let db = crate::Sqlite::migrate_journal_candidate(
        &path,
        &scratch.path("large-migrated"),
        &plan(),
        profile,
        || Ok(()),
    )
    .unwrap();
    compare_profile(&db, expected, profile);
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn fixture_preserves_image_when_test_unwinds() {
    let path = std::cell::RefCell::new(None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let scratch = Scratch::new();
        let image = scratch.path("preserved-image");
        std::fs::write(&image, b"owned fixture evidence").unwrap();
        *path.borrow_mut() = Some(image);
        panic!("intentional preservation witness");
    }));
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(path.into_inner().unwrap()).unwrap(),
        b"owned fixture evidence"
    );
}
