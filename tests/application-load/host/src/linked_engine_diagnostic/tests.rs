use super::*;

fn identity() -> rom_sqlite::LinkedEngineIdentity {
    rom_sqlite::LinkedEngineIdentity {
        version: "3.53.4".into(),
        source_id: SOURCE_ID.into(),
        compile_options: vec![
            "ENABLE_COLUMN_METADATA".into(),
            "THREADSAFE=1".into(),
            "USE_URI".into(),
        ],
    }
}

#[test]
fn linked_engine_arguments_are_closed_before_query_or_output() {
    assert!(!arguments(&[FLAG.into()]).unwrap());
    assert!(arguments(&[FLAG.into(), EXPECT.into()]).unwrap());
    for values in [
        vec![],
        vec!["wrong"],
        vec![FLAG, "wrong"],
        vec![FLAG, EXPECT, "extra"],
    ] {
        let values: Vec<_> = values.into_iter().map(String::from).collect();
        assert!(arguments(&values).is_err());
    }
}

#[test]
fn linked_engine_selected_profile_checks_version_source_and_each_required_option() {
    let (bytes, matched) = encode(identity(), true).unwrap();
    assert!(matched && bytes.len() <= MAXIMUM_BYTES);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["profile_matches"], true);
    assert_eq!(value["acceptance"], false);
    assert_eq!(value["persisted_database"], false);
    assert_eq!(value["remote_endpoint"], false);
    for case in 0..5 {
        let mut changed = identity();
        match case {
            0 => changed.version = "3.53.2".into(),
            1 => changed.source_id = "different-source".into(),
            _ => {
                changed.compile_options.remove(case - 2);
            }
        }
        let (bytes, matched) = encode(changed, true).unwrap();
        assert!(!matched);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["profile_matches"],
            false
        );
    }
}

#[test]
fn linked_engine_unasserted_identity_is_explicit_and_invalid_metadata_has_no_output() {
    let (bytes, matched) = encode(identity(), false).unwrap();
    assert!(matched);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["expected_profile"], serde_json::Value::Null);
    assert_eq!(value["profile_matches"], serde_json::Value::Null);
    let mut invalid = identity();
    invalid.version.clear();
    assert!(encode(invalid, true).is_err());
}

#[test]
fn linked_engine_maximum_bounded_identity_fits_complete_json_without_truncation() {
    let mut maximum = identity();
    maximum.version = "v".repeat(32);
    maximum.source_id = "s".repeat(192);
    maximum.compile_options = (0..128)
        .map(|index| format!("{index:03}{}", "\\\"".repeat(126) + "x"))
        .collect();
    let (bytes, _) = encode(maximum, false).unwrap();
    assert!(bytes.len() <= MAXIMUM_BYTES);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["engine"]["compile_options"].as_array().unwrap().len(),
        128
    );
}
