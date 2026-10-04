use rom_studio_host::{HostConfig, OidcProviderConfig, TrustedLoopbackBackchannel};
fn config() -> HostConfig {
    HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        "/assets",
        rom::Actor::trusted("host", "configuration"),
    )
    .provider(OidcProviderConfig {
        authority: "demo".into(),
        label: "Demo only".into(),
        issuer: "https://studio.example/identity".into(),
        client_id: "studio".into(),
        authorization_endpoint: "https://studio.example/identity/auth".into(),
        token_endpoint: "https://studio.example/identity/token".into(),
        jwks_endpoint: "https://studio.example/identity/jwks".into(),
        client_secret: None,
    })
}
fn profile(issuer: &str, token: &str, jwks: &str) -> TrustedLoopbackBackchannel {
    TrustedLoopbackBackchannel::new(issuer, token, jwks)
}
#[test]
fn trusted_backchannel_keeps_public_https_identity() {
    assert!(
        config()
            .provider_backchannel(
                "demo",
                profile(
                    "https://studio.example/identity",
                    "http://127.0.0.1:44174/identity/token",
                    "http://127.0.0.1:44174/identity/jwks"
                )
            )
            .validate()
            .is_ok()
    );
}
#[test]
fn rejects_unbound_or_ambiguous_backchannels() {
    for (authority, issuer, token, jwks) in [
        (
            "unknown",
            "https://studio.example/identity",
            "http://127.0.0.1:44174/token",
            "http://127.0.0.1:44174/jwks",
        ),
        (
            "demo",
            "https://other.example",
            "http://127.0.0.1:44174/token",
            "http://127.0.0.1:44174/jwks",
        ),
        (
            "demo",
            "https://studio.example/identity",
            "http://localhost:44174/token",
            "http://127.0.0.1:44174/jwks",
        ),
        (
            "demo",
            "https://studio.example/identity",
            "http://10.66.0.2:44174/token",
            "http://10.66.0.2:44174/jwks",
        ),
        (
            "demo",
            "https://studio.example/identity",
            "http://127.0.0.1:44174/token?x=1",
            "http://127.0.0.1:44174/jwks",
        ),
        (
            "demo",
            "https://studio.example/identity",
            "http://127.0.0.1:44174/token",
            "http://127.0.0.1:44175/jwks",
        ),
        (
            "demo",
            "https://studio.example/identity",
            "https://127.0.0.1:44174/token",
            "https://127.0.0.1:44174/jwks",
        ),
    ] {
        assert!(
            config()
                .provider_backchannel(authority, profile(issuer, token, jwks))
                .validate()
                .is_err(),
            "{authority} {issuer} {token} {jwks}"
        );
    }
    let p = profile(
        "https://studio.example/identity",
        "http://127.0.0.1:44174/token",
        "http://127.0.0.1:44174/jwks",
    );
    assert!(
        config()
            .provider_backchannel("demo", p.clone())
            .provider_backchannel("demo", p)
            .validate()
            .is_err()
    );
}
