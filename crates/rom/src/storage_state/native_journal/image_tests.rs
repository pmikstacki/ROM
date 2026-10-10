use super::*;
fn metadata() -> StorageMetadata {
    let state = StorageState::new(StorageLimits::default()).unwrap();
    let mut m = StorageMetadata::from_state(&state);
    m.head = 2;
    m.events = (1..=2)
        .map(|position| JournalEvent {
            position,
            identity: format!("event-{position}"),
            row: Row {
                key: Key {
                    kind: "test".into(),
                    id: position.to_string(),
                },
                revision: position,
                value: Some(json!({"n": position})),
                protected: ProtectedMetadata::default(),
            },
        })
        .collect();
    m
}
#[test]
fn canonical_and_native_images_preserve_exact_metadata() {
    let m = metadata();
    let value = serde_json::to_value(&m).unwrap();
    let image = JournalImage::from_metadata(m).unwrap();
    let native =
        JournalImage::from_native(image.header().clone(), image.events().to_vec()).unwrap();
    assert_eq!(serde_json::to_value(native.into_metadata()).unwrap(), value);
    assert_eq!(serde_json::to_value(image.into_metadata()).unwrap(), value);
}
#[test]
fn canonical_image_rejects_duplicate_identity_holes_and_retention_limits() {
    let mut cases = vec![];
    let mut m = metadata();
    m.events[1].identity = m.events[0].identity.clone();
    cases.push(m);
    let mut m = metadata();
    m.events[1].position = 3;
    cases.push(m);
    let mut m = metadata();
    m.events[0].position = 0;
    cases.push(m);
    let mut m = metadata();
    m.limits.journal_rows = 1;
    cases.push(m);
    let mut m = metadata();
    m.limits.journal_bytes = 1;
    cases.push(m);
    for m in cases {
        assert!(JournalImage::from_metadata(m).is_err());
    }
}
#[test]
fn native_image_rejects_header_count_and_byte_disagreement() {
    let image = JournalImage::from_metadata(metadata()).unwrap();
    let mut events = image.events().to_vec();
    events.pop();
    assert!(JournalImage::from_native(image.header().clone(), events).is_err());
    let mut p = image.header().parts();
    p.journal_bytes += 1;
    let header = MetadataHeader::from_parts(p).unwrap();
    assert!(JournalImage::from_native(header, image.events().to_vec()).is_err());
}
