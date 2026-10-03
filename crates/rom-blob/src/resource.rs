//! Attachment resource state and authorization policy.
use crate::Digest;
use rom::{Actor, Field, PrincipalKind, Resource, Value};

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
pub(crate) fn owner(actor: &Actor) -> String {
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
