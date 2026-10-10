use super::*;
fn parts() -> MetadataHeaderParts {
    MetadataHeaderParts {
        retry_epochs: RetryEpochs {
            current: 8,
            admission_floor: 6,
            replay_floor: 4,
        },
        limits: StorageLimits::default(),
        generation: "scalar-journal".into(),
        head: u64::MAX,
        floor: u64::MAX,
        receipts: 12,
        effects: 13,
        journal_records: 0,
        journal_bytes: 0,
    }
}
#[test]
fn parts_serialization_preserves_advanced_epochs_and_empty_maximum_head() {
    let expected = parts();
    let encoded = serde_json::to_vec(&expected).unwrap();
    let decoded: MetadataHeaderParts = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, expected);
    assert_eq!(
        MetadataHeader::from_parts(decoded).unwrap().parts(),
        expected
    );
}
#[test]
fn parts_deserialization_rejects_missing_fields_and_type_mismatch() {
    let encoded = serde_json::to_value(parts()).unwrap();
    for field in [
        "retry_epochs",
        "limits",
        "generation",
        "head",
        "floor",
        "receipts",
        "effects",
        "journal_records",
        "journal_bytes",
    ] {
        let mut missing = encoded.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<MetadataHeaderParts>(missing).is_err());
    }
    let mut bad = encoded;
    bad["journal_bytes"] = json!("zero");
    assert!(serde_json::from_value::<MetadataHeaderParts>(bad).is_err());
}
#[test]
fn deserialized_parts_are_not_validated_header_authority() {
    let mut encoded = serde_json::to_value(parts()).unwrap();
    encoded["floor"] = json!(0);
    let decoded: MetadataHeaderParts = serde_json::from_value(encoded).unwrap();
    assert_eq!(MetadataHeader::from_parts(decoded), Err(Error::Storage));
}

#[test]
fn parts_deserialization_rejects_unknown_native_fields() {
    let mut encoded = serde_json::to_value(parts()).unwrap();
    encoded["unexpected_authority"] = json!(true);
    assert!(serde_json::from_value::<MetadataHeaderParts>(encoded).is_err());
}
