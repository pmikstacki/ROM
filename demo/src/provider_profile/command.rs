//! Private host inputs and thin local command composition.
use super::provisioning::bounded;
use super::{
    ApprovedProvider, AuthLimits, HostAuth, LocalMode, Provisioning, SecretFiles, build,
    configuration_reader, maintain, provision, secrets::read_private, serve,
};
use rom::{Error, Result, Runtime, Storage, Value};
use rom_auth::introspection::EndpointPolicy;
use rom_identity::{IdentityProvider, ProviderProfile, User};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// Private bounded host configuration. Paths and material have no Debug output.
pub struct ProfileConfig {
    pub provisioning: Provisioning,
    pub approved: ApprovedProvider,
    pub secrets: SecretFiles,
    pub endpoint_policy: EndpointPolicy,
    pub auth: AuthLimits,
}
impl ProfileConfig {
    /// Read one private regular file, at most 64 KiB. Unknown and duplicate keys fail.
    /// Relative approved secret paths are resolved against this file's directory.
    pub fn load(path: &Path) -> Result<Self> {
        let value = read_document(path, 65_536)?;
        let root = object(
            &value,
            &[
                "version",
                "provider",
                "user",
                "service_subject",
                "secrets",
                "endpoint_policy",
                "auth",
            ],
        )?;
        if root.get("version").and_then(Value::as_u64) != Some(1) {
            return Err(Error::Denied);
        }
        let provider = object(
            required(root, "provider")?,
            &[
                "authority",
                "issuer",
                "audience",
                "endpoint",
                "introspection_client",
                "credential_ref",
            ],
        )?;
        let approved = ApprovedProvider {
            authority: text(provider, "authority", 2048)?,
            issuer: text(provider, "issuer", 2048)?,
            audience: text(provider, "audience", 2048)?,
            endpoint: text(provider, "endpoint", 2048)?,
            introspection_client: text(provider, "introspection_client", 2048)?,
        };
        approved.validate()?;
        let reference = text(provider, "credential_ref", 2048)?;
        let user = object(required(root, "user")?, &["id", "display_name"])?;
        let provisioning = Provisioning {
            provider_id: approved.authority.clone(),
            provider: IdentityProvider {
                enabled: true,
                profile: ProviderProfile::OAuthIntrospectionService,
                issuer: approved.issuer.clone(),
                audience: approved.audience.clone(),
                endpoint: Some(approved.endpoint.clone()),
                credential_ref: Some(reference.clone()),
            },
            user_id: text(user, "id", 2048)?,
            user: User {
                enabled: true,
                display_name: text(user, "display_name", 2048)?,
            },
            service_subject: text(root, "service_subject", 2048)?,
        };
        provisioning.validate()?;
        let entries = required(root, "secrets")?
            .as_object()
            .ok_or(Error::Denied)?;
        if entries.is_empty() || entries.len() > 8 {
            return Err(Error::Denied);
        }
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let mut files = BTreeMap::new();
        for (key, value) in entries {
            bounded(key, 2048)?;
            let value = value.as_str().ok_or(Error::Denied)?;
            bounded(value, 4096)?;
            files.insert(key.clone(), parent.join(value));
        }
        if !files.contains_key(&reference) {
            return Err(Error::Denied);
        }
        let secrets = SecretFiles::new(files)?;
        let endpoint_policy = match root.get("endpoint_policy") {
            None => EndpointPolicy::HttpsOnly,
            Some(v) => match v.as_str() {
                Some("https-only") => EndpointPolicy::HttpsOnly,
                Some("loopback-test-only") => EndpointPolicy::LoopbackTestOnly,
                _ => return Err(Error::Denied),
            },
        };
        let auth = match root.get("auth") {
            None => AuthLimits::default(),
            Some(v) => {
                let auth = object(v, &["jobs", "response_timeout_ms"])?;
                AuthLimits {
                    jobs: usize::try_from(required(auth, "jobs")?.as_u64().ok_or(Error::Denied)?)
                        .map_err(|_| Error::Denied)?,
                    response_timeout: Duration::from_millis(
                        required(auth, "response_timeout_ms")?
                            .as_u64()
                            .ok_or(Error::Denied)?,
                    ),
                }
            }
        };
        auth.validate()?;
        Ok(Self {
            provisioning,
            approved,
            secrets,
            endpoint_policy,
            auth,
        })
    }
}
fn object<'a>(value: &'a Value, allowed: &[&str]) -> Result<&'a rom::Map<String, Value>> {
    let map = value.as_object().ok_or(Error::Denied)?;
    if map.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(Error::Denied);
    }
    Ok(map)
}
fn required<'a>(map: &'a rom::Map<String, Value>, key: &str) -> Result<&'a Value> {
    map.get(key).ok_or(Error::Denied)
}
fn text(map: &rom::Map<String, Value>, key: &str, limit: usize) -> Result<String> {
    let value = required(map, key)?.as_str().ok_or(Error::Denied)?;
    bounded(value, limit)?;
    Ok(value.into())
}
fn read_document(path: &Path, bound: usize) -> Result<Value> {
    let source = read_private(path, bound)?;
    let parsed = rom_config::parse(rom_config::Format::Json, &source, "provider_profile")
        .map_err(|_| Error::Denied)?;
    Ok(parsed.values().clone())
}
/// Local commands accept paths only. Credential material never becomes arguments.
/// Serving uses the current provider Resource; it never provisions automatically.
pub async fn run_command(mode: &str, redb: bool, args: Vec<String>) -> Result<()> {
    let expected = if mode == "provider-maintain" { 3 } else { 2 };
    if args.len() < expected
        || args.len() > expected + usize::from(mode == "provider-serve")
        || !matches!(
            mode,
            "provider-provision" | "provider-maintain" | "provider-serve"
        )
    {
        return Err(Error::Denied);
    }
    let config = ProfileConfig::load(Path::new(&args[1]))?;
    let invocation = if mode == "provider-maintain" {
        Some(
            serde_json::from_value(read_document(Path::new(&args[2]), 16_384)?)
                .map_err(|_| Error::Denied)?,
        )
    } else {
        None
    };
    let port = if mode == "provider-serve" {
        args.get(2)
            .map(|v| v.parse::<u16>().map_err(|_| Error::Denied))
            .transpose()?
            .unwrap_or(8080)
    } else {
        0
    };
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(PathBuf::from(&args[0]))?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(PathBuf::from(&args[0]))?)
    };
    let clock: Arc<dyn rom::Clock> = Arc::new(rom::SystemClock);
    let runtime = build(
        storage,
        clock.clone(),
        &config.provisioning,
        if mode == "provider-provision" {
            LocalMode::Provisioning
        } else {
            LocalMode::Serving
        },
    )?;
    let result = async {
        match mode {
            "provider-provision" => {
                provision(&runtime, &config.provisioning).await?;
                println!("{{\"provisioned\":true}}");
                Ok(())
            }
            "provider-maintain" => {
                let row = maintain(
                    &runtime,
                    &config.provisioning,
                    invocation.ok_or(Error::Denied)?,
                )
                .await?;
                println!("{{\"revision\":{}}}", row.revision);
                Ok(())
            }
            _ => serve_command(runtime.clone(), clock, config, port).await,
        }
    }
    .await;
    let shutdown = runtime.shutdown().await;
    result.and(shutdown)
}
async fn serve_command(
    runtime: Runtime,
    clock: Arc<dyn rom::Clock>,
    config: ProfileConfig,
    port: u16,
) -> Result<()> {
    let auth = HostAuth::new(
        runtime.clone(),
        configuration_reader(),
        clock,
        config.approved,
        config.secrets,
        config.endpoint_policy,
        config.auth,
    )?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|_| Error::Storage)?;
    let address = listener.local_addr().map_err(|_| Error::Storage)?;
    // Install signal reception before reporting readiness.
    let mut termination = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| Error::Storage)?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|_| Error::Storage)?;
    println!("{{\"endpoint\":\"http://{address}\"}}");
    serve(runtime, auth, listener, async move {
        tokio::select! {_ = interrupt.recv()=>{},_ = termination.recv()=>{}}
    })
    .await
}
