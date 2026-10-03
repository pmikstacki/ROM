//! Epoch-aware predecessor archives preserve metadata instead of rebinding it.
use crate::tests::unchecked_archive;
use crate::*;
use rom::{
    Bundle, Cause, Descriptor, Error, FieldDescriptor, Key, PendingWork, ReactionLimits, Receipt,
    RetryEpochs, Row, Shape, StorageLimits, StorageState, WorkPayload, json,
};

fn snapshot(origin: Option<u32>) -> Snapshot {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .apply_retention(
            RetryEpochs {
                current: 4,
                admission_floor: 2,
                replay_floor: 1,
            },
            0,
            0,
            0,
        )
        .unwrap();
    let row = Row {
        key: Key {
            kind: "items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(json!({"name":"unchanged"})),
        protected: Default::default(),
    };
    let receipt = Receipt {
        retry_epoch: 3,
        replay_version: origin,
        identity: "original".into(),
        fingerprint: "retained".into(),
        row: row.clone(),
    };
    state
        .bundle(&Bundle {
            expected: None,
            receipt: receipt.clone(),
            changed: true,
            effects: vec![],
            reactions: vec![PendingWork {
                id: "pending".into(),
                cause: Cause {
                    retry_epoch: 3,
                    root: "original".into(),
                    parent: None,
                    depth: 0,
                    started_at: 1,
                    path: vec![],
                },
                definition: "reaction".into(),
                version: 1,
                service_key: "worker".into(),
                payload: WorkPayload::Source(row.clone()),
            }],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
    Snapshot {
        state,
        rows: vec![row.clone()],
        receipts: vec![receipt],
        events: vec![("original".into(), row)],
        effects: vec![],
        references: vec![],
        descriptors: vec![Descriptor {
            kind: "items".into(),
            version: 2,
            fields: vec![FieldDescriptor {
                name: "name".into(),
                shape: Shape::String,
            }],
        }],
    }
}

#[test]
fn archive_four_requires_explicit_upgrade_and_preserves_epochs_and_origins() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        for origin in [None, Some(1)] {
            let target = std::env::temp_dir().join(format!(
                "rom-epoch-upgrade-{}-{backend:?}-{origin:?}",
                std::process::id()
            ));
            let source = Stage::new(&target.with_extension("source")).unwrap();
            let snapshot = snapshot(origin);
            snapshot.validate().unwrap();
            let mut manifest = snapshot.manifest(backend);
            manifest.archive_version = 4;
            manifest.storage_format = 6;
            let before = serde_json::to_value(&snapshot).unwrap();
            unchecked_archive(
                source.path(),
                &serde_json::to_value(manifest).unwrap(),
                &before,
            );
            let original = std::fs::read(source.path()).unwrap();
            assert!(matches!(
                read(source.path(), backend, BackupLimits::default()),
                Err(Error::Unsupported(_))
            ));
            let manifest =
                upgrade_v4_archive(source.path(), &target, backend, BackupLimits::default())
                    .unwrap();
            assert_eq!((manifest.archive_version, manifest.storage_format), (5, 7));
            let (_, after) = read(&target, backend, BackupLimits::default()).unwrap();
            assert_eq!(serde_json::to_value(after).unwrap(), before);
            assert!(
                std::fs::read(source.path()).unwrap() == original,
                "source archive changed"
            );
            assert_eq!(
                upgrade_v4_archive(source.path(), &target, backend, BackupLimits::default()),
                Err(Error::Conflict)
            );
            std::fs::remove_file(target).unwrap();
        }
    }
}
#[test]
fn current_snapshot_upgrade_checks_catalog_integrity_and_complete_bounds() {
    let input = snapshot(None);
    let expected = serde_json::to_value(&input).unwrap();
    let descriptors = input.descriptors.clone();
    let output = upgrade_current_snapshot(input, &descriptors, BackupLimits::default()).unwrap();
    assert_eq!(serde_json::to_value(output).unwrap(), expected);
    assert!(matches!(
        upgrade_current_snapshot(snapshot(None), &[], BackupLimits::default()),
        Err(Error::Unsupported(_))
    ));
    let mut corrupt = snapshot(Some(1));
    corrupt.receipts[0].retry_epoch = 5;
    assert!(matches!(
        upgrade_current_snapshot(corrupt, &descriptors, BackupLimits::default()),
        Err(Error::Storage)
    ));
    for limits in [
        BackupLimits {
            max_records: 1,
            ..BackupLimits::default()
        },
        BackupLimits {
            max_bytes: 1,
            ..BackupLimits::default()
        },
    ] {
        assert!(matches!(
            upgrade_current_snapshot(snapshot(None), &descriptors, limits),
            Err(Error::TooLarge)
        ));
    }
    assert!(matches!(
        upgrade_legacy_snapshot(snapshot(Some(1)), &descriptors, BackupLimits::default()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn physical_records_share_logical_collector_budgets_without_entering_snapshot() {
    let state =
        serde_json::to_string(&StorageState::new(StorageLimits::default()).unwrap()).unwrap();
    let descriptor = Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![],
    };
    let text = serde_json::to_string(&descriptor).unwrap();
    let total = state.len() + text.len() + "items".len() + 9;
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: total,
            max_records: 2,
        },
    )
    .unwrap();
    collect.descriptor("items", &text).unwrap();
    let original = serde_json::to_value(&collect.snapshot).unwrap();
    collect.physical(9).unwrap();
    assert_eq!(serde_json::to_value(&collect.snapshot).unwrap(), original);
    assert_eq!(collect.physical(0), Err(Error::TooLarge));
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: total - 1,
            max_records: 3,
        },
    )
    .unwrap();
    collect.physical(9).unwrap();
    assert_eq!(collect.descriptor("items", &text), Err(Error::TooLarge));
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: usize::MAX,
            max_records: usize::MAX,
        },
    )
    .unwrap();
    assert_eq!(collect.physical(usize::MAX), Err(Error::TooLarge));
}
