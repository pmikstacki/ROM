use rom_studio_host::{HostConfig, OidcProviderConfig};

fn host_config(origin: &str, base: &str, assets: &str) -> HostConfig {
    HostConfig::new(
        origin,
        base,
        assets,
        rom::Actor::trusted("host", "configuration"),
    )
}

fn provider() -> OidcProviderConfig {
    OidcProviderConfig {
        authority: "employees".into(),
        label: "Company account".into(),
        issuer: "https://identity.example".into(),
        client_id: "studio".into(),
        authorization_endpoint: "https://identity.example/auth".into(),
        token_endpoint: "https://identity.example/token".into(),
        jwks_endpoint: "https://identity.example/jwks".into(),
        client_secret: None,
    }
}

#[test]
fn pins_exact_public_origin_and_confines_all_provider_endpoints() {
    let config = host_config(
        "https://studio.example",
        "/rom-studio/",
        "/var/lib/rom/assets",
    )
    .provider(provider());
    assert!(config.validate().is_ok());
    for origin in [
        "https://studio.example/path",
        "https://name:password@studio.example",
        "https://studio.example?x=1",
        "http://studio.example",
    ] {
        assert!(
            host_config(origin, "/rom-studio/", "/var/lib/rom/assets")
                .validate()
                .is_err(),
            "{origin}"
        );
    }
    let mut altered = provider();
    altered.token_endpoint = "https://attacker.example/token".into();
    assert!(config.clone().providers(vec![altered]).validate().is_err());
}

#[test]
fn loopback_exception_cannot_enable_plaintext_network_deployment() {
    assert!(
        host_config("http://127.0.0.1:4173", "/rom-studio/", "/assets")
            .allow_loopback_http(true)
            .validate()
            .is_ok()
    );
    for origin in [
        "http://localhost:4173",
        "http://10.66.0.2",
        "http://192.168.76.250",
    ] {
        assert!(
            host_config(origin, "/rom-studio/", "/assets")
                .allow_loopback_http(true)
                .validate()
                .is_err()
        );
    }
}

#[test]
fn rejects_ambiguous_paths_duplicate_authorities_and_zero_limits() {
    for path in [
        "//",
        "/rom/../",
        "/rom//studio/",
        "/rom/%2f/",
        "/rom?x/",
        "/rom",
    ] {
        assert!(
            host_config("https://studio.example", path, "/assets")
                .validate()
                .is_err(),
            "{path}"
        );
    }
    let config = host_config("https://studio.example", "/rom-studio/", "/assets")
        .provider(provider())
        .provider(provider());
    assert!(config.validate().is_err());
    let mut config = host_config("https://studio.example", "/rom-studio/", "/assets");
    config.limits.sessions = 0;
    assert!(config.validate().is_err());
}
