#![cfg(feature = "oidc")]
//! Real synthetic signatures exercise the declared ID-token profile, without a provider or browser claim.
#[path = "support/oidc_keys.rs"]
mod oidc_keys;
#[path = "support/signing.rs"]
mod signing;
use jsonwebtoken::{Algorithm, Header};
use rom_auth::jwt::{DecodingKey, JwtAdapter, TrustedKeys};
use rom_auth::oidc::OidcTokenBindings;
use rom_auth::{AuthError, IdentityProfile, OidcIdTokenAdapter, PrincipalKind};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const NOW: u64 = 1_800_000_000;
const NONCE: &str = "retained-independent-host-nonce";
struct Keys;
impl TrustedKeys for Keys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        Ok(BTreeMap::from([("fixture".into(), signing::public(0))]))
    }
}
fn adapter() -> OidcIdTokenAdapter<Keys> {
    OidcIdTokenAdapter::configured("provider", "https://issuer.example", "studio-client", Keys)
        .unwrap()
}
fn claims() -> Value {
    json!({"iss":"https://issuer.example","sub":"human-subject","aud":"studio-client","exp":NOW+600,"iat":NOW,"nonce":NONCE,"email":"ignored@example.invalid","admin":true})
}
fn header() -> Header {
    let mut value = Header::new(Algorithm::RS256);
    value.kid = Some("fixture".into());
    value
}
fn authenticate(claims: &Value, header: Header) -> Result<rom_auth::VerifiedIdentity, AuthError> {
    adapter().authenticate(
        &signing::token(claims, header, 0),
        NONCE,
        OidcTokenBindings::default(),
        NOW,
    )
}

#[test]
fn id_token_constructs_sealed_bounded_human_proof_with_explicit_profile() {
    let token = signing::token(&claims(), header(), 0);
    let proof = adapter()
        .authenticate(&token, NONCE, OidcTokenBindings::default(), NOW)
        .unwrap();
    assert_eq!(proof.profile(), IdentityProfile::OidcRs256Human);
    assert_eq!(proof.principal_kind(), PrincipalKind::Human);
    assert_eq!(proof.subject(), "human-subject");
    assert_eq!(proof.authority(), "provider");
    assert_eq!(proof.valid_until(), NOW + 30);
    let debug = format!("{proof:?}");
    for private in [
        token.as_str(),
        NONCE,
        proof.subject(),
        "ignored@example.invalid",
        "admin",
    ] {
        assert!(!debug.contains(private));
    }
}

#[test]
fn wrong_signature_binding_nonce_times_or_missing_claims_never_construct_proof() {
    assert!(
        adapter()
            .authenticate(
                &signing::token(&claims(), header(), 1),
                NONCE,
                OidcTokenBindings::default(),
                NOW
            )
            .is_err()
    );
    for (key, replacement) in [
        ("iss", json!("https://other.example")),
        ("iss", json!(["https://issuer.example"])),
        ("aud", json!("other-client")),
        ("aud", json!(["studio-client", 7])),
        ("nonce", json!("wrong")),
        ("nonce", json!(null)),
        ("exp", json!(NOW)),
        ("iat", json!(NOW + 1)),
        ("iat", json!(NOW - 3601)),
        ("sub", json!("")),
        ("sub", json!("é")),
        ("nbf", json!(NOW + 1)),
        ("nbf", json!(null)),
    ] {
        let mut malformed = claims();
        malformed[key] = replacement;
        assert!(
            authenticate(&malformed, header()).is_err(),
            "rejected {key}"
        );
    }
    for key in ["iss", "aud", "sub", "exp", "iat", "nonce"] {
        let mut value = claims();
        value.as_object_mut().unwrap().remove(key);
        assert!(authenticate(&value, header()).is_err(), "required {key}");
    }
}

#[test]
fn multi_audience_requires_explicit_trust_and_client_authorized_party() {
    let mut value = claims();
    value["aud"] = json!(["studio-client", "trusted-other"]);
    value["azp"] = json!("studio-client");
    assert!(authenticate(&value, header()).is_err());
    let mut verifier = adapter()
        .with_trusted_audiences(&["trusted-other"])
        .unwrap();
    assert!(
        verifier
            .authenticate(
                &signing::token(&value, header(), 0),
                NONCE,
                OidcTokenBindings::default(),
                NOW
            )
            .is_ok()
    );
    for audience in [
        json!([]),
        json!(["studio-client", "studio-client"]),
        json!(["studio-client", ""]),
        json!(["studio-client", "unapproved"]),
    ] {
        value["aud"] = audience;
        assert!(
            verifier
                .authenticate(
                    &signing::token(&value, header(), 0),
                    NONCE,
                    OidcTokenBindings::default(),
                    NOW
                )
                .is_err()
        );
    }
    value["aud"] = json!(["studio-client", "trusted-other"]);
    for party in [Some(json!("other-client")), Some(json!(null)), None] {
        value.as_object_mut().unwrap().remove("azp");
        if let Some(party) = party {
            value["azp"] = party;
        }
        assert!(
            verifier
                .authenticate(
                    &signing::token(&value, header(), 0),
                    NONCE,
                    OidcTokenBindings::default(),
                    NOW
                )
                .is_err()
        );
    }
}

#[test]
fn old_access_verifier_rejects_id_tokens_and_oidc_rejects_access_type() {
    assert_eq!(
        JwtAdapter::configured("provider", "https://issuer.example", "studio-client", Keys)
            .unwrap()
            .authenticate(&signing::token(&claims(), header(), 0), NOW),
        Err(AuthError::WrongProfile)
    );
    let mut access = header();
    access.typ = Some("at+jwt".into());
    assert_eq!(
        authenticate(&claims(), access),
        Err(AuthError::WrongProfile)
    );
}

#[test]
fn present_hashes_bind_the_original_returned_tokens_and_cannot_be_ignored() {
    let mut value = claims();
    // SHA-256 left-half, base64url without padding, for the synthetic token "SlAV32hkKG".
    value["at_hash"] = json!("rXH7QWVTZnXYCou_6Vdpfg");
    value["c_hash"] = value["at_hash"].clone();
    let token = signing::token(&value, header(), 0);
    let bindings = OidcTokenBindings {
        access_token: Some("SlAV32hkKG"),
        authorization_code: Some("SlAV32hkKG"),
    };
    assert!(adapter().authenticate(&token, NONCE, bindings, NOW).is_ok());
    for bindings in [
        OidcTokenBindings::default(),
        OidcTokenBindings {
            access_token: Some("different"),
            authorization_code: Some("SlAV32hkKG"),
        },
        OidcTokenBindings {
            access_token: Some("SlAV32hkKG"),
            authorization_code: None,
        },
    ] {
        assert!(
            adapter()
                .authenticate(&token, NONCE, bindings, NOW)
                .is_err()
        );
    }
    for field in ["at_hash", "c_hash"] {
        for invalid in [json!(null), json!("bad"), json!(7)] {
            let mut malformed = value.clone();
            malformed[field] = invalid;
            assert!(
                adapter()
                    .authenticate(
                        &signing::token(&malformed, header(), 0),
                        NONCE,
                        bindings,
                        NOW
                    )
                    .is_err()
            );
        }
    }
}

#[test]
fn header_trust_and_host_context_bounds_reject_before_key_acquisition() {
    let mut forged = header();
    forged.jku = Some("https://attacker.invalid/keys".into());
    assert!(authenticate(&claims(), forged).is_err());
    let mut forged = header();
    forged.crit = Some(vec!["unknown".into()]);
    assert!(authenticate(&claims(), forged).is_err());
    let mut forged = header();
    forged.x5u = Some("https://attacker.invalid/certificate".into());
    assert!(authenticate(&claims(), forged).is_err());
    let mut missing = header();
    missing.kid = None;
    assert!(authenticate(&claims(), missing).is_err());
    let mut absent_type = header();
    absent_type.typ = None;
    assert!(authenticate(&claims(), absent_type).is_ok());
    assert_eq!(
        adapter().authenticate(
            &"x".repeat(16_385),
            NONCE,
            OidcTokenBindings::default(),
            NOW
        ),
        Err(AuthError::TooLarge)
    );
    assert!(
        adapter()
            .authenticate(
                &signing::token(&claims(), header(), 0),
                "",
                OidcTokenBindings::default(),
                NOW
            )
            .is_err()
    );
    assert!(
        OidcIdTokenAdapter::configured("", "https://issuer.example", "studio-client", Keys)
            .is_err()
    );
    assert!(
        adapter()
            .with_trusted_audiences(&["studio-client"])
            .is_err()
    );
    assert!(
        adapter()
            .with_trusted_audiences(&["other", "other"])
            .is_err()
    );
    assert!(adapter().with_trusted_audiences(&[""]).is_err());
    assert!(
        adapter()
            .with_trusted_audiences(&["1", "2", "3", "4", "5", "6", "7", "8"])
            .is_err()
    );
}

#[test]
fn declared_claim_and_configuration_limits_reject_ambiguous_values() {
    for (key, replacement) in [
        ("aud", json!(vec!["studio-client"; 9])),
        ("aud", json!("x".repeat(2049))),
        ("sub", json!("x".repeat(256))),
        ("sub", json!("control\nsubject")),
        ("nonce", json!("x".repeat(2049))),
        ("nonce", json!(7)),
        ("azp", json!("other-client")),
        ("azp", json!(null)),
        ("iat", json!(-1)),
        ("exp", json!(NOW + 3601)),
        ("exp", json!(null)),
    ] {
        let mut value = claims();
        value[key] = replacement;
        assert!(authenticate(&value, header()).is_err(), "rejected {key}");
    }
    let long = "x".repeat(2049);
    for invalid in ["", " leading", "trailing ", long.as_str()] {
        assert!(
            OidcIdTokenAdapter::configured(
                invalid,
                "https://issuer.example",
                "studio-client",
                Keys
            )
            .is_err()
        );
        assert!(
            OidcIdTokenAdapter::configured("provider", invalid, "studio-client", Keys).is_err()
        );
        assert!(
            OidcIdTokenAdapter::configured("provider", "https://issuer.example", invalid, Keys)
                .is_err()
        );
        assert!(adapter().with_trusted_audiences(&[invalid]).is_err());
    }
    let token = signing::token(&claims(), header(), 0);
    for invalid in ["", "two words", "control\nnonce", long.as_str()] {
        assert!(
            adapter()
                .authenticate(&token, invalid, OidcTokenBindings::default(), NOW)
                .is_err()
        );
    }
    // A single audience may carry a matching azp; absence of hash claims is valid for code flow.
    let mut value = claims();
    value["azp"] = json!("studio-client");
    value["exp"] = json!(NOW + 1);
    assert_eq!(
        authenticate(&value, header()).unwrap().valid_until(),
        NOW + 1
    );
}
