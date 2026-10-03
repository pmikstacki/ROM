use super::oidc::{original_expiry, parse_jwks};

#[test]
fn key_sets_reject_duplicate_ids_and_token_selected_trust_fields() {
    let duplicate = br#"{"keys":[{"kid":"same","kty":"RSA","alg":"RS256","use":"sig","n":"AQAB","e":"AQAB"},{"kid":"same","kty":"RSA","alg":"RS256","use":"sig","n":"AQAB","e":"AQAB"}]}"#;
    assert!(parse_jwks(duplicate).is_err());
    for wrong in [
        br#"{"keys":[{"kid":"k","kty":"oct","k":"secret"}]}"#.as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","alg":"RS256","use":"enc","n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","alg":"HS256","n":"AQAB","e":"AQAB"}]}"#.as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","key_ops":["sign"],"n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[{"kid":"k","kty":"RSA","jku":"https://evil.example","n":"AQAB","e":"AQAB"}]}"#
            .as_slice(),
        br#"{"keys":[]}"#.as_slice(),
    ] {
        assert!(parse_jwks(wrong).is_err());
    }
}

#[test]
fn original_expiry_parser_is_a_bound_not_an_authenticator() {
    // This intentionally unsigned value cannot establish a session. The caller must verify first.
    let token = "e30.eyJleHAiOjIwMDB9.unsigned";
    assert_eq!(original_expiry(token).unwrap(), 2000);
    for invalid in [
        "bad",
        "e30.e30.x",
        "e30.eyJleHAiOi0xfQ.x",
        "e30.eyJleHAiOjIuNX0.x",
    ] {
        assert!(original_expiry(invalid).is_err());
    }
}
