use crate::oidc::parse_jwks;
use serde_json::{Value, json};

const ORIGINAL: &[u8] = include_bytes!("../tests/fixtures/authentik-2026-8-3-jwks.json");
fn changed(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(ORIGINAL).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn rsa_components_are_the_only_key_source() {
    let original = parse_jwks(ORIGINAL).expect("original registered metadata is unused");
    let bytes = changed(|value| {
        value["keys"][0]["x5c"] = json!(["AQID"]);
        value["keys"][0]["x5t"] = json!("AAAAAAAAAAAAAAAAAAAAAAAAAAA");
        value["keys"][0]["x5t#S256"] = json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
    });
    assert_eq!(
        parse_jwks(&bytes).unwrap().keys().collect::<Vec<_>>(),
        original.keys().collect::<Vec<_>>()
    );
    assert!(
        parse_jwks(&changed(|value| {
            value["keys"][0].as_object_mut().unwrap().remove("n");
        }))
        .is_err()
    );
    assert!(
        parse_jwks(&changed(|value| {
            value["keys"][0].as_object_mut().unwrap().remove("e");
        }))
        .is_err()
    );
}

#[test]
fn unused_metadata_has_closed_types_and_finite_bounds() {
    for (field, value) in [
        ("x5c", json!(null)),
        ("x5c", json!("not-a-chain")),
        ("x5c", json!([])),
        ("x5c", json!([7])),
        ("x5c", json!(["!"])),
        ("x5c", json!(["A".repeat(8196)])),
        ("x5c", json!(["AQID", "AQID", "AQID", "AQID", "AQID"])),
        ("x5t", json!(null)),
        ("x5t", json!(7)),
        ("x5t", json!("wrong")),
        ("x5t#S256", json!(null)),
        ("x5t#S256", json!("wrong")),
        ("x5u", json!("https://unapproved.invalid/certificate")),
        ("unknown", json!("extension")),
    ] {
        assert!(
            parse_jwks(&changed(|document| document["keys"][0][field] = value)).is_err(),
            "accepted {field}"
        );
    }
}

#[test]
fn certificate_metadata_cannot_override_algorithm_usage_or_key_operations() {
    for (field, value) in [
        ("alg", json!("HS256")),
        ("use", json!("enc")),
        ("key_ops", json!(["sign"])),
    ] {
        assert!(parse_jwks(&changed(|document| document["keys"][0][field] = value)).is_err());
    }
    let bytes = changed(|document| {
        document["keys"][0]["kid"] = json!("a".repeat(256));
    });
    assert!(parse_jwks(&bytes).is_ok());
    assert!(
        parse_jwks(&changed(
            |document| document["keys"][0]["kid"] = json!("a".repeat(257))
        ))
        .is_err()
    );
    assert!(
        parse_jwks(&changed(
            |document| document["keys"][0]["kid"] = json!("control\u{0085}")
        ))
        .is_err()
    );
}

#[test]
fn duplicate_metadata_and_duplicate_ids_are_rejected() {
    let bytes = String::from_utf8(ORIGINAL.to_vec()).unwrap();
    let duplicate = bytes.replacen("\"x5c\":", "\"x5c\":[\"AQID\"],\"x5c\":", 1);
    assert_ne!(duplicate, bytes);
    assert!(parse_jwks(duplicate.as_bytes()).is_err());
    assert!(
        parse_jwks(&changed(|document| {
            let key = document["keys"][0].clone();
            document["keys"].as_array_mut().unwrap().push(key);
        }))
        .is_err()
    );
}
