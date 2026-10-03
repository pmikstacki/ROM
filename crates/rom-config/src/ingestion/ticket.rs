//! Prepared reload identity, generation fencing and source-owned application.
use super::{
    activation::{REQUEST_RELOAD, ReloadRequest, SourceActivation},
    errors::{ReloadError, safe_runtime_error},
};
use crate::{Format, ParsedDocument, parse};
use rom::{
    Actor, Command, Error, Invocation, Key, Operation, PrincipalKind, ProjectedView, Resource,
    Result, RevisionCondition, Runtime, SourcePermit, SourceProvenance,
};

/// Prepared native request identity. No serde construction or credential retention.
#[derive(Clone)]
pub struct ReloadTicket {
    retry_epoch: u64,
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
        Self::request_with_epoch(runtime, actor, source_id, version, 0).await
    }
    /// Begin with an explicit retry epoch. Retain this epoch with the request's
    /// recovery information; never substitute a newer epoch when retrying it.
    pub async fn request_with_epoch(
        runtime: &Runtime,
        actor: &Actor,
        source_id: &str,
        version: &str,
        retry_epoch: u64,
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
                .retry_epoch(retry_epoch)
                .idempotency(&format!("request-{}", before.revision)),
            )
            .await?;
        let activation = outcome.value.ok_or(Error::Denied)?;
        Ok(Self {
            retry_epoch,
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
        Self::resume_with_epoch(runtime, actor, source_id, 0).await
    }
    /// Recover using the original epoch retained by the caller. SourceActivation
    /// does not persist this epoch, so the default resume method always uses zero.
    /// This method never queries or substitutes the current admission epoch.
    pub async fn resume_with_epoch(
        runtime: &Runtime,
        actor: &Actor,
        source_id: &str,
        retry_epoch: u64,
    ) -> Result<Self> {
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
            retry_epoch,
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
    pub fn retry_epoch(&self) -> u64 {
        self.retry_epoch
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
            retry_epoch: self.retry_epoch,
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
