use rom::{Error, Result};
use url::Url;

/// Trusted host-only routing for a public HTTPS identity provider.
/// This approval cannot come from browser input or Resource configuration.
#[derive(Clone)]
pub struct TrustedLoopbackBackchannel {
    issuer: String,
    pub(crate) token: String,
    pub(crate) jwks: String,
}
impl TrustedLoopbackBackchannel {
    pub fn new(issuer: &str, token_endpoint: &str, jwks_endpoint: &str) -> Self {
        Self {
            issuer: issuer.into(),
            token: token_endpoint.into(),
            jwks: jwks_endpoint.into(),
        }
    }
    pub(crate) fn validate(&self, issuer: &str, public_origin: &str) -> Result<()> {
        if self.issuer != issuer
            || !issuer.starts_with("https://")
            || !public_origin.starts_with("https://")
        {
            return Err(invalid());
        }
        let token = loopback(&self.token)?;
        let jwks = loopback(&self.jwks)?;
        if token.origin() != jwks.origin() {
            return Err(invalid());
        }
        Ok(())
    }
}
fn loopback(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| invalid())?;
    let local = matches!(url.host(), Some(url::Host::Ipv4(ip)) if ip.is_loopback())
        || matches!(url.host(), Some(url::Host::Ipv6(ip)) if ip.is_loopback());
    if value.len() > 2048
        || url.as_str() != value
        || url.scheme() != "http"
        || !local
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(url)
}
fn invalid() -> Error {
    Error::invalid("studio-host", "Invalid trusted loopback backchannel")
}
