use super::{
    MemoryRecord,
    support::{BACKENDS, Backend, Scratch, seed},
};
use rom::{Error, Resource, Result};
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::path::Path;

#[derive(Clone, Resource)]
#[resource(name = "ownership-memory-records", version = 2)]
struct UpdatedRecord {
    value: String,
}

fn convert(value: MemoryRecord) -> Result<UpdatedRecord> {
    Ok(UpdatedRecord { value: value.value })
}
fn plan(convert: fn(MemoryRecord) -> Result<UpdatedRecord>) -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<MemoryRecord, UpdatedRecord>(convert).unwrap(),
    ])
    .unwrap()
}
fn migrate(
    backend: Backend,
    source: &Path,
    destination: &Path,
    plan: &MigrationPlan,
    observe: impl FnOnce() -> Result<()>,
) -> Result<()> {
    match backend {
        Backend::Sqlite => rom_sqlite::Sqlite::migrate_from_observed(
            source,
            destination,
            plan,
            BackupLimits::default(),
            observe,
        )
        .map(drop),
        Backend::Redb => rom_redb::Redb::migrate_from_observed(
            source,
            destination,
            plan,
            BackupLimits::default(),
            observe,
        )
        .map(drop),
    }
}

#[test]
fn maintenance_rejects_a_live_source_before_conversion() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let store = backend.open(&scratch.path()).unwrap();
        seed(&*store, "kept");
        let destination = scratch.0.join("destination");
        assert_eq!(
            migrate(
                backend,
                &scratch.path(),
                &destination,
                &plan(convert),
                || Ok(())
            ),
            Err(Error::Conflict)
        );
        let descriptors = [MemoryRecord::descriptor()];
        let limits = BackupLimits::default();
        let policy = rom_backup::RetentionPolicy::new(rom::RetryEpochs::default());
        let (upgrade, retain) = match backend {
            Backend::Sqlite => (
                rom_sqlite::Sqlite::upgrade_from(
                    scratch.path(),
                    &destination,
                    &descriptors,
                    limits,
                )
                .map(drop),
                rom_sqlite::Sqlite::retain_from(scratch.path(), &destination, &policy, limits)
                    .map(drop),
            ),
            Backend::Redb => (
                rom_redb::Redb::upgrade_from(scratch.path(), &destination, &descriptors, limits)
                    .map(drop),
                rom_redb::Redb::retain_from(scratch.path(), &destination, &policy, limits)
                    .map(drop),
            ),
        };
        assert_eq!(upgrade, Err(Error::Conflict), "{backend:?}: upgrade");
        assert_eq!(retain, Err(Error::Conflict), "{backend:?}: retention");
        if matches!(backend, Backend::Sqlite) {
            assert_eq!(
                rom_sqlite::Sqlite::rebuild_indexes_from(scratch.path(), &destination, limits)
                    .map(drop),
                Err(Error::Conflict)
            );
        }
        assert!(!destination.exists());
        drop(store);
    }
}

fn inspect_reservations_then_fail(value: MemoryRecord) -> Result<UpdatedRecord> {
    let config: serde_json::Value = serde_json::from_str(&value.value).unwrap();
    let backend = if config["redb"].as_bool().unwrap() {
        Backend::Redb
    } else {
        Backend::Sqlite
    };
    for name in ["source", "destination"] {
        let path = Path::new(config[name].as_str().unwrap());
        assert!(
            matches!(backend.open(path), Err(Error::Conflict)),
            "{backend:?}: {name} must be reserved during conversion"
        );
    }
    Err(Error::NotCommitted)
}

#[test]
fn conversion_holds_both_reservations_and_failure_releases_them() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let destination = scratch.0.join("destination");
        let store = backend.open(&scratch.path()).unwrap();
        seed(&*store, &serde_json::json!({"redb": matches!(backend,Backend::Redb), "source":scratch.path(), "destination":destination}).to_string());
        drop(store);
        let before = std::fs::read(scratch.path()).unwrap();
        assert_eq!(
            migrate(
                backend,
                &scratch.path(),
                &destination,
                &plan(inspect_reservations_then_fail),
                || Ok(())
            ),
            Err(Error::NotCommitted)
        );
        assert_eq!(std::fs::read(scratch.path()).unwrap(), before);
        assert!(!destination.exists());
        drop(backend.open(&scratch.path()).unwrap());
        drop(backend.open(&destination).unwrap());
    }
}

#[test]
fn publication_retains_reservations_and_returns_a_single_link_destination() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let destination = scratch.0.join("destination");
        let store = backend.open(&scratch.path()).unwrap();
        seed(&*store, "kept");
        drop(store);
        let before = std::fs::read(scratch.path()).unwrap();
        migrate(
            backend,
            &scratch.path(),
            &destination,
            &plan(convert),
            || {
                assert!(matches!(
                    backend.open(&scratch.path()),
                    Err(Error::Conflict)
                ));
                assert!(matches!(backend.open(&destination), Err(Error::Conflict)));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(std::fs::read(scratch.path()).unwrap(), before);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&destination).unwrap().nlink(), 1);
        }
        drop(backend.open(&destination).unwrap());
    }
}
