//! Provider-neutral blob storage and ordinary authorized attachment Resources.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
use rom::{Actor, Field, PrincipalKind, Resource, Value};
use sha2::{Digest as _, Sha256};
use std::{future::Future, pin::Pin};

pub type Result<T> = std::result::Result<T, Error>;
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub type Upload = Pin<Box<dyn futures_util::Stream<Item = Result<Vec<u8>>> + Send>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    TooLarge,
    Input,
    Timeout,
    Conflict,
    Missing,
    Denied,
    Unsupported,
    Backend,
    Unknown,
    Overloaded,
    Closed,
    Panicked,
    Core(rom::Error),
}
impl From<rom::Error> for Error {
    fn from(error: rom::Error) -> Self {
        Self::Core(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// SHA-256 content identity, independent of provider ETags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest(String);
impl Digest {
    pub fn of(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() != 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Invalid);
        }
        Ok(Self(text.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Field for Digest {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> Value {
        rom::json!(self.0)
    }
    fn decode(value: Value) -> rom::Result<Self> {
        value
            .as_str()
            .and_then(|s| Self::parse(s).ok())
            .ok_or_else(|| rom::Error::invalid("blobs", "digest"))
    }
}
/// Opaque physical key. It conveys no authority; only trusted adapters consume it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKey(Digest);
impl ObjectKey {
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self(Digest::parse(text)?))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub bytes: u64,
}
/// Trusted storage port. A successful create publishes one complete object;
/// existing keys return Conflict without overwrite. Unknown must not mean rollback.
pub trait BlobStore: Send + Sync + 'static {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()>;
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>>;
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata>;
    /// Trusted maintenance only: caller must prove detachment, grace and quiescence.
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobState {
    Pending,
    Ready,
    Detached,
}
impl Field for BlobState {
    fn shape() -> rom::Shape {
        rom::Shape::Enum(vec!["pending".into(), "ready".into(), "detached".into()])
    }
    fn encode(&self) -> Value {
        rom::json!(match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Detached => "detached",
        })
    }
    fn decode(value: Value) -> rom::Result<Self> {
        match value.as_str() {
            Some("pending") => Ok(Self::Pending),
            Some("ready") => Ok(Self::Ready),
            Some("detached") => Ok(Self::Detached),
            _ => Err(rom::Error::invalid("blobs", "state")),
        }
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "blobs")]
pub struct Blob {
    pub owner: String,
    pub store: String,
    pub digest: Digest,
    pub bytes: u64,
    pub state: BlobState,
    pub upload_revision: u64,
}
/// Explicit host principal; enable it in IdentityGate when using native identities.
/// Never map client identity fields to this principal.
pub fn worker_actor() -> Actor {
    Actor::trusted("rom-blob-host", "attachments").with_kind(PrincipalKind::Service)
}
fn is_worker(actor: &Actor) -> bool {
    actor == &worker_actor()
}
fn owner(actor: &Actor) -> String {
    rom::json!([actor.authority, actor.principal_kind(), actor.subject]).to_string()
}
/// Register this definition to enforce the attachment protocol. Customizing its
/// policies transfers responsibility for preventing forged Ready references to the host.
pub fn definition() -> rom::Definition<Blob> {
    Blob::definition()
        .policy(|actor, _, blob| is_worker(actor) || blob.owner == owner(actor))
        .field_policy(|actor, access, _, blob| {
            matches!(access, rom::Access::Read)
                || is_worker(actor)
                || blob.state == BlobState::Pending
        })
}
mod service;
pub use service::*;
