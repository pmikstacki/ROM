//! OAuth introspection using a host-configured endpoint and explicit service profile.
//!
//! ```no_run
//! use rom_auth::{AuthError, VerifiedIdentity};
//! use rom_auth::introspection::{EndpointPolicy, IntrospectionAdapter};
//! fn verify(client_secret: &str, token: &str, now: u64) -> Result<VerifiedIdentity, AuthError> {
//!     let mut adapter = IntrospectionAdapter::configured("automation",
//!         "https://issuer.example", "rom-api", "https://issuer.example/introspect",
//!         "host-client", client_secret, EndpointPolicy::HttpsOnly)?;
//!     adapter.authenticate(token, now)
//! }
//! ```
//! Reuse the configured adapter in a real host to retain its bounded evidence cache.
use crate::{AuthError, PrincipalKind, VerifiedIdentity};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, time::Duration};

/// Explicit transport policy; insecure transport is confined to deliberate local fixtures.
#[derive(Clone, Copy, Debug)]
pub enum EndpointPolicy {
    /// Require HTTPS for a configured provider.
    HttpsOnly,
    /// Permit only HTTP on a numeric loopback address, for local test fixtures.
    LoopbackTestOnly,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}
impl Audience {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(x) => x == expected,
            Self::Many(xs) => xs.iter().any(|x| x == expected),
        }
    }
}
#[derive(Deserialize)]
struct IntrospectionResponse {
    active: bool,
    #[serde(default, deserialize_with = "crate::present_claim")]
    sub: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    iss: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    aud: Option<Audience>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    exp: Option<u64>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    nbf: Option<u64>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    iat: Option<u64>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    client_id: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    principal_kind: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    token_type: Option<String>,
    #[serde(default, deserialize_with = "crate::present_claim")]
    cnf: Option<serde_json::Value>,
}
/// Blocking authenticated introspection with five-second, expiry-bounded evidence cache.
///
/// Requires the explicit service profile documented on [`Self::authenticate`].
/// Execute outside async runtime workers. No secret-bearing Debug implementation.
pub struct IntrospectionAdapter {
    authority: String,
    expected_issuer: String,
    audience: String,
    endpoint: reqwest::Url,
    client_id: String,
    client_secret: String,
    http: reqwest::blocking::Client,
    cache: BTreeMap<[u8; 32], VerifiedIdentity>,
}
impl IntrospectionAdapter {
    /// Configure host-approved issuer, resource audience, endpoint and client credentials.
    ///
    /// HTTPS is required by `HttpsOnly`; the test policy accepts numeric loopback HTTP only.
    /// HTTP redirects and automatic proxies are disabled. Connect/request timeouts are
    /// 250/500 ms, and the decoded response is limited to 16 KiB.
    pub fn configured(
        authority: &str,
        issuer: &str,
        audience: &str,
        endpoint: &str,
        client_id: &str,
        secret: &str,
        endpoint_policy: EndpointPolicy,
    ) -> Result<Self, AuthError> {
        if ![authority, issuer, audience]
            .into_iter()
            .all(crate::valid_name)
            || client_id.is_empty()
            || client_id.len() > 2048
            || secret.is_empty()
            || secret.len() > 4096
        {
            return Err(AuthError::InvalidConfiguration);
        }
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| AuthError::UnsafeEndpoint)?;
        let loopback = endpoint
            .host_str()
            .and_then(|s| s.parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !match endpoint_policy {
            EndpointPolicy::HttpsOnly => endpoint.scheme() == "https",
            EndpointPolicy::LoopbackTestOnly => endpoint.scheme() == "http" && loopback,
        } {
            return Err(AuthError::UnsafeEndpoint);
        }
        if !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(AuthError::UnsafeEndpoint);
        }
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_millis(500))
            .connect_timeout(Duration::from_millis(250))
            .build()
            .map_err(|_| AuthError::Unavailable)?;
        Ok(Self {
            authority: authority.into(),
            expected_issuer: issuer.into(),
            audience: audience.into(),
            endpoint,
            client_id: client_id.into(),
            client_secret: secret.into(),
            http,
            cache: BTreeMap::new(),
        })
    }
    /// Discard cached evidence for subsequent verification. Already returned proofs
    /// remain bounded by their expiry; the host must separately invalidate actors.
    pub fn invalidate(&mut self) {
        self.cache.clear();
    }
    /// Number of retained credential digests/evidence entries (at most eight).
    pub fn cache_entries(&self) -> usize {
        self.cache.len()
    }
    /// Verify an opaque bearer with trusted host Unix time.
    ///
    /// Requires active=true, exact issuer, matching audience, expiry, Bearer type,
    /// `principal_kind=service` and nonempty `sub=client_id`. Rejects `cnf` proof
    /// requirements. These additional requirements define this supported profile,
    /// not every RFC 7662 provider. Provider claims and token bytes are not retained.
    pub fn authenticate(&mut self, token: &str, now: u64) -> Result<VerifiedIdentity, AuthError> {
        if token.is_empty() || token.len() > 4096 {
            return Err(AuthError::TooLarge);
        }
        let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        if let Some(actor) = self.cache.get(&digest).filter(|a| now < a.valid_until()) {
            return Ok(actor.clone());
        }
        self.cache.remove(&digest);
        let response = self
            .http
            .post(self.endpoint.clone())
            .basic_auth(
                form_urlencoded::byte_serialize(self.client_id.as_bytes()).collect::<String>(),
                Some(
                    form_urlencoded::byte_serialize(self.client_secret.as_bytes())
                        .collect::<String>(),
                ),
            )
            .form(&[("token", token), ("token_type_hint", "access_token")])
            .send()
            .map_err(|_| AuthError::Unavailable)?;
        if !response.status().is_success() {
            return Err(AuthError::Unavailable);
        }
        let mut bytes = Vec::new();
        response
            .take(16_385)
            .read_to_end(&mut bytes)
            .map_err(|_| AuthError::Unavailable)?;
        if bytes.len() > 16_384 {
            return Err(AuthError::TooLarge);
        }
        let r: IntrospectionResponse =
            serde_json::from_slice(&bytes).map_err(|_| AuthError::Invalid)?;
        if !r.active {
            return Err(AuthError::Inactive);
        }
        if r.iss.as_deref() != Some(&self.expected_issuer)
            || !r
                .aud
                .as_ref()
                .is_some_and(|aud| aud.contains(&self.audience))
        {
            return Err(AuthError::Binding);
        }
        let exp = r.exp.ok_or(AuthError::Expired)?;
        if exp <= now || r.nbf.is_some_and(|v| v > now) || r.iat.is_some_and(|v| v > now) {
            return Err(AuthError::Expired);
        }
        let sub = r
            .sub
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or(AuthError::WrongProfile)?;
        // Explicitly supported provider profile: service sub equals verified client_id.
        if r.principal_kind.as_deref() != Some("service")
            || r.client_id.as_deref() != Some(&sub)
            || r.token_type.as_deref() != Some("Bearer")
            || r.cnf.is_some()
        {
            return Err(AuthError::WrongProfile);
        }
        let actor = VerifiedIdentity::verified(
            &self.authority,
            sub,
            PrincipalKind::Service,
            exp.min(now.saturating_add(5)),
        );
        // Hard entry bound, deliberately simple flush eviction for bounded storage.
        if self.cache.len() >= 8 {
            self.cache.clear();
        }
        self.cache.insert(digest, actor.clone());
        Ok(actor)
    }
}
