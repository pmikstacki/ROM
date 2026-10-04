use crate::{HostConfig, OidcProviderConfig, TrustedLoopbackBackchannel};

#[test]
fn internal_routing_changes_neither_public_claims_nor_other_provider_routes() {
    let provider = OidcProviderConfig {
        authority: "demo".into(),
        label: "Demo only".into(),
        issuer: "https://studio.example/idp".into(),
        client_id: "studio".into(),
        authorization_endpoint: "https://studio.example/idp/auth".into(),
        token_endpoint: "https://studio.example/idp/token".into(),
        jwks_endpoint: "https://studio.example/idp/jwks".into(),
        client_secret: None,
    };
    let config = HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        "/assets",
        rom::Actor::trusted("host", "configuration"),
    )
    .provider(provider.clone())
    .provider_backchannel(
        "demo",
        TrustedLoopbackBackchannel::new(
            &provider.issuer,
            "http://127.0.0.1:44174/idp/token",
            "http://127.0.0.1:44174/idp/jwks",
        ),
    );
    assert!(config.validate().is_ok());
    assert_eq!(
        config.token_endpoint(&provider),
        "http://127.0.0.1:44174/idp/token"
    );
    assert_eq!(
        config.jwks_endpoint(&provider),
        "http://127.0.0.1:44174/idp/jwks"
    );
    assert_eq!(config.approved[0].issuer, "https://studio.example/idp");
    assert_eq!(
        config.callback("demo"),
        "https://studio.example/rom-studio/auth/callback/demo"
    );
    let mut other = provider.clone();
    other.authority = "other".into();
    assert_eq!(config.token_endpoint(&other), provider.token_endpoint);
    assert_eq!(config.jwks_endpoint(&other), provider.jwks_endpoint);
}
