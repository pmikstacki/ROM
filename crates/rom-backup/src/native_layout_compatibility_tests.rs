//! Native layout markers share the unchanged, fully validated canonical archive body.
use crate::tests::unchecked_archive;
use crate::*;
use rom::{Error, json};

#[test]
fn new_archives_identify_the_current_native_layout() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
        let destination = std::env::temp_dir().join(format!(
            "rom-native-layout-writer-{}-{backend:?}",
            std::process::id()
        ));
        let stage = Stage::new(&destination).unwrap();
        // The writer refuses an existing destination, including the staging placeholder.
        let archive = stage.path().with_extension("archive");
        let manifest = write(&archive, backend, &snapshot, BackupLimits::default()).unwrap();
        assert_eq!((manifest.archive_version, manifest.storage_format), (7, 11));
        let (read_manifest, restored) = read(&archive, backend, BackupLimits::default()).unwrap();
        assert_eq!(read_manifest, manifest);
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(snapshot).unwrap()
        );
        std::fs::remove_file(archive).unwrap();
    }
}

#[test]
fn version_seven_layout_markers_preserve_manifest_body_and_source() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
        let body = serde_json::to_value(&snapshot).unwrap();
        for marker in [9, 10, 11] {
            let destination = std::env::temp_dir().join(format!(
                "rom-native-layout-compat-{}-{backend:?}-{marker}",
                std::process::id()
            ));
            let stage = Stage::new(&destination).unwrap();
            let mut manifest = serde_json::to_value(snapshot.manifest(backend)).unwrap();
            manifest["archive_version"] = json!(7);
            manifest["storage_format"] = json!(marker);
            unchecked_archive(stage.path(), &manifest, &body);
            let original = std::fs::read(stage.path()).unwrap();
            let (actual_manifest, actual_snapshot) =
                read(stage.path(), backend, BackupLimits::default()).unwrap();
            assert_eq!(serde_json::to_value(actual_manifest).unwrap(), manifest);
            assert_eq!(serde_json::to_value(actual_snapshot).unwrap(), body);
            assert_eq!(std::fs::read(stage.path()).unwrap(), original);
        }
    }
}

#[test]
fn layout_compatibility_is_exact_and_keeps_canonical_validation() {
    let (snapshot, _) = crate::epoch_upgrade_tests::operator_snapshot();
    let body = serde_json::to_value(&snapshot).unwrap();
    let original_manifest = serde_json::to_value(snapshot.manifest(Backend::Sqlite)).unwrap();
    let destination =
        std::env::temp_dir().join(format!("rom-native-layout-reject-{}", std::process::id()));
    let stage = Stage::new(&destination).unwrap();
    for (version, marker) in [
        (6, 9),
        (6, 10),
        (6, 11),
        (7, 8),
        (7, 12),
        (8, 9),
        (8, 10),
        (8, 11),
    ] {
        let mut manifest = original_manifest.clone();
        manifest["archive_version"] = json!(version);
        manifest["storage_format"] = json!(marker);
        unchecked_archive(stage.path(), &manifest, &body);
        assert!(matches!(
            read(stage.path(), Backend::Sqlite, BackupLimits::default()),
            Err(Error::Unsupported(_))
        ));
    }
    for marker in [9, 10, 11] {
        let mut manifest = original_manifest.clone();
        manifest["storage_format"] = json!(marker);
        let mut malformed = manifest.clone();
        malformed["operator_receipts"] = json!(0);
        unchecked_archive(stage.path(), &malformed, &body);
        assert!(matches!(
            read(stage.path(), Backend::Sqlite, BackupLimits::default()),
            Err(Error::Storage)
        ));
        malformed = manifest.clone();
        malformed["unknown"] = json!(true);
        unchecked_archive(stage.path(), &malformed, &body);
        assert!(matches!(
            read(stage.path(), Backend::Sqlite, BackupLimits::default()),
            Err(Error::Storage)
        ));
        let mut malformed_body = body.clone();
        malformed_body["state"]["work"]["roots"]["orphan"] = json!(1);
        unchecked_archive(stage.path(), &manifest, &malformed_body);
        assert!(matches!(
            read(stage.path(), Backend::Sqlite, BackupLimits::default()),
            Err(Error::Storage)
        ));
        unchecked_archive(stage.path(), &manifest, &body);
        assert!(matches!(
            read(stage.path(), Backend::Redb, BackupLimits::default()),
            Err(Error::Unsupported(_))
        ));
        let limits = BackupLimits {
            max_bytes: std::fs::metadata(stage.path()).unwrap().len() as usize - 1,
            ..BackupLimits::default()
        };
        assert!(matches!(
            read(stage.path(), Backend::Sqlite, limits),
            Err(Error::TooLarge)
        ));
    }
}
