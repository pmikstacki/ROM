//! Original public Authentik response; no token, credential, or provider mutation.
use super::oidc::parse_jwks;

#[test]
fn original_authentik_registered_certificate_metadata_is_compatible() {
    let original = include_bytes!("../tests/fixtures/authentik-2026-8-3-jwks.json");
    let document: serde_json::Value = serde_json::from_slice(original).unwrap();
    let key_id = document["keys"][0]["kid"].as_str().unwrap();
    assert_eq!(key_id.len(), 86);
    assert!(document["keys"][0]["x5c"].is_array());
    assert!(document["keys"][0]["x5t"].is_string());
    assert!(document["keys"][0]["x5t#S256"].is_string());
    let parsed = parse_jwks(original).expect("original public Authentik JWKS must parse");
    assert_eq!(parsed.len(), 1);
    assert!(parsed.contains_key(key_id));
}
