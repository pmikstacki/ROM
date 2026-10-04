use rom::{Actor, Error, Result};
use std::{collections::BTreeSet, path::PathBuf, time::Duration};
use url::Url;
type BlobStoreDisclosure = std::sync::Arc<dyn Fn(&Actor, &str) -> bool + Send + Sync>;

/// Explicit host-approved OIDC network sources. Never deserialize this from browser input.
#[derive(Clone)]
pub struct OidcProviderConfig {
    pub authority: String,
    pub label: String,
    pub issuer: String,
    pub client_id: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_endpoint: String,
    pub client_secret: Option<String>,
}

/// Finite host admission, credential, network, and static asset limits.
#[derive(Clone)]
pub struct HostLimits {
    pub sessions: usize,
    pub session_seconds: u64,
    pub login_attempts: usize,
    pub login_seconds: u64,
    pub authentication_jobs: usize,
    pub acquisition_timeout: Duration,
    pub acquisition_bytes: usize,
    pub assets: usize,
    pub asset_bytes: usize,
    pub observation_poll: Duration,
}
impl Default for HostLimits {
    fn default() -> Self {
        Self {
            sessions: 256,
            session_seconds: 3600,
            login_attempts: 64,
            login_seconds: 120,
            authentication_jobs: 8,
            acquisition_timeout: Duration::from_secs(5),
            acquisition_bytes: 65_536,
            assets: 256,
            asset_bytes: 64 * 1024 * 1024,
            observation_poll: Duration::from_millis(100),
        }
    }
}

/// Host-only approval. Secrets are deliberately excluded from Debug and serialization.
#[derive(Clone)]
pub struct HostConfig {
    pub(crate) public_origin: String,
    pub(crate) base_path: String,
    pub(crate) asset_directory: PathBuf,
    pub(crate) host_actor: Actor,
    pub(crate) approved: Vec<OidcProviderConfig>,
    pub(crate) primary: Option<String>,
    pub(crate) settings_id: Option<String>,
    pub(crate) blobs: Option<rom_blob::BlobService>,
    pub(crate) blob_store_discovery: BlobStoreDisclosure,
    pub(crate) loopback_http: bool,
    pub limits: HostLimits,
    pub http_limits: rom_http::Limits,
    pub(crate) clock: std::sync::Arc<dyn rom::Clock>,
}
impl HostConfig {
    pub fn new(
        origin: &str,
        base_path: &str,
        assets: impl Into<PathBuf>,
        host_actor: Actor,
    ) -> Self {
        Self {
            public_origin: origin.into(),
            base_path: base_path.into(),
            asset_directory: assets.into(),
            host_actor,
            approved: Vec::new(),
            primary: None,
            settings_id: None,
            blobs: None,
            blob_store_discovery: std::sync::Arc::new(|_, _| false),
            loopback_http: false,
            limits: HostLimits::default(),
            http_limits: rom_http::Limits::default(),
            clock: std::sync::Arc::new(rom::SystemClock),
        }
    }
    pub fn clock(mut self, clock: std::sync::Arc<dyn rom::Clock>) -> Self {
        self.clock = clock;
        self
    }
    pub fn provider(mut self, provider: OidcProviderConfig) -> Self {
        self.approved.push(provider);
        self
    }
    pub fn providers(mut self, providers: Vec<OidcProviderConfig>) -> Self {
        self.approved = providers;
        self
    }
    pub fn settings(mut self, id: &str) -> Self {
        self.settings_id = Some(id.into());
        self
    }
    /// Add supervised binary transport for the same Resource runtime.
    pub fn blobs(mut self, service: rom_blob::BlobService) -> Self {
        self.blobs = Some(service);
        self
    }
    /// Explicit metadata disclosure for configured store names, default denied.
    /// This does not authorize reservation or other Resource mutations.
    pub fn blob_store_discovery<P>(mut self, policy: P) -> Self
    where
        P: Fn(&Actor, &str) -> bool + Send + Sync + 'static,
    {
        self.blob_store_discovery = std::sync::Arc::new(policy);
        self
    }
    pub fn primary(mut self, authority: &str) -> Self {
        self.primary = Some(authority.into());
        self
    }
    pub fn allow_loopback_http(mut self, allow: bool) -> Self {
        self.loopback_http = allow;
        self
    }
    pub fn validate(&self) -> Result<()> {
        let origin = approved_url(&self.public_origin, self.loopback_http)?;
        if origin.origin().ascii_serialization() != self.public_origin
            || origin.path() != "/"
            || origin.query().is_some()
        {
            return Err(invalid("Studio origin must be exact"));
        }
        if !self.base_path.starts_with('/')
            || !self.base_path.ends_with('/')
            || self.base_path.contains("//")
            || self.base_path != "/"
                && self.base_path[1..self.base_path.len() - 1]
                    .split('/')
                    .any(|part| !path_segment(part))
        {
            return Err(invalid("Studio base path must be canonical"));
        }
        let l = &self.limits;
        if [
            l.sessions,
            l.login_attempts,
            l.authentication_jobs,
            l.acquisition_bytes,
            l.assets,
            l.asset_bytes,
        ]
        .contains(&0)
            || l.session_seconds == 0
            || l.login_seconds == 0
            || l.acquisition_timeout.is_zero()
            || l.observation_poll.is_zero()
            || l.sessions > 65_536
            || l.login_attempts > 4096
            || l.authentication_jobs > 256
            || l.acquisition_bytes > 1024 * 1024
            || l.session_seconds > 3600
            || l.login_seconds > 600
        {
            return Err(invalid("Studio limits must be finite and bounded"));
        }
        let mut authorities = BTreeSet::new();
        if self.approved.len() > 32 {
            return Err(Error::TooLarge);
        }
        for provider in &self.approved {
            if !path_segment(&provider.authority)
                || provider.label.is_empty()
                || provider.label.len() > 256
                || provider.client_id.is_empty()
                || provider.client_id.len() > 2048
                || !authorities.insert(&provider.authority)
                || provider
                    .client_secret
                    .as_ref()
                    .is_some_and(|s| s.is_empty() || s.len() > 4096)
            {
                return Err(invalid("Invalid Studio provider approval"));
            }
            let issuer = approved_url(&provider.issuer, self.loopback_http)?;
            if issuer.query().is_some() {
                return Err(invalid("Issuer cannot contain a query"));
            }
            for endpoint in [
                &provider.authorization_endpoint,
                &provider.token_endpoint,
                &provider.jwks_endpoint,
            ] {
                let endpoint = approved_url(endpoint, self.loopback_http)?;
                if endpoint.origin() != issuer.origin() || endpoint.query().is_some() {
                    return Err(invalid("OIDC endpoint must match approved issuer origin"));
                }
            }
        }
        if self
            .primary
            .as_ref()
            .is_some_and(|id| !authorities.contains(id))
        {
            return Err(invalid("Primary provider must be approved"));
        }
        Ok(())
    }
    pub(crate) fn secure(&self) -> bool {
        self.public_origin.starts_with("https://")
    }
    pub(crate) fn callback(&self, authority: &str) -> String {
        format!(
            "{}{}auth/callback/{}",
            self.public_origin, self.base_path, authority
        )
    }
}
fn approved_url(value: &str, loopback: bool) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| invalid("Invalid approved URL"))?;
    let local = matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_loopback())
        || matches!(url.host(), Some(url::Host::Ipv6(ip)) if ip.is_loopback());
    if value.len() > 2048
        || url.username() != ""
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host().is_none()
        || !(url.scheme() == "https" || url.scheme() == "http" && loopback && local)
    {
        return Err(invalid("URL requires HTTPS or explicit numeric loopback"));
    }
    Ok(url)
}
fn path_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn invalid(field: &str) -> Error {
    Error::Invalid {
        kind: "studio-host".into(),
        field: field.into(),
    }
}
