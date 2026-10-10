//! Shared isolated-provider Host configuration for recovery and load fixtures.
use rom_studio_host::{HostConfig, OidcProviderConfig, TrustedLoopbackBackchannel};
pub fn configured(
    issuer: &str,
    client_id: &str,
    client_secret: &str,
    actor: rom::Actor,
    blobs: rom_blob::BlobService,
) -> HostConfig {
    let mut config = HostConfig::new(
        "https://127.0.0.1:44389",
        "/rom-studio/",
        "/root/ROM/tests/identity/provider/host/assets",
        actor,
    )
    .settings("main")
    .blobs(blobs)
    .provider(OidcProviderConfig {
        authority: "authentik".into(),
        label: "Isolated recovery identity".into(),
        issuer: issuer.to_owned(),
        client_id: client_id.to_owned(),
        authorization_endpoint: "https://127.0.0.1:44392/application/o/authorize/".into(),
        token_endpoint: "https://127.0.0.1:44392/application/o/token/".into(),
        jwks_endpoint: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/jwks/".into(),
        client_secret: Some(client_secret.to_owned()),
    })
    .provider_backchannel(
        "authentik",
        TrustedLoopbackBackchannel::new(
            issuer,
            "http://127.0.0.1:44393/application/o/token/",
            "http://127.0.0.1:44393/application/o/rom-synthetic-identity/jwks/",
        ),
    );
    config.limits.sessions = 4;
    config.limits.authentication_jobs = 2;
    config.limits.acquisition_bytes = 65_536;
    config.limits.assets = 2;
    config.limits.asset_bytes = 4096;
    config
}
