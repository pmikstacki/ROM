//! Service-private run Resource with frozen owner, policy and conservative counters.
use super::{
    codec::{self, Validated},
    reservation::ReservationEntry,
    view::RunMilestone,
};
use crate::{
    AiError, AiResult, AttemptEvidence, CompletionRequest, RouteCursor, RoutingPolicy, Usage,
    request::{MAX_OUTPUT_BYTES, bounded_value, valid_name},
};
use rom::{Actor, PrincipalKind};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerIdentity {
    authority: String,
    subject: String,
    kind: PrincipalKind,
}
impl OwnerIdentity {
    pub(crate) fn validate_identity(&self) -> AiResult<()> {
        <Self as Validated>::validate(self)
    }
    /// Only a trusted host supplies the Actor; serialized request fields are not credentials.
    pub fn from_actor(actor: &Actor) -> AiResult<Self> {
        let value = Self {
            authority: actor.authority.clone(),
            subject: actor.subject.clone(),
            kind: actor.principal_kind(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn matches(&self, actor: &Actor) -> bool {
        self.authority == actor.authority
            && self.subject == actor.subject
            && self.kind == actor.principal_kind()
    }
    pub(crate) fn validate_service(&self) -> AiResult<()> {
        self.validate()?;
        if self.kind != PrincipalKind::Service {
            return Err(AiError::Denied);
        }
        Ok(())
    }
}
impl Validated for OwnerIdentity {
    fn validate(&self) -> AiResult<()> {
        for value in [&self.authority, &self.subject] {
            if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                return Err(AiError::InvalidRequest);
            }
        }
        Ok(())
    }
}
impl fmt::Debug for OwnerIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnerIdentity")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RunState {
    Queued,
    Prepared,
    Executing,
    ToolsPending,
    Waiting { retry_at_unix_ms: u64 },
    AwaitingReconciliation,
    Completed,
    Failed,
    Cancelled,
    CancelRequested,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCounters {
    pub(crate) generation_attempts: u32,
    pub(crate) tool_calls: u32,
    pub(crate) ticks: u32,
}
impl RunCounters {
    pub fn generation_attempts(&self) -> u32 {
        self.generation_attempts
    }
    pub fn tool_calls(&self) -> u32 {
        self.tool_calls
    }
    pub fn ticks(&self) -> u32 {
        self.ticks
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    version: u32,
    pub(crate) run_id: String,
    pub(crate) service: OwnerIdentity,
    owner: OwnerIdentity,
    pub(crate) request: CompletionRequest,
    pub(crate) policy: RoutingPolicy,
    pub(crate) state: RunState,
    pub(crate) cursor: Option<RouteCursor>,
    created_at_unix_ms: u64,
    pub(crate) expires_at_unix_ms: u64,
    pub(crate) last_observed_unix_ms: u64,
    pub(crate) checkpoint: u32,
    pub(crate) counters: RunCounters,
    pub(crate) active_attempt: Option<ReservationEntry>,
    pub(crate) evidence_history: Vec<AttemptEvidence>,
    pub(crate) usage_history: Vec<Usage>,
    pub(crate) milestones: Vec<RunMilestone>,
    pub(crate) validated_output: Option<serde_json::Value>,
    pub(crate) failure: Option<AiError>,
    pub(crate) cancel_requested: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) queue_admitted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) operations: Vec<super::operation::OperationStamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tool_registry_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tool_turns: Vec<crate::tools::transcript::ToolTurn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) unstarted: Option<super::unstarted::UnstartedProof>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) route_continuations: Vec<super::route_continuation::RouteContinuation>,
}
impl RunRecord {
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn owner(&self) -> &OwnerIdentity {
        &self.owner
    }
    pub fn request(&self) -> &CompletionRequest {
        &self.request
    }
    pub(crate) fn effective_request(&self, generation: u32) -> AiResult<CompletionRequest> {
        crate::tools::transcript::effective(&self.request, &self.tool_turns, generation)
    }
    pub fn policy(&self) -> &RoutingPolicy {
        &self.policy
    }
    pub fn state(&self) -> &RunState {
        &self.state
    }
    pub fn cursor(&self) -> Option<&RouteCursor> {
        self.cursor.as_ref()
    }
    pub fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at_unix_ms
    }
    pub fn last_observed_unix_ms(&self) -> u64 {
        self.last_observed_unix_ms
    }
    pub fn checkpoint(&self) -> u32 {
        self.checkpoint
    }
    pub fn counters(&self) -> &RunCounters {
        &self.counters
    }
    pub fn active_attempt(&self) -> Option<&ReservationEntry> {
        self.active_attempt.as_ref()
    }
    pub fn attempt_evidence(&self) -> &[AttemptEvidence] {
        &self.evidence_history
    }
    pub fn attempt_usage(&self) -> &[Usage] {
        &self.usage_history
    }
    pub fn failure(&self) -> Option<&AiError> {
        self.failure.as_ref()
    }
    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }
    pub(crate) fn mark(&mut self) {
        if self.checkpoint == 0 {
            return;
        }
        let milestone = RunMilestone {
            sequence: self.checkpoint,
            state: self.state.clone(),
            observed_at_unix_ms: self.last_observed_unix_ms,
        };
        if self
            .milestones
            .last()
            .is_some_and(|old| old.sequence == self.checkpoint)
        {
            self.milestones.pop();
        }
        self.milestones.push(milestone);
    }
    pub(crate) fn note_evidence(
        &mut self,
        evidence: AttemptEvidence,
        usage: Usage,
    ) -> AiResult<()> {
        evidence.validate()?;
        let active = self.active_attempt.as_ref().ok_or(AiError::Conflict)?;
        if evidence.attempt_id() != active.prepared().identity() {
            return Err(AiError::Conflict);
        }
        if let Some(index) = self
            .evidence_history
            .iter()
            .position(|old| old.attempt_id() == evidence.attempt_id())
        {
            let old = &self.evidence_history[index];
            let old_usage = &self.usage_history[index];
            if (old.provider_request_id().is_some()
                && old.provider_request_id() != evidence.provider_request_id())
                || (old.generation_id().is_some()
                    && old.generation_id() != evidence.generation_id())
                || (old_usage.cost.is_some() && old_usage.cost != usage.cost)
                || (old_usage.input_tokens.is_some()
                    && old_usage.input_tokens != usage.input_tokens)
                || (old_usage.output_tokens.is_some()
                    && old_usage.output_tokens != usage.output_tokens)
                || (old_usage.reasoning_tokens.is_some()
                    && old_usage.reasoning_tokens != usage.reasoning_tokens)
            {
                return Err(AiError::Conflict);
            }
            self.evidence_history[index] = evidence;
            self.usage_history[index] = usage;
        } else {
            self.evidence_history.push(evidence);
            self.usage_history.push(usage);
        }
        Ok(())
    }
    pub(crate) fn observe(&mut self, now: u64) -> AiResult<()> {
        if now < self.last_observed_unix_ms || now >= self.expires_at_unix_ms {
            return Err(AiError::DeadlineExceeded);
        }
        self.last_observed_unix_ms = now;
        Ok(())
    }
    pub(crate) fn observe_recovery(&mut self, now: u64) -> AiResult<()> {
        if now < self.last_observed_unix_ms {
            return Err(AiError::DeadlineExceeded);
        }
        self.last_observed_unix_ms = now;
        Ok(())
    }
    pub(crate) fn transition_from(&self, before: &Self) -> AiResult<()> {
        if self.unstarted != super::unstarted::expected(before, self) {
            return Err(AiError::Conflict);
        }
        let mut normalized = self.clone();
        normalized.unstarted = before.unstarted.clone();
        normalized.transition_base(before)
    }
    fn transition_base(&self, before: &Self) -> AiResult<()> {
        if self.service != before.service
            || self.owner != before.owner
            || self.run_id != before.run_id
            || self.request != before.request
            || self.policy != before.policy
            || self.tool_registry_version != before.tool_registry_version
            || self.created_at_unix_ms != before.created_at_unix_ms
            || self.expires_at_unix_ms != before.expires_at_unix_ms
            || self.last_observed_unix_ms < before.last_observed_unix_ms
        {
            return Err(AiError::Conflict);
        }
        if self == before {
            return Ok(());
        }
        if super::route_continuation::transition(self, before)? {
            return Ok(());
        }
        if crate::tools::checkpoint::transition(self, before)? {
            return Ok(());
        }
        if super::operation::transition(self, before)? {
            return Ok(());
        }
        if super::knowledge::transition(self, before)? {
            return Ok(());
        }
        if before.state == RunState::Queued
            && self.state == RunState::Queued
            && !before.queue_admitted
            && self.queue_admitted
        {
            let mut admitted = before.clone();
            admitted.queue_admitted = true;
            return if self == &admitted {
                Ok(())
            } else {
                Err(AiError::Conflict)
            };
        }
        if self.queue_admitted != before.queue_admitted {
            return Err(AiError::Conflict);
        }
        if self.tool_turns != before.tool_turns {
            return Err(AiError::Conflict);
        }
        if before.cancel_requested && !self.cancel_requested {
            return Err(AiError::Conflict);
        }
        if self.evidence_history.len() < before.evidence_history.len()
            || self.evidence_history.len() > before.evidence_history.len() + 1
        {
            return Err(AiError::Conflict);
        }
        for (index, old) in before.evidence_history.iter().enumerate() {
            let new = &self.evidence_history[index];
            if old.attempt_id() != new.attempt_id() {
                return Err(AiError::Conflict);
            }
            if before
                .active_attempt
                .as_ref()
                .is_none_or(|entry| old.attempt_id() != entry.prepared().identity())
                && (old != new || before.usage_history[index] != self.usage_history[index])
            {
                return Err(AiError::Conflict);
            }
        }
        let preparing = matches!(
            (&before.state, &self.state),
            (RunState::Queued, RunState::Prepared)
                | (RunState::Waiting { .. }, RunState::Prepared)
                | (RunState::ToolsPending, RunState::Prepared)
        );
        if !preparing
            && (self.checkpoint != before.checkpoint
                || self.counters != before.counters
                || self.active_attempt != before.active_attempt
                || self.cursor != before.cursor)
        {
            return Err(AiError::Conflict);
        }
        match (&before.state, &self.state) {
            (RunState::Queued, RunState::Prepared)
            | (RunState::Waiting { .. }, RunState::Prepared)
            | (RunState::ToolsPending, RunState::Prepared) => {
                if before.state == RunState::ToolsPending
                    && before
                        .tool_turns
                        .last()
                        .is_none_or(|turn| !turn.completed())
                {
                    return Err(AiError::Conflict);
                }
                if self.checkpoint != before.checkpoint + 1
                    || self.counters.generation_attempts != before.counters.generation_attempts + 1
                    || self.counters.ticks != before.counters.ticks + 1
                    || self.counters.tool_calls != before.counters.tool_calls
                {
                    return Err(AiError::Conflict);
                }
                if let RunState::Waiting { retry_at_unix_ms } = before.state
                    && self.last_observed_unix_ms < retry_at_unix_ms
                {
                    return Err(AiError::Conflict);
                }
                if self.evidence_history != before.evidence_history
                    || self.usage_history != before.usage_history
                    || self.cancel_requested
                {
                    return Err(AiError::Conflict);
                }
                if let Some(old) = &before.cursor {
                    let new = self.cursor.as_ref().ok_or(AiError::Conflict)?;
                    if old.catalog_identity() != new.catalog_identity()
                        || (old.tier() == crate::RoutingTier::Paid
                            && new.tier() == crate::RoutingTier::Free)
                        || (old.tier() == new.tier() && new.next_candidate() < old.next_candidate())
                        || self
                            .policy
                            .free()
                            .iter()
                            .chain(self.policy.paid())
                            .any(|model| old.is_rejected(model) && !new.is_rejected(model))
                    {
                        return Err(AiError::Conflict);
                    }
                }
            }
            (RunState::Prepared, RunState::Executing)
            | (RunState::Executing, RunState::AwaitingReconciliation)
            | (RunState::CancelRequested, RunState::AwaitingReconciliation)
            | (RunState::Executing, RunState::Completed)
            | (RunState::AwaitingReconciliation, RunState::Completed)
            | (RunState::Executing, RunState::Waiting { .. })
            | (RunState::AwaitingReconciliation, RunState::Waiting { .. }) => {}
            (
                RunState::Queued | RunState::Prepared | RunState::Waiting { .. },
                RunState::Cancelled,
            )
            | (RunState::Executing, RunState::CancelRequested)
            | (RunState::AwaitingReconciliation, RunState::AwaitingReconciliation) => {
                if !self.cancel_requested || before.cancel_requested {
                    return Err(AiError::Conflict);
                }
            }
            (RunState::CancelRequested | RunState::AwaitingReconciliation, RunState::Cancelled) => {
                if !self.cancel_requested {
                    return Err(AiError::Conflict);
                }
            }
            (old, RunState::Failed)
                if !matches!(
                    old,
                    RunState::Completed | RunState::Failed | RunState::Cancelled
                ) => {}
            _ => return Err(AiError::Conflict),
        }
        Ok(())
    }
}
impl Validated for RunRecord {
    fn validate(&self) -> AiResult<()> {
        if self.operations.len() > super::operation::MAX_OPERATIONS {
            return Err(AiError::BudgetExhausted);
        }
        for (index, operation) in self.operations.iter().enumerate() {
            operation.validate()?;
            let pending_cancel = operation.kind == super::operation::OperationKind::Cancel
                && operation.phase == super::operation::OperationPhase::Pending;
            if operation.checkpoint > self.checkpoint
                || operation.admitted_at_ms < self.created_at_unix_ms
                || (!pending_cancel && operation.admitted_at_ms > self.last_observed_unix_ms)
                || self.operations[..index]
                    .iter()
                    .any(|old| old.key == operation.key)
            {
                return Err(AiError::InvalidRequest);
            }
        }
        self.service.validate_service()?;
        self.owner.validate()?;
        self.request.validate()?;
        self.policy.validate()?;
        super::route_continuation::validate(self)?;
        super::unstarted::validate(self)?;
        let completed_tools = crate::tools::transcript::validate(
            &self.request,
            &self.tool_turns,
            &self.run_id,
            self.policy.version(),
            self.tool_registry_version,
            self.counters.generation_attempts,
            self.policy.limits().tool_calls,
        )?;
        crate::tools::read_checkpoint::validate_record(self)?;
        if completed_tools != self.counters.tool_calls
            || self.checkpoint
                != self
                    .counters
                    .generation_attempts
                    .checked_add(completed_tools)
                    .ok_or(AiError::InvalidRequest)?
        {
            return Err(AiError::InvalidRequest);
        }
        let expires = self
            .created_at_unix_ms
            .checked_add(self.policy.limits().age_seconds * 1000)
            .ok_or(AiError::InvalidRequest)?;
        if self.version != 1
            || self.tool_registry_version == Some(0)
            || !valid_name(&self.run_id, 64)
            || self.expires_at_unix_ms != expires
            || self.last_observed_unix_ms < self.created_at_unix_ms
            || (self.last_observed_unix_ms >= expires
                && !matches!(
                    self.state,
                    RunState::AwaitingReconciliation
                        | RunState::Completed
                        | RunState::Failed
                        | RunState::Cancelled
                        | RunState::CancelRequested
                ))
            || self.checkpoint > 32
            || self.counters.generation_attempts > self.policy.limits().generation_attempts
            || self.counters.tool_calls > self.policy.limits().tool_calls
            || self.counters.ticks > self.policy.limits().ticks
            || u64::from(self.counters.ticks)
                != u64::from(self.checkpoint)
                    + self
                        .operations
                        .iter()
                        .filter(|operation| {
                            operation.kind == super::operation::OperationKind::Reconcile
                        })
                        .count() as u64
        {
            return Err(AiError::InvalidRequest);
        }
        if let Some(cursor) = &self.cursor {
            cursor.validate()?;
            if cursor.policy_version() != self.policy.version() {
                return Err(AiError::Conflict);
            }
        }
        if let Some(entry) = &self.active_attempt {
            entry.validate()?;
            if entry.key().run_id() != self.run_id
                || entry.key().step()
                    != crate::tools::transcript::source_checkpoint(
                        self.counters.generation_attempts,
                        &self.tool_turns,
                    )?
                || entry.key().attempt_ordinal() != self.counters.generation_attempts
                || entry.prepared().request()
                    != &self.effective_request(self.counters.generation_attempts)?
                || entry.prepared().policy() != &self.policy
                || Some(entry.prepared().route().next_cursor()) != self.cursor.as_ref()
                || entry.prepared().deadline().expires_at_unix_ms() > expires
                || entry.status() != &super::reservation::ReservationStatus::Reserved
                || entry.prepared().deadline().expires_at_unix_ms()
                    - entry.prepared().deadline().remaining_ms()
                    < self.created_at_unix_ms
                || entry.prepared().deadline().expires_at_unix_ms()
                    - entry.prepared().deadline().remaining_ms()
                    > self.last_observed_unix_ms
                || (self.state == RunState::Prepared
                    && entry.prepared().deadline().expires_at_unix_ms()
                        - entry.prepared().deadline().remaining_ms()
                        != self.last_observed_unix_ms)
            {
                return Err(AiError::Conflict);
            }
        }
        if matches!(
            self.state,
            RunState::Prepared
                | RunState::Executing
                | RunState::ToolsPending
                | RunState::AwaitingReconciliation
                | RunState::CancelRequested
                | RunState::Waiting { .. }
                | RunState::Completed
        ) && self.active_attempt.is_none()
        {
            return Err(AiError::InvalidRequest);
        }
        if self.evidence_history.len() > 8
            || self.evidence_history.len() != self.usage_history.len()
            || self.evidence_history.len() > self.counters.generation_attempts as usize
            || self.milestones.len() > 32
            || self.milestones.len() != self.checkpoint as usize
        {
            return Err(AiError::InvalidRequest);
        }
        for (index, evidence) in self.evidence_history.iter().enumerate() {
            evidence.validate()?;
            if self.evidence_history[..index]
                .iter()
                .any(|old| old.attempt_id() == evidence.attempt_id())
            {
                return Err(AiError::InvalidRequest);
            }
        }
        for (index, milestone) in self.milestones.iter().enumerate() {
            if milestone.sequence != index as u32 + 1
                || milestone.observed_at_unix_ms < self.created_at_unix_ms
                || milestone.observed_at_unix_ms > self.last_observed_unix_ms
            {
                return Err(AiError::InvalidRequest);
            }
        }
        if self.milestones.last().is_some_and(|last| {
            last.state != self.state || last.observed_at_unix_ms != self.last_observed_unix_ms
        }) {
            return Err(AiError::InvalidRequest);
        }
        if let Some(output) = &self.validated_output {
            bounded_value(output, MAX_OUTPUT_BYTES).map_err(|_| AiError::InvalidOutput)?;
            if !matches!(self.state, RunState::Completed | RunState::Cancelled) {
                return Err(AiError::InvalidRequest);
            }
        }
        if (self.state == RunState::Completed
            && (self.validated_output.is_none() || self.cancel_requested))
            || (self.state == RunState::Failed && self.failure.is_none())
            || (self.state == RunState::Cancelled && !self.cancel_requested)
            || (self.state == RunState::CancelRequested && !self.cancel_requested)
            || (self.cancel_requested
                && !matches!(
                    self.state,
                    RunState::CancelRequested
                        | RunState::AwaitingReconciliation
                        | RunState::Cancelled
                ))
        {
            return Err(AiError::InvalidRequest);
        }
        if self.failure.is_some() && !matches!(self.state, RunState::Failed | RunState::Cancelled) {
            return Err(AiError::InvalidRequest);
        }
        if let RunState::Waiting { retry_at_unix_ms } = self.state
            && (retry_at_unix_ms % 1000 != 0
                || retry_at_unix_ms <= self.last_observed_unix_ms
                || retry_at_unix_ms >= expires
                || self.cancel_requested
                || self.counters.generation_attempts >= self.policy.limits().generation_attempts
                || self.counters.ticks >= self.policy.limits().ticks)
        {
            return Err(AiError::InvalidRequest);
        }
        if self.state == RunState::Queued
            && (self.counters != RunCounters::default()
                || self.checkpoint != 0
                || self.active_attempt.is_some()
                || self.cursor.is_some())
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
impl fmt::Debug for RunRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunRecord")
            .field("state", &self.state)
            .field("counters", &self.counters)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, rom::Resource)]
#[resource(name = "ai_runs")]
pub struct AiRun {
    encoded: String,
}
impl AiRun {
    pub fn queued(
        run_id: impl Into<String>,
        service: &Actor,
        owner: OwnerIdentity,
        request: CompletionRequest,
        policy: RoutingPolicy,
        now_unix_ms: u64,
    ) -> AiResult<Self> {
        let expires_at_unix_ms = now_unix_ms
            .checked_add(
                policy
                    .limits()
                    .age_seconds
                    .checked_mul(1000)
                    .ok_or(AiError::InvalidRequest)?,
            )
            .ok_or(AiError::InvalidRequest)?;
        let record = RunRecord {
            version: 1,
            run_id: run_id.into(),
            service: OwnerIdentity::from_actor(service)?,
            owner,
            request,
            policy,
            state: RunState::Queued,
            cursor: None,
            created_at_unix_ms: now_unix_ms,
            expires_at_unix_ms,
            last_observed_unix_ms: now_unix_ms,
            checkpoint: 0,
            counters: RunCounters::default(),
            active_attempt: None,
            evidence_history: Vec::new(),
            usage_history: Vec::new(),
            milestones: Vec::new(),
            validated_output: None,
            failure: None,
            cancel_requested: false,
            queue_admitted: false,
            operations: Vec::new(),
            tool_registry_version: None,
            tool_turns: Vec::new(),
            route_continuations: Vec::new(),
            unstarted: None,
        };
        Ok(Self {
            encoded: codec::encode(&record)?,
        })
    }
    pub fn record(&self) -> AiResult<RunRecord> {
        codec::decode(&self.encoded)
    }
    pub(crate) fn store(&mut self, mut record: RunRecord) -> AiResult<()> {
        super::unstarted::retain(&self.record()?, &mut record);
        self.encoded = codec::encode(&record)?;
        Ok(())
    }
    pub fn definition_for(service: &Actor) -> AiResult<rom::Definition<Self>> {
        use rom::Resource;
        let service = OwnerIdentity::from_actor(service)?;
        service.validate_service()?;
        Ok(Self::definition()
            .policy(|actor, _, row| {
                row.record()
                    .is_ok_and(|record| record.service.matches(actor))
            })
            .allow_all_fields()
            .validate_transition(move |actor, before, after| {
                if !service.matches(actor) {
                    return Err(rom::Error::Denied);
                }
                let after = after
                    .ok_or(rom::Error::Denied)?
                    .record()
                    .map_err(codec::rom_error)?;
                if after.service != service {
                    return Err(rom::Error::Denied);
                }
                if let Some(before) = before {
                    after
                        .transition_from(&before.record().map_err(codec::rom_error)?)
                        .map_err(codec::rom_error)?;
                } else if after.state != RunState::Queued {
                    return Err(rom::Error::Denied);
                }
                Ok(())
            })
            .action(super::route_continuation::CONFIRM_NONACCEPTANCE)
            .action(super::route_continuation::STAGE_SUCCESSOR)
            .action(super::route_continuation::FAIL_CONTINUATION)
            .action(super::actions::PREPARE_RUN)
            .action(super::preparation::FAIL_PREPARATION)
            .action(super::actions::START_RUN)
            .action(super::actions::HOLD_RUN)
            .action(crate::tools::checkpoint::ADMIT_TOOL_BATCH)
            .action(crate::tools::checkpoint::PREPARE_TOOL_ACTION)
            .action(crate::tools::checkpoint::COMMIT_TOOL_RESULT)
            .action(crate::tools::checkpoint::HOLD_TOOL_ACTION)
            .action(super::knowledge::RECORD_ATTEMPT_EVIDENCE)
            .action(super::checkpoint::CHECKPOINT_RUN)
            .action(super::checkpoint::CANCEL_RUN))
    }
}
impl fmt::Debug for AiRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AiRun { .. }")
    }
}
