use super::*;
/// Host-established identity only. Intentionally does not implement Deserialize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub authority: String,
    pub subject: String,
    expires_at: Option<u64>,
}
impl Actor {
    /// Trusted embedded/adapter boundary; this constructor does not verify credentials.
    pub fn trusted(authority: &str, subject: &str) -> Self {
        Self {
            authority: authority.into(),
            subject: subject.into(),
            expires_at: None,
        }
    }
    /// Exclusive expiry boundary, expressed in Unix seconds from the runtime's clock.
    pub fn expires_at(mut self, unix_seconds: u64) -> Self {
        self.expires_at = Some(unix_seconds);
        self
    }
    pub fn valid_until(&self) -> Option<u64> {
        self.expires_at
    }
    pub(crate) fn key(&self) -> String {
        json!([self.authority, self.subject]).to_string()
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Access {
    Read,
    Write,
}
/// Trusted host time source, expressed as Unix seconds. This seam makes expiry deterministic in tests.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> u64;
}
#[derive(Default)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(u64::MAX, |d| d.as_secs())
    }
}
