use super::*;
use redb::ReadableTableMetadata;

#[test]
fn public_open_creates_real_format_eleven_layout() {
    let path = directory("public-eleven-layout");
    let storage = Redb::open(path.join("db")).unwrap();
    storage.register(&[Item::descriptor()]).unwrap();
    storage.commit(&bundle_with_work("one")).unwrap();
    let tx = storage.db.begin_read().unwrap();
    assert_eq!(
        tx.open_table(crate::format::META)
            .unwrap()
            .get("format")
            .unwrap()
            .unwrap()
            .value(),
        11,
        "public writers must create the native journal format"
    );
    assert_eq!(tx.list_tables().unwrap().count(), 13);
    let state = tx.open_table(crate::format::STATE).unwrap();
    assert_eq!(state.len().unwrap(), 3);
    let raw = state.get("metadata").unwrap().unwrap();
    let header: serde_json::Value = serde_json::from_str(raw.value()).unwrap();
    assert!(header.get("events").is_none());
    assert_eq!(
        tx.open_table(crate::format::POSITIONS)
            .unwrap()
            .len()
            .unwrap(),
        1
    );
    drop((raw, state));
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn public_archive_and_restore_truthfully_use_native_eleven() {
    let path = directory("public-eleven-archive");
    let limits = BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    };
    let storage =
        Redb::open_with_validation_limits(path.join("source"), StorageLimits::default(), limits)
            .unwrap();
    storage.register(&[Item::descriptor()]).unwrap();
    storage.commit(&bundle_with_work("one")).unwrap();
    storage.commit(&bundle("two")).unwrap();
    let archive = path.join("before.rombk");
    let manifest = storage.backup_to(&archive, limits).unwrap();
    assert_eq!(manifest.archive_version, 7);
    assert_eq!(
        manifest.storage_format, 11,
        "archive must report its real native format"
    );
    let (_, mut expected) = rom_backup::read(&archive, rom_backup::Backend::Redb, limits).unwrap();
    assert_eq!(
        serde_json::to_value(&expected).unwrap(),
        canonical(&storage)
    );
    let generation = expected.state.journal_head("journal-items").generation;
    expected.state.prepare_restore().unwrap();
    let restored = Redb::restore_from(&archive, path.join("restored"), limits).unwrap();
    assert_eq!(restored.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(restored.validation_limits.max_records, limits.max_records);
    assert_fenced_conversion(
        canonical(&restored),
        serde_json::to_value(expected).unwrap(),
        &generation,
    );
    let tx = restored.db.begin_read().unwrap();
    assert_eq!(tx.list_tables().unwrap().count(), 13);
    assert_eq!(
        tx.open_table(crate::format::POSITIONS)
            .unwrap()
            .len()
            .unwrap(),
        2
    );
    drop((tx, restored, storage));
    std::fs::remove_dir_all(path).unwrap();
}
