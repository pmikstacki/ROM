//! DISPOSABLE verification profiles. Tokens never enter core state or error strings.
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use rom_auth_probe_core::{Actor, ActorIssuer, PrincipalKind};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    Invalid,
    TooLarge,
    WrongProfile,
    UnknownKey,
    RefreshLimited,
    Unavailable,
    Inactive,
    Expired,
    Binding,
    UnsafeEndpoint,
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
struct JwtClaims {
    sub: String,
    exp: u64,
    iat: u64,
    jti: String,
    client_id: String,
    principal_kind: String,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    cnf: Option<serde_json::Value>,
}

/// A trusted, host-selected key acquisition seam. This probe supplies in-memory public keys.
/// Network JWKS fetch/discovery and its timeout protocol are intentionally not implemented.
pub trait TrustedKeys {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError>;
}
pub struct JwtAdapter<K> {
    issuer: ActorIssuer,
    validation: Validation,
    source: K,
    keys: BTreeMap<String, DecodingKey>,
    keys_until: u64,
    last_refresh: Option<u64>,
}
impl<K: TrustedKeys> JwtAdapter<K> {
    pub fn configured(authority: &str, expected_issuer: &str, audience: &str, source: K) -> Self {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[expected_issuer]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp"]);
        validation.leeway = 0;
        validation.validate_nbf = true;
        Self {
            issuer: ActorIssuer::configured_by_host(authority, "tenant-a"),
            validation,
            source,
            keys: BTreeMap::new(),
            keys_until: 0,
            last_refresh: None,
        }
    }
    pub fn authenticate(&mut self, token: &str, now: u64) -> Result<Actor, AuthError> {
        if token.len() > 16_384 {
            return Err(AuthError::TooLarge);
        }
        let header = decode_header(token).map_err(|_| AuthError::Invalid)?;
        // Unverified header can select only a key from the configured source; it grants no trust.
        if header.alg != Algorithm::RS256
            || !matches!(header.typ.as_deref(), Some("at+jwt" | "application/at+jwt"))
        {
            return Err(AuthError::WrongProfile);
        }
        if header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
            || header.crit.as_ref().is_some_and(|c| !c.is_empty())
        {
            return Err(AuthError::WrongProfile);
        }
        let kid = header
            .kid
            .as_deref()
            .filter(|x| !x.is_empty() && x.len() <= 64)
            .ok_or(AuthError::UnknownKey)?;
        if now >= self.keys_until || !self.keys.contains_key(kid) {
            if self
                .last_refresh
                .is_some_and(|last| now < last.saturating_add(5))
            {
                return Err(AuthError::RefreshLimited);
            }
            self.last_refresh = Some(now);
            let fresh = self.source.fetch()?;
            if fresh.is_empty()
                || fresh.len() > 8
                || fresh.keys().any(|k| k.is_empty() || k.len() > 64)
            {
                return Err(AuthError::Invalid);
            }
            self.keys = fresh;
            self.keys_until = now.saturating_add(30);
        }
        let key = self.keys.get(kid).ok_or(AuthError::UnknownKey)?;
        let claims = decode::<JwtClaims>(token, key, &self.validation)
            .map_err(|_| AuthError::Invalid)?
            .claims;
        if claims.exp <= now
            || claims.nbf.is_some_and(|nbf| nbf > now)
            || claims.iat > now
            || claims.exp.saturating_sub(claims.iat) > 3600
        {
            return Err(AuthError::Expired);
        }
        if claims.sub.is_empty()
            || claims.sub.len() > 256
            || claims.jti.is_empty()
            || claims.client_id.is_empty()
            || claims.principal_kind != "human"
            || claims.cnf.is_some()
        {
            return Err(AuthError::WrongProfile);
        }
        Ok(self.issuer.verified(
            claims.sub,
            PrincipalKind::Human,
            claims.exp.min(self.keys_until).min(now.saturating_add(30)),
        ))
    }
}

#[derive(Deserialize)]
struct IntrospectionResponse {
    active: bool,
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    iss: Option<String>,
    #[serde(default)]
    aud: Option<Audience>,
    #[serde(default)]
    exp: Option<u64>,
    #[serde(default)]
    nbf: Option<u64>,
    #[serde(default)]
    iat: Option<u64>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    principal_kind: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    cnf: Option<serde_json::Value>,
}
pub struct IntrospectionAdapter {
    issuer: ActorIssuer,
    expected_issuer: String,
    audience: String,
    endpoint: reqwest::Url,
    client_id: String,
    client_secret: String,
    http: reqwest::blocking::Client,
    cache: BTreeMap<[u8; 32], Actor>,
}
impl IntrospectionAdapter {
    /// HTTPS is mandatory; a separate explicit flag permits numeric loopback HTTP fixtures only.
    pub fn configured(
        authority: &str,
        issuer: &str,
        audience: &str,
        endpoint: &str,
        client_id: &str,
        secret: &str,
        loopback_fixture: bool,
    ) -> Result<Self, AuthError> {
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| AuthError::UnsafeEndpoint)?;
        let loopback = endpoint
            .host_str()
            .and_then(|s| s.parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if endpoint.scheme() != "https"
            && !(loopback_fixture && endpoint.scheme() == "http" && loopback)
        {
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
            issuer: ActorIssuer::configured_by_host(authority, "tenant-a"),
            expected_issuer: issuer.into(),
            audience: audience.into(),
            endpoint,
            client_id: client_id.into(),
            client_secret: secret.into(),
            http,
            cache: BTreeMap::new(),
        })
    }
    pub fn invalidate(&mut self) {
        self.cache.clear();
    }
    pub fn cache_entries(&self) -> usize {
        self.cache.len()
    }
    pub fn authenticate(&mut self, token: &str, now: u64) -> Result<Actor, AuthError> {
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
            .basic_auth(&self.client_id, Some(&self.client_secret))
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
        let actor =
            self.issuer
                .verified(sub, PrincipalKind::Service, exp.min(now.saturating_add(5)));
        // Hard entry bound, deliberately simple flush eviction for the disposable probe.
        if self.cache.len() >= 8 {
            self.cache.clear();
        }
        self.cache.insert(digest, actor.clone());
        Ok(actor)
    }
}
