use super::*;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Sequence {
    snapshots: VecDeque<Result<BTreeMap<String, DecodingKey>, AuthError>>,
    calls: Arc<AtomicUsize>,
}
impl TrustedKeys for Sequence {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.snapshots
            .pop_front()
            .unwrap_or(Err(AuthError::Unavailable))
    }
}
fn configured(
    snapshots: Vec<Result<BTreeMap<String, DecodingKey>, AuthError>>,
) -> (OidcIdTokenAdapter<Sequence>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let source = Sequence {
        snapshots: snapshots.into(),
        calls: calls.clone(),
    };
    (
        OidcIdTokenAdapter::configured(
            "provider",
            "https://issuer.example",
            "studio-client",
            source,
        )
        .unwrap(),
        calls,
    )
}
fn snapshot(index: usize) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
    Ok(BTreeMap::from([("fixture".into(), signing::public(index))]))
}

#[test]
fn expired_keys_refresh_completely_and_evidence_never_outlives_cached_keys() {
    let (mut verifier, calls) = configured(vec![snapshot(0), snapshot(1)]);
    let initial = signing::token(&claims(), header(), 0);
    assert_eq!(
        verifier
            .authenticate(&initial, NONCE, OidcTokenBindings::default(), NOW)
            .unwrap()
            .valid_until(),
        NOW + 30
    );
    assert_eq!(
        verifier
            .authenticate(&initial, NONCE, OidcTokenBindings::default(), NOW + 20)
            .unwrap()
            .valid_until(),
        NOW + 30
    );
    let mut unknown = header();
    unknown.kid = Some("unknown".into());
    assert_eq!(
        verifier.authenticate(
            &signing::token(&claims(), unknown, 0),
            NONCE,
            OidcTokenBindings::default(),
            NOW + 1
        ),
        Err(AuthError::RefreshLimited)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        verifier
            .authenticate(&initial, NONCE, OidcTokenBindings::default(), NOW + 30)
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let rotated = signing::token(&claims(), header(), 1);
    assert_eq!(
        verifier
            .authenticate(&rotated, NONCE, OidcTokenBindings::default(), NOW + 30)
            .unwrap()
            .valid_until(),
        NOW + 60
    );
    assert_eq!(
        verifier.authenticate(&rotated, NONCE, OidcTokenBindings::default(), NOW + 60),
        Err(AuthError::Unavailable)
    );
    assert_eq!(
        verifier.authenticate(&rotated, NONCE, OidcTokenBindings::default(), NOW + 61),
        Err(AuthError::RefreshLimited)
    );
}

#[test]
fn invalid_header_never_fetches_and_invalid_complete_key_sets_never_issue_proof() {
    let (mut verifier, calls) = configured(vec![]);
    for kind in ["jku", "jwk", "x5u", "x5c", "crit", "b64", "unexpected"] {
        // Unknown JOSE fields cannot add trust or change the payload interpretation.
        // Replace only the unverified header of a signed synthetic token.
        let token = signing::token(&claims(), header(), 0);
        let mut parts = token.split('.');
        let mut raw = json!({"alg":"RS256","kid":"fixture","typ":"JWT"});
        raw[kind] = json!(null);
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        let changed = format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(raw.to_string()),
            parts.nth(1).unwrap(),
            parts.next().unwrap()
        );
        assert_eq!(
            verifier.authenticate(&changed, NONCE, OidcTokenBindings::default(), NOW),
            Err(AuthError::WrongProfile)
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for map in [
        BTreeMap::new(),
        BTreeMap::from([("".into(), signing::public(0))]),
        BTreeMap::from([("x".repeat(257), signing::public(0))]),
        (0..9)
            .map(|index| (index.to_string(), signing::public(0)))
            .collect(),
    ] {
        let (mut verifier, calls) = configured(vec![Ok(map)]);
        assert_eq!(
            verifier.authenticate(
                &signing::token(&claims(), header(), 0),
                NONCE,
                OidcTokenBindings::default(),
                NOW
            ),
            Err(AuthError::Invalid)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn positional_header_array_is_rejected_before_host_key_acquisition() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let (mut verifier, calls) = configured(vec![]);
    let token = signing::token(&claims(), header(), 0);
    let (_, remainder) = token.split_once('.').unwrap();
    let changed = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(r#"["RS256","fixture","JWT"]"#),
        remainder
    );
    assert!(
        verifier
            .authenticate(&changed, NONCE, OidcTokenBindings::default(), NOW)
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn symmetric_algorithm_cannot_use_an_issuer_rsa_key_or_fetch_keys() {
    let (mut verifier, calls) = configured(vec![]);
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("fixture".into());
    let token = jsonwebtoken::encode(
        &header,
        &claims(),
        &jsonwebtoken::EncodingKey::from_secret(b"synthetic-HMAC-key"),
    )
    .unwrap();
    assert_eq!(
        verifier.authenticate(&token, NONCE, OidcTokenBindings::default(), NOW),
        Err(AuthError::WrongProfile)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
