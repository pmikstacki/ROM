//! Host-owned endpoint and finite transport profile.
use rom_ai::{AiError, AiResult};
use std::fmt;

pub struct OpenRouterConfig {
    pub endpoint: String,
    pub credential_ref: String,
    pub catalog_ttl_seconds: u64,
    pub response_bytes: usize,
}
impl OpenRouterConfig {
    pub(crate) fn validate(&self, loopback: bool) -> AiResult<reqwest::Url> {
        if !(1..=3600).contains(&self.catalog_ttl_seconds)
            || !(1..=1024 * 1024).contains(&self.response_bytes)
            || self.credential_ref.is_empty()
            || self.credential_ref.len() > 160
            || !self.credential_ref.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(AiError::InvalidRequest);
        }
        let url = reqwest::Url::parse(&self.endpoint).map_err(|_| AiError::InvalidRequest)?;
        let production = self.endpoint == "https://openrouter.ai/api/v1"
            || self.endpoint == "https://eu.openrouter.ai/api/v1"
            || self.endpoint == "https://us.openrouter.ai/api/v1";
        let local = loopback
            && matches!(url.scheme(), "http" | "https")
            && matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
            && url.port().is_some()
            && url.path() == "/api/v1"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none();
        if !production && !local {
            return Err(AiError::InvalidRequest);
        }
        Ok(url)
    }
}
impl fmt::Debug for OpenRouterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpenRouterConfig { .. }")
    }
}
