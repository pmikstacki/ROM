use crate::{ConfigError, Format, ParsedDocument, parse};
use rom::{
    Action, Actor, Command, Error, Intent, Invocation, Key, Operation, PrincipalKind,
    ProjectedView, Resource, Result, RevisionCondition, Runtime, SourcePermit, SourceProvenance,
};

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

/// Prepared native request identity. No serde construction or credential retention.
#[derive(Clone)]
pub struct ReloadTicket {
    actor: Actor,
    source_id: String,
    activation_revision: u64,
    activation: SourceActivation,
    expected: Option<u64>,
}
impl ReloadTicket {
    /// Begin before fetching/parsing, so even a newer failed fetch supersedes old work.
    /// Conflict means another requester advanced the same record; reacquire explicitly.
    pub async fn request(
        runtime: &Runtime,
        actor: &Actor,
        source_id: &str,
        version: &str,
    ) -> Result<Self> {
        if actor.principal_kind() != PrincipalKind::Service {
            return Err(Error::Denied);
        }
        let before = runtime.read::<SourceActivation>(actor, source_id).await?;
        let config = before.value.as_ref().ok_or(Error::Denied)?;
        if !config.allows_worker(actor) {
            return Err(Error::Denied);
        }
        let actor = actor.clone().expires_at(
            actor
                .valid_until()
                .map_or(config.valid_until, |end| end.min(config.valid_until)),
        );
        let state = runtime
            .source_state(&actor, &config.target_kind, &config.target_id)
            .await?;
        let expected = state.as_ref().map(|r| r.revision);
        let target_present = state.as_ref().is_some_and(|r| r.present);
        let outcome = runtime
            .execute(
                &actor,
                Command::action(
                    source_id,
                    REQUEST_RELOAD,
                    ReloadRequest {
                        version: version.into(),
                        target_revision: expected,
                        target_present,
                    },
                )
                .at_revision(before.revision)
                .idempotency(&format!("request-{}", before.revision)),
            )
            .await?;
        let activation = outcome.value.ok_or(Error::Denied)?;
        Ok(Self {
            actor,
            source_id: source_id.into(),
            activation_revision: outcome.revision,
            activation,
            expected,
        })
    }
    /// Recover the current request without advancing generation, preserving its
    /// original target revision and idempotency scope across a process restart.
    pub async fn resume(runtime: &Runtime, actor: &Actor, source_id: &str) -> Result<Self> {
        if actor.principal_kind() != PrincipalKind::Service {
            return Err(Error::Denied);
        }
        let row = runtime.read::<SourceActivation>(actor, source_id).await?;
        let activation = row.value.ok_or(Error::Denied)?;
        if !activation.enabled
            || activation.requested_generation == 0
            || !activation.allows_worker(actor)
        {
            return Err(Error::Denied);
        }
        let actor =
            actor
                .clone()
                .expires_at(actor.valid_until().map_or(activation.valid_until, |end| {
                    end.min(activation.valid_until)
                }));
        Ok(Self {
            actor,
            source_id: source_id.into(),
            activation_revision: row.revision,
            expected: activation.target_revision,
            activation,
        })
    }
    /// Source generation requested by this ticket, distinct from accepted target state.
    pub fn generation(&self) -> u64 {
        self.activation.requested_generation
    }
    /// Parse errors do not apply an empty document or delete anything.
    pub async fn load(
        &self,
        runtime: &Runtime,
        format: Format,
        document: &str,
        safe_origin: &str,
    ) -> std::result::Result<ProjectedView, ReloadError> {
        let parsed = parse(format, document, safe_origin).map_err(ReloadError::Parse)?;
        self.apply(runtime, &parsed)
            .await
            .map_err(ReloadError::Runtime)
    }
    fn permit(&self, origins: std::collections::BTreeMap<String, String>) -> Result<SourcePermit> {
        SourcePermit::trusted(
            Key {
                kind: self.activation.target_kind.clone(),
                id: self.activation.target_id.clone(),
            },
            SourceProvenance {
                source: self.source_id.clone(),
                version: self.activation.source_version.clone(),
                generation: self.activation.requested_generation,
                field_origins: origins,
            },
            RevisionCondition {
                key: Key {
                    kind: SourceActivation::KIND.into(),
                    id: self.source_id.clone(),
                },
                revision: self.activation_revision,
            },
            self.activation.valid_until,
        )
    }
    fn invocation(&self, operation: Operation) -> Invocation {
        Invocation {
            kind: self.activation.target_kind.clone(),
            id: self.activation.target_id.clone(),
            expected: self.expected,
            idempotency: format!(
                "source-{}-{}",
                self.source_id, self.activation.requested_generation
            ),
            operation,
        }
    }
    /// Submit complete values through the ordinary registered codec and permissions.
    pub async fn apply(
        &self,
        runtime: &Runtime,
        candidate: &ParsedDocument,
    ) -> Result<ProjectedView> {
        let operation = if !self.activation.target_present {
            Operation::Create(candidate.values().clone())
        } else {
            Operation::Replace(candidate.values().clone())
        };
        runtime
            .invoke_sourced(
                &self.actor,
                self.invocation(operation),
                self.permit(candidate.origins().clone())?,
            )
            .await
            .map_err(safe_runtime_error)
    }
    /// Explicit owned deletion. Missing/unavailable sources never call this implicitly.
    pub async fn delete(&self, runtime: &Runtime) -> Result<ProjectedView> {
        runtime
            .invoke_sourced(
                &self.actor,
                self.invocation(Operation::Delete),
                self.permit(Default::default())?,
            )
            .await
            .map_err(safe_runtime_error)
    }
}
impl std::fmt::Debug for ReloadTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReloadTicket")
            .field("generation", &self.generation())
            .finish_non_exhaustive()
    }
}
fn safe_runtime_error(error: Error) -> Error {
    match error {
        Error::Invalid { .. } => Error::invalid("configuration-target", "candidate"),
        Error::Unsupported(_) => Error::Unsupported("configuration target capability".into()),
        Error::Duplicate(_) => Error::Duplicate("configuration target".into()),
        other => other,
    }
}
/// Structured safe failure; dynamic target validation details are redacted.
#[derive(Debug)]
pub enum ReloadError {
    Parse(ConfigError),
    Runtime(Error),
}
impl std::fmt::Display for ReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => e.fmt(f),
            Self::Runtime(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for ReloadError {}
