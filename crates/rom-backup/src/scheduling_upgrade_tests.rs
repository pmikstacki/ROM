//! Actual archive predecessor shape, including populated operator receipts.
use crate::tests::unchecked_archive;
use crate::*;
use rom::{Error, json};

#[test]
fn scheduling_formats_fence_previous_readers() {
    assert_eq!(STORAGE_FORMAT, 11);
    assert_eq!(crate::model::ARCHIVE_VERSION, 7);
}
#[test]
fn ordinary_reader_refuses_operator_aware_predecessor_archive() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
        let destination = std::env::temp_dir().join(format!(
            "rom-scheduling-old-{}-{backend:?}",
            std::process::id()
        ));
        let stage = Stage::new(&destination).unwrap();
        let mut manifest = serde_json::to_value(snapshot.manifest(backend)).unwrap();
        manifest["archive_version"] = json!(6);
        manifest["storage_format"] = json!(8);
        unchecked_archive(
            stage.path(),
            &manifest,
            &serde_json::to_value(snapshot).unwrap(),
        );
        let original = std::fs::read(stage.path()).unwrap();
        assert!(matches!(
            read(stage.path(), backend, BackupLimits::default()),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(std::fs::read(stage.path()).unwrap(), original);
    }
}
#[test]
fn explicit_archive_six_upgrade_preserves_operator_and_work_metadata() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
        let destination = std::env::temp_dir().join(format!(
            "rom-scheduling-upgrade-{}-{backend:?}",
            std::process::id()
        ));
        let source = Stage::new(&destination.with_extension("source")).unwrap();
        let mut manifest = serde_json::to_value(snapshot.manifest(backend)).unwrap();
        manifest["archive_version"] = json!(6);
        manifest["storage_format"] = json!(8);
        let before = serde_json::to_value(&snapshot).unwrap();
        unchecked_archive(source.path(), &manifest, &before);
        let original = std::fs::read(source.path()).unwrap();
        let upgraded = upgrade_v6_archive(
            source.path(),
            &destination,
            backend,
            BackupLimits::default(),
        )
        .unwrap();
        assert_eq!((upgraded.archive_version, upgraded.storage_format), (7, 11));
        assert_eq!(upgraded.operator_receipts, 1);
        let (_, after) = read(&destination, backend, BackupLimits::default()).unwrap();
        assert_eq!(serde_json::to_value(after).unwrap(), before);
        assert_eq!(std::fs::read(source.path()).unwrap(), original);
        assert_eq!(
            upgrade_v6_archive(
                source.path(),
                &destination,
                backend,
                BackupLimits::default()
            ),
            Err(Error::Conflict)
        );
        std::fs::remove_file(destination).unwrap();
    }
}
#[test]
fn predecessor_archive_rejects_scheduling_fields_in_pending_or_effect_intent() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        for location in ["pending", "intent"] {
            let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
            let destination = std::env::temp_dir().join(format!(
                "rom-scheduling-invalid-{}-{backend:?}-{location}",
                std::process::id()
            ));
            let source = Stage::new(&destination.with_extension("source")).unwrap();
            let mut manifest = serde_json::to_value(snapshot.manifest(backend)).unwrap();
            manifest["archive_version"] = json!(6);
            manifest["storage_format"] = json!(8);
            let mut body = serde_json::to_value(snapshot).unwrap();
            if location == "pending" {
                body["state"]["work"]["work"]["pending"]["pending"]["not_before"] = json!(null);
            } else {
                body["effects"] = json!([{"identity":"original", "ordinal":0,
                    "intent":{"channel":"audit", "payload":{}, "not_before":20}}]);
            }
            unchecked_archive(source.path(), &manifest, &body);
            let original = std::fs::read(source.path()).unwrap();
            assert_eq!(
                upgrade_v6_archive(
                    source.path(),
                    &destination,
                    backend,
                    BackupLimits::default()
                ),
                Err(Error::Storage)
            );
            assert!(!destination.exists());
            assert_eq!(std::fs::read(source.path()).unwrap(), original);
        }
    }
}
