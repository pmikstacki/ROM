use crate::archive::MAGIC;
use crate::*;
use rom::{Descriptor, Error, StorageLimits, StorageState, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn unchecked_archive(path: &Path, manifest: &serde_json::Value, snapshot: &serde_json::Value) {
    let header = serde_json::to_vec(manifest).unwrap();
    let body = serde_json::to_vec(snapshot).unwrap();
    let mut hash = Sha256::new();
    hash.update(&header);
    hash.update(&body);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&hash.finalize());
    bytes.extend(header);
    bytes.extend(body);
    fs::write(path, bytes).unwrap();
}
#[test]
fn reader_rejects_valid_checksum_wrong_format_counts_or_metadata() {
    let stage =
        Stage::new(&std::env::temp_dir().join(format!("rom-backup-unit-{}", std::process::id())))
            .unwrap();
    let snapshot = Snapshot {
        state: StorageState::new(StorageLimits::default()).unwrap(),
        rows: vec![],
        receipts: vec![],
        events: vec![],
        effects: vec![],
        descriptors: vec![],
        references: vec![],
    };
    let manifest = serde_json::to_value(snapshot.manifest(Backend::Sqlite)).unwrap();
    let body = serde_json::to_value(snapshot).unwrap();
    for field in ["archive_version", "storage_format", "rows"] {
        let mut bad = manifest.clone();
        bad[field] = json!(42);
        unchecked_archive(stage.path(), &bad, &body);
        assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
    }
    for field in ["head", "receipts", "effects"] {
        let mut bad = body.clone();
        bad["state"][field] = json!(42);
        unchecked_archive(stage.path(), &manifest, &bad);
        assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
    }
    let mut bad = body.clone();
    bad["state"]["work"]["roots"] = json!({"orphan":1});
    unchecked_archive(stage.path(), &manifest, &bad);
    assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
    unchecked_archive(stage.path(), &manifest, &body);
    assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_ok());
}
#[test]
fn explicit_legacy_archive_upgrade_preserves_data_and_requires_descriptors() {
    let base = std::env::temp_dir().join(format!("rom-legacy-upgrade-{}", std::process::id()));
    let source = Stage::new(&base.with_extension("source")).unwrap();
    let target = base.with_extension("target");
    let row = rom::Row {
        key: rom::Key {
            kind: "items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(json!({"name":"retained"})),
        protected: Default::default(),
    };
    let receipt = rom::Receipt {
        retry_epoch: 0,
        replay_version: None,
        identity: "original".into(),
        fingerprint: "unchanged".into(),
        row: row.clone(),
    };
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .bundle(&rom::Bundle {
            expected: None,
            receipt: receipt.clone(),
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
    let body = json!({"state":state,"rows":[row],"receipts":[receipt],"events":[["original",row]],"effects":[]});
    let manifest = json!({"archive_version":1,"storage_format":3,"backend":"Sqlite","rows":1,"receipts":1,"events":1,"effects":0,"work":0,"external_blobs_included":false,"external_deliveries_included":false});
    unchecked_archive(source.path(), &manifest, &body);
    let original = fs::read(source.path()).unwrap();
    assert!(read(source.path(), Backend::Sqlite, BackupLimits::default()).is_err());
    assert!(
        upgrade_v1_archive(
            source.path(),
            &target,
            Backend::Sqlite,
            &[],
            BackupLimits::default()
        )
        .is_err()
    );
    assert!(!target.exists());
    let descriptors = vec![Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![rom::FieldDescriptor {
            name: "name".into(),
            shape: rom::Shape::String,
        }],
    }];
    let result = upgrade_v1_archive(
        source.path(),
        &target,
        Backend::Sqlite,
        &descriptors,
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!((result.archive_version, result.storage_format), (4, 6));
    let (_, upgraded) = read(&target, Backend::Sqlite, BackupLimits::default()).unwrap();
    let upgraded = serde_json::to_value(upgraded).unwrap();
    let mut expected_body = body.clone();
    expected_body["receipts"][0]["replay_version"] = json!(1);
    for field in ["state", "rows", "receipts", "events", "effects"] {
        assert_eq!(upgraded[field], expected_body[field]);
    }
    assert_eq!(fs::read(source.path()).unwrap(), original);
    assert_eq!(
        upgrade_v1_archive(
            source.path(),
            &target,
            Backend::Sqlite,
            &descriptors,
            BackupLimits::default()
        ),
        Err(Error::Conflict)
    );
    fs::remove_file(target).unwrap();
}
#[test]
fn publication_race_and_symlink_refuse_overwrite() {
    let destination = std::env::temp_dir().join(format!("rom-backup-race-{}", std::process::id()));
    let stage = Stage::new(&destination).unwrap();
    fs::write(stage.path(), b"new").unwrap();
    fs::write(&destination, b"old").unwrap();
    assert_eq!(stage.publish(), Err(Error::Conflict));
    assert_eq!(fs::read(&destination).unwrap(), b"old");
    fs::remove_file(&destination).unwrap();
    std::os::unix::fs::symlink(stage.path(), &destination).unwrap();
    assert!(matches!(
        read(&destination, Backend::Sqlite, BackupLimits::default()),
        Err(Error::Storage)
    ));
    assert!(matches!(Stage::new(&destination), Err(Error::Conflict)));
    fs::remove_file(destination).unwrap();
}

#[test]
fn catalogued_archive_upgrade_preserves_or_binds_receipt_origin() {
    for archive_version in [2, 3] {
        let upgrade = if archive_version == 2 {
            |source: &Path, destination: &Path| {
                upgrade_v2_archive(
                    source,
                    destination,
                    Backend::Sqlite,
                    BackupLimits::default(),
                )
            }
        } else {
            |source: &Path, destination: &Path| {
                upgrade_v3_archive(
                    source,
                    destination,
                    Backend::Sqlite,
                    BackupLimits::default(),
                )
            }
        };
        for origin in [None, Some(1)] {
            let target = std::env::temp_dir().join(format!(
                "rom-archive-upgrade-{archive_version}-{}-{origin:?}",
                std::process::id()
            ));
            let source = Stage::new(&target.with_extension("source")).unwrap();
            let row = rom::Row {
                key: rom::Key {
                    kind: "items".into(),
                    id: "one".into(),
                },
                revision: 1,
                value: Some(json!({"name":"retained"})),
                protected: Default::default(),
            };
            let receipt = rom::Receipt {
                retry_epoch: 0,
                replay_version: origin,
                identity: "original".into(),
                fingerprint: "unchanged".into(),
                row: row.clone(),
            };
            let mut state = StorageState::new(StorageLimits::default()).unwrap();
            state
                .bundle(&rom::Bundle {
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
                events: vec![("original".into(), row)],
                effects: vec![],
                references: vec![],
                descriptors: vec![Descriptor {
                    kind: "items".into(),
                    version: 2,
                    fields: vec![rom::FieldDescriptor {
                        name: "name".into(),
                        shape: rom::Shape::String,
                    }],
                }],
            };
            let mut manifest = snapshot.manifest(Backend::Sqlite);
            manifest.archive_version = archive_version;
            manifest.storage_format = archive_version + 2;
            let manifest = serde_json::to_value(manifest).unwrap();
            let mut legacy_body = serde_json::to_value(&snapshot).unwrap();
            legacy_body["state"]
                .as_object_mut()
                .unwrap()
                .remove("retry_epochs");
            legacy_body["receipts"][0]
                .as_object_mut()
                .unwrap()
                .remove("retry_epoch");
            unchecked_archive(source.path(), &manifest, &legacy_body);
            let original = fs::read(source.path()).unwrap();
            assert!(matches!(
                read(source.path(), Backend::Sqlite, BackupLimits::default()),
                Err(Error::Unsupported(_))
            ));
            let upgraded = upgrade(source.path(), &target).unwrap();
            assert_eq!((upgraded.archive_version, upgraded.storage_format), (4, 6));
            let (_, data) = read(&target, Backend::Sqlite, BackupLimits::default()).unwrap();
            assert_eq!(data.receipts[0].replay_version, Some(origin.unwrap_or(2)));
            assert_eq!(data.receipts[0].row, snapshot.receipts[0].row);
            assert_eq!(data.receipts[0].fingerprint, "unchanged");
            assert_eq!(data.receipts[0].retry_epoch, 0);
            assert_eq!(data.state.retry_epochs(), rom::RetryEpochs::default());
            assert_eq!(data.descriptors, snapshot.descriptors);
            assert_eq!(fs::read(source.path()).unwrap(), original);
            assert_eq!(upgrade(source.path(), &target), Err(Error::Conflict));
            fs::remove_file(&target).unwrap();
            legacy_body["state"]["retry_epochs"] =
                json!({"current":1,"admission_floor":0,"replay_floor":0});
            unchecked_archive(source.path(), &manifest, &legacy_body);
            let original = fs::read(source.path()).unwrap();
            assert!(matches!(
                upgrade(source.path(), &target),
                Err(Error::Unsupported(_))
            ));
            assert!(!target.exists());
            assert_eq!(fs::read(source.path()).unwrap(), original);
        }
    }
}
