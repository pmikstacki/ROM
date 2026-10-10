use super::*;

#[test]
fn public_snapshot_rejects_embedded_row_key_corruption_without_writes() {
    let (storage, path) = candidate(4);
    let original = bundle("one");
    storage.commit(&original).unwrap();
    assert_eq!(
        storage.snapshot("journal-items", 10, 4096).unwrap(),
        vec![original.receipt.row.clone()]
    );
    let key = original.receipt.row.key.clone();
    let mut corrupted = original.receipt.row;
    corrupted.key.kind = "other-items".into();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::ROWS)
        .unwrap()
        .insert(
            (key.kind.as_str(), key.id.as_str()),
            serde_json::to_string(&corrupted).unwrap().as_str(),
        )
        .unwrap();
    tx.commit().unwrap();
    let before = super::file_witness::capture(&path.join("db")).unwrap();
    let result = storage.snapshot("journal-items", 10, 4096);
    assert_eq!(
        super::file_witness::capture(&path.join("db")).unwrap(),
        before
    );
    assert_eq!(result, Err(Error::Storage));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn public_load_rejects_embedded_row_key_corruption_without_writes() {
    let (storage, path) = candidate(4);
    let original = bundle("one");
    storage.commit(&original).unwrap();
    let key = original.receipt.row.key.clone();
    assert_eq!(
        storage.load(&key).unwrap(),
        Some(original.receipt.row.clone())
    );
    let mut corrupted = original.receipt.row;
    corrupted.key.id = "other".into();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::ROWS)
        .unwrap()
        .insert(
            (key.kind.as_str(), key.id.as_str()),
            serde_json::to_string(&corrupted).unwrap().as_str(),
        )
        .unwrap();
    tx.commit().unwrap();
    let before = super::file_witness::capture(&path.join("db")).unwrap();
    let result = storage.load(&key);
    assert_eq!(
        super::file_witness::capture(&path.join("db")).unwrap(),
        before
    );
    assert_eq!(result, Err(Error::Storage));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn public_receipt_rejects_embedded_identity_corruption_without_writes() {
    let (storage, path) = candidate(4);
    let original = bundle("one");
    storage.commit(&original).unwrap();
    let identity = original.receipt.identity.clone();
    assert_eq!(
        storage.receipt(&identity).unwrap(),
        Some(original.receipt.clone())
    );
    let mut corrupted = original.receipt;
    corrupted.identity = "other".into();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::RECEIPTS)
        .unwrap()
        .insert(
            identity.as_str(),
            serde_json::to_string(&corrupted).unwrap().as_str(),
        )
        .unwrap();
    tx.commit().unwrap();
    let before = super::file_witness::capture(&path.join("db")).unwrap();
    let result = storage.receipt(&identity);
    assert_eq!(
        super::file_witness::capture(&path.join("db")).unwrap(),
        before
    );
    assert_eq!(result, Err(Error::Storage));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}
