//! Managed source activation, strict request codec and pure reload action.
use rom::{Action, Actor, Error, Intent, PrincipalKind, Resource, Result};

/// Managed source grant and latest requested generation. The target's protected
/// provenance, not this request record, states which generation actually committed.
#[derive(Clone, Debug, Resource)]
#[resource(name = "configuration-sources")]
pub struct SourceActivation {
    /// Explicit trusted service authority, editable only by source administrators.
    pub worker_authority: String,
    /// Explicit trusted service subject; read access alone does not establish this identity.
    pub worker_subject: String,
    pub enabled: bool,
    pub target_kind: String,
    pub target_id: String,
    pub requested_generation: u64,
    pub source_version: String,
    /// Target revision pinned before this request, retained for restart-safe replay.
    pub target_revision: Option<u64>,
    /// Whether the pinned target was live, so restart retries keep Create/Replace semantics.
    pub target_present: bool,
    /// Exclusive write-permit expiry in host Unix seconds, not a target data TTL.
    pub valid_until: u64,
}
impl SourceActivation {
    /// Use this in host row/field policy as well as the loader's request/resume checks.
    pub fn allows_worker(&self, actor: &Actor) -> bool {
        actor.principal_kind() == PrincipalKind::Service
            && actor.authority == self.worker_authority
            && actor.subject == self.worker_subject
    }
}
/// Pure action input: safe version label and the pinned target revision.
#[derive(Clone)]
pub struct ReloadRequest {
    pub version: String,
    pub target_revision: Option<u64>,
    pub target_present: bool,
}
impl rom::Input for ReloadRequest {
    fn encode(&self) -> rom::Value {
        rom::json!({"version":self.version,"target_revision":self.target_revision,"target_present":self.target_present})
    }
    fn decode(value: rom::Value) -> Result<Self> {
        let map = value
            .as_object()
            .ok_or_else(|| Error::invalid(SourceActivation::KIND, "request"))?;
        if map.len() != 3 {
            return Err(Error::invalid(SourceActivation::KIND, "request"));
        }
        let version = map
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid(SourceActivation::KIND, "version"))?
            .to_owned();
        let target_revision = match map.get("target_revision") {
            Some(rom::Value::Null) => None,
            Some(v) => Some(
                v.as_u64()
                    .ok_or_else(|| Error::invalid(SourceActivation::KIND, "target_revision"))?,
            ),
            None => return Err(Error::invalid(SourceActivation::KIND, "target_revision")),
        };
        Ok(Self {
            version,
            target_revision,
            target_present: map
                .get("target_present")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| Error::invalid(SourceActivation::KIND, "target_present"))?,
        })
    }
}
fn request(source: &mut SourceActivation, input: ReloadRequest) -> Result<Vec<Intent>> {
    let version = input.version;
    if !source.enabled
        || source.target_kind.is_empty()
        || source.target_id.is_empty()
        || version.is_empty()
        || version.len() > 128
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        return Err(Error::Denied);
    }
    source.requested_generation = source
        .requested_generation
        .checked_add(1)
        .ok_or(Error::TooLarge)?;
    source.source_version = version;
    source.target_revision = input.target_revision;
    source.target_present = input.target_present;
    Ok(vec![])
}
/// Register explicitly on SourceActivation::definition; host field policy should
/// allow workers to change only requested_generation, source_version, target_revision.
pub const REQUEST_RELOAD: Action<SourceActivation, ReloadRequest> =
    Action::new("request-reload", request);
