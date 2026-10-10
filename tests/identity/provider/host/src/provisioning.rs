use rom::{Actor, Command, Resource, Runtime};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::StudioSettings;
use serde::Deserialize;
use std::{io::Read, sync::Arc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub adapter: String,
    pub assets_directory: String,
    pub database: String,
    pub stop_file: String,
    pub control_directory: Option<String>,
    pub public_origin: Option<String>,
    pub private_token_endpoint: Option<String>,
    pub private_jwks_endpoint: Option<String>,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_endpoint: String,
    pub verified_synthetic_subject: String,
}
pub fn read(path: &str) -> Result<Configuration, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(65_537)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 65_536 {
        return Err("bounded host configuration exceeded".into());
    }
    let configuration: Configuration = serde_json::from_slice(&bytes)?;
    let _ = approved_origin(&configuration)?;
    if configuration.assets_directory != "/root/ROM/tests/identity/provider/host/assets" {
        return Err("explicit owned authoring asset path required".into());
    }
    for path in [&configuration.database, &configuration.stop_file] {
        if !path.starts_with("/var/tmp/rom-010-authentik-20261007/run/volume/private/")
            || path.split('/').any(|part| part == "..")
        {
            return Err("private fixture path required".into());
        }
    }
    if std::path::Path::new(&configuration.database).exists()
        || std::path::Path::new(&configuration.stop_file).exists()
    {
        return Err("fresh disposable host paths required".into());
    }
    if let Some(directory) = &configuration.control_directory {
        let _ = crate::control::Mailbox::new(directory)?;
    }
    Ok(configuration)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportProfile {
    HttpLoopback,
    TerminatedLoopback,
    NativeTls,
}

/// Closed fixture profiles. Native TLS never installs an HTTP backchannel.
pub fn transport_profile(configuration: &Configuration) -> rom::Result<TransportProfile> {
    let endpoints = (
        configuration.private_token_endpoint.as_deref(),
        configuration.private_jwks_endpoint.as_deref(),
    );
    match (configuration.public_origin.as_deref(), endpoints) {
        (None | Some("http://127.0.0.1:44391"), (None, None)) => {
            Ok(TransportProfile::HttpLoopback)
        }
        (Some("https://127.0.0.1:44389"), (None, None)) => Ok(TransportProfile::NativeTls),
        (
            Some("https://127.0.0.1:44389"),
            (
                Some("http://127.0.0.1:44393/application/o/token/"),
                Some("http://127.0.0.1:44393/application/o/rom-synthetic-identity/jwks/"),
            ),
        ) => Ok(TransportProfile::TerminatedLoopback),
        _ => Err(rom::Error::Denied),
    }
}

pub fn approved_origin(configuration: &Configuration) -> rom::Result<&str> {
    let profile = transport_profile(configuration)?;
    let origin = configuration
        .public_origin
        .as_deref()
        .unwrap_or("http://127.0.0.1:44391");
    let provider = match profile {
        TransportProfile::HttpLoopback => "http://127.0.0.1:44390",
        TransportProfile::TerminatedLoopback | TransportProfile::NativeTls => {
            "https://127.0.0.1:44392"
        }
    };
    if configuration.issuer != format!("{provider}/application/o/rom-synthetic-identity/")
        || configuration.authorization_endpoint != format!("{provider}/application/o/authorize/")
        || configuration.token_endpoint != format!("{provider}/application/o/token/")
        || configuration.jwks_endpoint
            != format!("{provider}/application/o/rom-synthetic-identity/jwks/")
    {
        return Err(rom::Error::Denied);
    }
    Ok(origin)
}
pub fn runtime(configuration: &Configuration) -> rom::Result<(Runtime, Actor)> {
    let storage: Arc<dyn rom::Storage> = match configuration.adapter.as_str() {
        "sqlite" => Arc::new(rom_sqlite::Sqlite::open(&configuration.database)?),
        "redb" => Arc::new(rom_redb::Redb::open(&configuration.database)?),
        _ => return Err(rom::Error::Denied),
    };
    let actor = Actor::trusted("fixture-host", "configuration");
    let gate = IdentityGate::default().allow_host(
        "fixture-host",
        rom::PrincipalKind::Embedded,
        "configuration",
    )?;
    let runtime = Runtime::builder()
        .actor_gate(Arc::new(gate))
        .resource(crate::protected::definition())
        .resource(
            User::definition()
                .policy(|actor, _, _| actor.authority == "fixture-host")
                .allow_all_fields(),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|actor, _, _| actor.authority == "fixture-host")
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|actor, _, _| actor.authority == "fixture-host")
                .allow_all_fields(),
        )
        .resource(
            StudioSettings::definition()
                .policy(|actor, _, _| actor.authority == "fixture-host")
                .allow_all_fields(),
        )
        .build(storage, Runtime::shared_cpu_pool(2)?)?;
    Ok((runtime, actor))
}
pub async fn seed(
    runtime: &Runtime,
    actor: &Actor,
    configuration: &Configuration,
) -> rom::Result<()> {
    runtime
        .execute(
            actor,
            Command::create(
                "private",
                crate::protected::ProtectedDocument {
                    content: "Synthetic protected fixture document".into(),
                },
            )
            .idempotency("seed-protected"),
        )
        .await?;
    runtime
        .execute(
            actor,
            Command::create(
                "fixture-user",
                User {
                    enabled: true,
                    display_name: "Synthetic identity fixture".into(),
                },
            )
            .idempotency("seed-user"),
        )
        .await?;
    runtime
        .execute(
            actor,
            Command::create(
                "authentik",
                IdentityProvider {
                    enabled: true,
                    profile: ProviderProfile::OidcRs256Human,
                    issuer: configuration.issuer.clone(),
                    audience: configuration.client_id.clone(),
                    endpoint: None,
                    credential_ref: None,
                },
            )
            .idempotency("seed-provider"),
        )
        .await?;
    runtime
        .execute(
            actor,
            Command::create(
                &link_key(
                    "authentik",
                    rom::PrincipalKind::Human,
                    &configuration.verified_synthetic_subject,
                ),
                IdentityLink {
                    authority: "authentik".into(),
                    subject: configuration.verified_synthetic_subject.clone(),
                    principal_kind: "human".into(),
                    user_id: "fixture-user".into(),
                    enabled: true,
                },
            )
            .idempotency("seed-link"),
        )
        .await?;
    runtime
        .execute(
            actor,
            Command::create(
                "main",
                StudioSettings {
                    primary_provider: Some("authentik".into()),
                },
            )
            .idempotency("seed-settings"),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "native_tls_tests.rs"]
mod native_tls_tests;
