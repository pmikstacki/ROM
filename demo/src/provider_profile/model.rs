//! Explicit host settings; none of these inputs are proof constructors.
use rom::{Error, Result};
use std::time::{Duration, Instant};

/// Host authentication admission and caller deadline, independent of HTTP body limits.
#[derive(Clone, Copy, Debug)]
pub struct AuthLimits {
    pub jobs: usize,
    pub response_timeout: Duration,
}
impl Default for AuthLimits {
    fn default() -> Self {
        Self {
            jobs: 4,
            response_timeout: Duration::from_secs(2),
        }
    }
}
impl AuthLimits {
    pub(super) fn validate(&self) -> Result<()> {
        if self.jobs == 0
            || self.jobs > tokio::sync::Semaphore::MAX_PERMITS
            || self.response_timeout.is_zero()
            || Instant::now().checked_add(self.response_timeout).is_none()
        {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
}

/// Exact host-approved values for the service introspection profile.
/// A Resource mutation cannot approve a different endpoint or credential namespace.
#[derive(Clone, Debug)]
pub struct ApprovedProvider {
    pub authority: String,
    pub issuer: String,
    pub audience: String,
    pub endpoint: String,
    pub introspection_client: String,
}
impl ApprovedProvider {
    pub(super) fn validate(&self) -> Result<()> {
        if [
            &self.authority,
            &self.issuer,
            &self.audience,
            &self.endpoint,
            &self.introspection_client,
        ]
        .into_iter()
        .any(|v| v.is_empty() || v.len() > 2048 || v.trim() != v || v.chars().any(char::is_control))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
