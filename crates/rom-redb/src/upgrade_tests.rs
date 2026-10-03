//! Previous native format is inspected without changing its retry protocol.
use crate::{Redb, format::META};
use rom::{
    Bundle, Error, Key, Receipt, Resource, RetryEpochs, Row, Storage, StorageLimits, StorageState,
    json,
};
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration, Snapshot};

#[derive(Clone, Resource)]
#[resource(name = "items", version = 2)]
struct Before {
    name: String,
}
#[derive(Clone, Resource)]
#[resource(name = "items", version = 3)]
struct After {
    label: String,
}

fn source(path: &std::path::Path, origin: Option<u32>) -> RetryEpochs {
    let epochs = RetryEpochs {
        current: 4,
        admission_floor: 2,
        replay_floor: 1,
    };
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state.apply_retention(epochs, 0, 0, 0).unwrap();
    let row = Row {
        key: Key {
            kind: "items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(json!({"name":"kept"})),
        protected: Default::default(),
    };
    let receipt = Receipt {
        retry_epoch: 3,
        replay_version: origin,
        identity: "request".into(),
        fingerprint: "kept".into(),
        row: row.clone(),
    };
    state
        .bundle(&Bundle {
            expected: None,
            receipt: receipt.clone(),
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
    let snapshot = Snapshot {
        state,
        rows: vec![row.clone()],
        receipts: vec![receipt],
        events: vec![("request".into(), row)],
        effects: vec![],
        references: vec![],
        descriptors: vec![Before::descriptor().canonical().unwrap()],
    };
    let db = Redb::restore_snapshot(
        snapshot,
        rom_backup::NativeOwnership::acquire(path, rom_backup::NativeAccess::Fresh).unwrap(),
        BackupLimits::default(),
        || Ok(()),
    )
    .unwrap();
    drop(db);
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_write().unwrap();
    tx.open_table(META).unwrap().insert("format", 6).unwrap();
    tx.commit().unwrap();
    drop(db);
    epochs
}
#[test]
fn format_six_requires_upgrade_preserving_epochs_origins_and_source_bytes() {
    for origin in [None, Some(1)] {
        let directory =
            std::env::temp_dir().join(format!("rom-redb-six-{}-{origin:?}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("source");
        let epochs = source(&path, origin);
        let original = std::fs::read(&path).unwrap();
        assert!(matches!(Redb::open(&path), Err(Error::Unsupported(_))));
        assert!(
            std::fs::read(&path).unwrap() == original,
            "ordinary open changed source"
        );
        let upgraded = Redb::upgrade_from(
            &path,
            directory.join("upgraded"),
            &[Before::descriptor()],
            BackupLimits::default(),
        )
        .unwrap();
        assert_eq!(upgraded.retry_epochs().unwrap(), epochs);
        let receipt = upgraded.receipt("request").unwrap().unwrap();
        assert_eq!((receipt.retry_epoch, receipt.replay_version), (3, origin));
        assert_eq!(receipt.fingerprint, "kept");
        assert_eq!(receipt.row.value, Some(json!({"name":"kept"})));
        assert!(
            std::fs::read(&path).unwrap() == original,
            "upgrade changed source"
        );
        let plan = MigrationPlan::new(vec![
            ResourceMigration::new::<Before, After>(|old| Ok(After { label: old.name })).unwrap(),
        ])
        .unwrap();
        let migrated = Redb::migrate_from(
            &path,
            directory.join("migrated"),
            &plan,
            BackupLimits::default(),
        )
        .unwrap();
        assert_eq!(migrated.retry_epochs().unwrap(), epochs);
        let receipt = migrated.receipt("request").unwrap().unwrap();
        assert_eq!(receipt.retry_epoch, 3);
        assert_eq!(receipt.replay_version, Some(origin.unwrap_or(2)));
        assert_eq!(receipt.row.value, Some(json!({"label":"kept"})));
        assert!(
            std::fs::read(&path).unwrap() == original,
            "migration changed source"
        );
        drop((upgraded, migrated));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
