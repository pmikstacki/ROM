use super::*;
/// Principal namespaces are part of revocation and idempotency identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    Embedded,
    Human,
    Service,
}
/// Host-established identity only. Intentionally does not implement Deserialize.
#[derive(Clone, PartialEq, Eq)]
pub struct Actor {
    pub authority: String,
    pub subject: String,
    expires_at: Option<u64>,
    kind: PrincipalKind,
    host_stamp: Option<String>,
}
impl Actor {
    /// Trusted embedded/adapter boundary; this constructor does not verify credentials.
    pub fn trusted(authority: &str, subject: &str) -> Self {
        Self {
            authority: authority.into(),
            subject: subject.into(),
            expires_at: None,
            kind: PrincipalKind::Embedded,
            host_stamp: None,
        }
    }
    /// Trusted host classification; never infer this from a request's actor fields.
    pub fn with_kind(mut self, kind: PrincipalKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn principal_kind(&self) -> PrincipalKind {
        self.kind
    }
    /// Non-secret host validation context. Excluded from durable principal identity.
    /// The configured gate must validate this stamp; this setter grants no permission.
    pub fn with_host_stamp(mut self, stamp: &str) -> Self {
        self.host_stamp = Some(stamp.into());
        self
    }
    pub fn host_stamp(&self) -> Option<&str> {
        self.host_stamp.as_deref()
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
        json!([self.authority, self.kind, self.subject]).to_string()
    }
}
impl std::fmt::Debug for Actor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actor")
            .field("authority", &self.authority)
            .field("kind", &self.kind)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}
/// Bounded read-only storage view available inside an authoritative host check.
/// It exposes no writes, recursive runtime entrypoints, or unbounded snapshots.
pub trait AuthorizationRead {
    fn load(&mut self, key: &Key) -> Result<Option<Row>>;
}
/// Optional host integration for current identity state. Called only in bounded
/// blocking I/O work; implementations must not call the runtime recursively.
/// Every error denies the operation. An absent gate is the embedded host profile.
pub trait ActorGate: Send + Sync + 'static {
    fn check(&self, actor: &Actor, storage: &mut dyn AuthorizationRead) -> Result<()>;
}
pub(crate) struct GateRead<'a> {
    pub(crate) storage: &'a dyn Storage,
    pub(crate) reads: usize,
    pub(crate) bytes: usize,
}
impl AuthorizationRead for GateRead<'_> {
    fn load(&mut self, key: &Key) -> Result<Option<Row>> {
        if self.reads == 0 {
            return Err(Error::TooLarge);
        }
        self.reads -= 1;
        let row = self.storage.load(key)?;
        if let Some(row) = &row {
            if row.key != *key {
                return Err(Error::Storage);
            }
            self.bytes = self
                .bytes
                .checked_sub(serde_json::to_vec(row).map_err(|_| Error::Storage)?.len())
                .ok_or(Error::TooLarge)?;
        }
        Ok(row)
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
