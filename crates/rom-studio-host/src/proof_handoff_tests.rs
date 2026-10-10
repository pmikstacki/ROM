//! Public Host configuration rejects impossible margins before serving requests.
use crate::{HostConfig, OidcProviderConfig};
use rom::Actor;
use std::time::Duration;
fn config() -> HostConfig {
    HostConfig::new(
        "https://studio.example",
        "/",
        "unused-by-validation",
        Actor::trusted("host", "config"),
    )
    .provider(OidcProviderConfig {
        authority: "fixture".into(),
        label: "Fixture".into(),
        issuer: "https://issuer.example".into(),
        client_id: "studio".into(),
        authorization_endpoint: "https://issuer.example/authorize".into(),
        token_endpoint: "https://issuer.example/token".into(),
        jwks_endpoint: "https://issuer.example/jwks".into(),
        client_secret: None,
    })
}
#[test]
fn proof_budget_configuration_is_positive_rounded_up_and_below_fixed_profile_ttl() {
    assert!(config().validate().is_ok());
    assert!(
        config()
            .proof_handoff_budget(Duration::ZERO)
            .validate()
            .is_err()
    );
    assert!(
        config()
            .proof_handoff_budget(Duration::from_secs(29))
            .validate()
            .is_ok()
    );
    assert!(
        config()
            .proof_handoff_budget(Duration::from_secs(30))
            .validate()
            .is_err()
    );
    assert!(
        config()
            .proof_handoff_budget(Duration::from_millis(29_001))
            .validate()
            .is_err()
    );
    assert!(
        config()
            .proof_handoff_budget(Duration::MAX)
            .validate()
            .is_err()
    );
    let mut impossible = config();
    impossible.http_limits.body_timeout = Duration::from_secs(30);
    assert!(impossible.validate().is_err());
    // A custom minimum cannot silently lower the existing body timeout allowance.
    assert!(
        impossible
            .proof_handoff_budget(Duration::from_secs(1))
            .validate()
            .is_err()
    );
}

#[test]
fn no_oidc_provider_does_not_impose_an_oidc_timeout_ceiling() {
    let mut local = HostConfig::new(
        "https://studio.example",
        "/",
        "unused-by-validation",
        Actor::trusted("host", "config"),
    );
    local.http_limits.body_timeout = Duration::from_secs(60);
    assert!(local.validate().is_ok());
}
