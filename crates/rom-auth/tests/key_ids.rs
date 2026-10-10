#![cfg(feature = "oidc")]
//! Signed public-profile tests for opaque bounded key IDs and issuer-selected caches.
#[path = "support/signing.rs"]
mod signing;
use jsonwebtoken::{Algorithm, Header};
use rom_auth::{
    AuthError, OidcIdTokenAdapter,
    jwt::{DecodingKey, JwtAdapter, TrustedKeys},
    oidc::OidcTokenBindings,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const NOW: u64 = 1_800_000_000;
const NONCE: &str = "retained-independent-nonce";
struct Keys {
    ids: Vec<String>,
    calls: Arc<AtomicUsize>,
}
impl TrustedKeys for Keys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .ids
            .iter()
            .map(|id| (id.clone(), signing::public(0)))
            .collect())
    }
}
fn verify(
    kid: &str,
    ids: &[String],
    oidc: bool,
) -> (Result<rom_auth::VerifiedIdentity, AuthError>, usize) {
    let calls = Arc::new(AtomicUsize::new(0));
    let source = Keys {
        ids: ids.to_vec(),
        calls: calls.clone(),
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    header.typ = Some(if oidc { "JWT" } else { "at+jwt" }.into());
    let claims = json!({"iss":"https://issuer.example","aud":"consumer","sub":"synthetic-human","iat":NOW,"exp":NOW+600,"nonce":NONCE,"jti":"synthetic-id","client_id":"consumer","principal_kind":"human"});
    let token = signing::token(&claims, header, 0);
    let result = if oidc {
        OidcIdTokenAdapter::configured("provider", "https://issuer.example", "consumer", source)
            .unwrap()
            .authenticate(&token, NONCE, OidcTokenBindings::default(), NOW)
    } else {
        JwtAdapter::configured("provider", "https://issuer.example", "consumer", source)
            .unwrap()
            .authenticate(&token, NOW)
    };
    (result, calls.load(Ordering::SeqCst))
}
#[test]
fn original_provider_shape_and_utf8_byte_boundary_work_in_both_signed_profiles() {
    for kid in [
        format!("{}-_", "A".repeat(84)),
        "k".repeat(256),
        "é".repeat(128),
    ] {
        for oidc in [false, true] {
            let (result, calls) = verify(&kid, std::slice::from_ref(&kid), oidc);
            assert_eq!(result.unwrap().valid_until(), NOW + 30);
            assert_eq!(calls, 1);
        }
    }
}
#[test]
fn empty_control_and_oversize_headers_are_rejected_before_key_acquisition() {
    for kid in [
        String::new(),
        "k".repeat(257),
        "é".repeat(129),
        "key\n".into(),
        "key\u{7f}".into(),
        "key\u{85}".into(),
    ] {
        for oidc in [false, true] {
            let (result, calls) = verify(&kid, &["safe".into()], oidc);
            assert_eq!(result.unwrap_err(), AuthError::UnknownKey);
            assert_eq!(calls, 0);
        }
    }
}
#[test]
fn invalid_refreshed_key_ids_are_rejected_even_when_selected_key_is_valid() {
    for invalid in [
        String::new(),
        "k".repeat(257),
        "unsafe\n".into(),
        "unsafe\u{85}".into(),
    ] {
        for oidc in [false, true] {
            let (result, calls) = verify("safe", &["safe".into(), invalid.clone()], oidc);
            assert_eq!(result.unwrap_err(), AuthError::Invalid);
            assert_eq!(calls, 1);
        }
    }
}
#[test]
fn exact_matching_never_normalizes_case_unicode_or_common_long_prefixes() {
    let prefix = "p".repeat(255);
    for (selected, stored) in [
        ("Case".to_owned(), "case".to_owned()),
        ("é".to_owned(), "e\u{301}".to_owned()),
        (format!("{prefix}A"), format!("{prefix}B")),
    ] {
        for oidc in [false, true] {
            let (result, calls) = verify(&selected, std::slice::from_ref(&stored), oidc);
            assert_eq!(result.unwrap_err(), AuthError::UnknownKey);
            assert_eq!(calls, 1);
        }
    }
}
#[test]
fn key_count_limit_still_rejects_nine_issuer_selected_keys() {
    let ids: Vec<_> = (0..9).map(|index| format!("key-{index}")).collect();
    for oidc in [false, true] {
        let (result, calls) = verify("key-0", &ids, oidc);
        assert_eq!(result.unwrap_err(), AuthError::Invalid);
        assert_eq!(calls, 1);
    }
}
