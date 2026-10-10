//! Atomic private tool checkpoints; external invocation remains in the coordinator.
use super::{
    PreparedToolAction,
    transcript::{ToolKind, ToolResult, ToolTurn},
};
use crate::{
    AiError, AiResult, AttemptEvidence, Usage,
    flow::{
        AiRun, FLOW_TICKS, FlowTick, OwnerIdentity, RunRecord, RunState,
        codec::{self, Validated, input},
    },
    request::{MAX_TOOL_BYTES, bounded_value},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum ToolChange {
    Read {
        change: super::read_checkpoint::ReadChange,
    },
    Admit {
        turn: ToolTurn,
        evidence: AttemptEvidence,
        usage: Usage,
    },
    Prepare {
        prepared: PreparedToolAction,
    },
    AuthorizeRead {
        call_id: String,
        actor: OwnerIdentity,
    },
    Result {
        call_id: String,
        actor: OwnerIdentity,
        result: Value,
    },
    Unknown {
        call_id: String,
    },
    Recover {
        call_id: String,
    },
    Failed {
        error: AiError,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolCheckpoint {
    change: ToolChange,
    now_unix_ms: u64,
}
impl ToolCheckpoint {
    pub(crate) fn new(change: ToolChange, now_unix_ms: u64) -> AiResult<Self> {
        let value = Self {
            change,
            now_unix_ms,
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for ToolCheckpoint {
    fn validate(&self) -> AiResult<()> {
        match &self.change {
            ToolChange::Read { change } => change.validate()?,
            ToolChange::Admit {
                turn,
                evidence,
                usage,
            } => {
                turn.source.validate()?;
                evidence.validate()?;
                if turn.steps.is_empty()
                    || turn.steps.len() > 8
                    || turn.registry_version == 0
                    || usage.cost.is_none()
                {
                    return Err(AiError::InvalidOutput);
                }
                for step in &turn.steps {
                    step.call.validate()?;
                    if step.actor.is_some()
                        || step.prepared.is_some()
                        || step.result.is_some()
                        || step.unknown
                        || step.read_attempt.is_some()
                    {
                        return Err(AiError::InvalidOutput);
                    }
                }
            }
            ToolChange::Prepare { prepared } => prepared.validate()?,
            ToolChange::AuthorizeRead { call_id, actor } => {
                if !crate::request::valid_name(call_id, 128) {
                    return Err(AiError::InvalidOutput);
                }
                actor.validate_identity()?;
            }
            ToolChange::Result {
                call_id,
                actor,
                result,
            } => {
                if !crate::request::valid_name(call_id, 128) {
                    return Err(AiError::InvalidOutput);
                }
                actor.validate_identity()?;
                bounded_value(result, MAX_TOOL_BYTES)?;
            }
            ToolChange::Unknown { call_id } | ToolChange::Recover { call_id } => {
                if !crate::request::valid_name(call_id, 128) {
                    return Err(AiError::InvalidOutput);
                }
            }
            ToolChange::Failed { error } => {
                if matches!(error, AiError::UnknownOutcome | AiError::RateLimited { .. }) {
                    return Err(AiError::InvalidRequest);
                }
            }
        }
        Ok(())
    }
}
input!(ToolCheckpoint);

pub(super) fn definite_read_failure(error: &AiError) -> bool {
    matches!(
        error,
        AiError::Denied
            | AiError::InvalidOutput
            | AiError::InvalidRequest
            | AiError::UnsupportedCapability
            | AiError::BudgetExhausted
    )
}

fn apply(record: &mut RunRecord, input: &ToolCheckpoint) -> AiResult<()> {
    input.validate()?;
    if record.cancel_requested {
        return Err(AiError::Conflict);
    }
    match &input.change {
        ToolChange::Read { change } => {
            super::read_checkpoint::apply(record, change, input.now_unix_ms)?
        }
        ToolChange::Admit {
            turn,
            evidence,
            usage,
        } => {
            let active = record.active_attempt.as_ref().ok_or(AiError::Conflict)?;
            if record.state != RunState::Executing
                || &turn.source != active.key()
                || evidence.attempt_id() != active.prepared().identity()
                || Some(turn.registry_version) != record.tool_registry_version
                || usage
                    .cost
                    .is_none_or(|cost| cost > active.prepared().route().maximum_cost())
                || record.tool_turns.last().is_some_and(|old| !old.completed())
            {
                return Err(AiError::Conflict);
            }
            record.observe(input.now_unix_ms)?;
            record.note_evidence(evidence.clone(), usage.clone())?;
            record.tool_turns.push(turn.clone());
            record.state = RunState::ToolsPending;
        }
        ToolChange::Prepare { prepared } => {
            if record.state != RunState::ToolsPending {
                return Err(AiError::Conflict);
            }
            record.observe(input.now_unix_ms)?;
            let turn = record.tool_turns.last_mut().ok_or(AiError::Conflict)?;
            let step = turn
                .steps
                .iter_mut()
                .find(|step| step.result.is_none())
                .ok_or(AiError::Conflict)?;
            if step.kind != ToolKind::Action
                || step.actor.is_some()
                || step.prepared.is_some()
                || prepared.call_id() != step.call.id()
                || prepared.tool_name != step.call.name()
                || prepared.registry_version != turn.registry_version
            {
                return Err(AiError::Conflict);
            }
            step.actor = Some(prepared.actor.clone());
            step.prepared = Some(prepared.clone());
        }
        ToolChange::AuthorizeRead { call_id, actor } => {
            if record.state != RunState::ToolsPending {
                return Err(AiError::Conflict);
            }
            record.observe(input.now_unix_ms)?;
            let step = record
                .tool_turns
                .last_mut()
                .ok_or(AiError::Conflict)?
                .steps
                .iter_mut()
                .find(|step| step.result.is_none())
                .ok_or(AiError::Conflict)?;
            if step.kind != ToolKind::Read
                || step.actor.is_some()
                || step.prepared.is_some()
                || step.call.id() != call_id
            {
                return Err(AiError::Conflict);
            }
            step.actor = Some(actor.clone());
        }
        ToolChange::Result {
            call_id,
            actor,
            result,
        } => {
            if record.state != RunState::ToolsPending {
                return Err(AiError::Conflict);
            }
            record.observe(input.now_unix_ms)?;
            let turn = record.tool_turns.last_mut().ok_or(AiError::Conflict)?;
            let step = turn
                .steps
                .iter_mut()
                .find(|step| step.result.is_none())
                .ok_or(AiError::Conflict)?;
            if step.kind != ToolKind::Action
                || step.call.id() != call_id
                || step.unknown
                || step.actor.as_ref().is_some_and(|old| old != actor)
                || (step.kind == ToolKind::Action && step.prepared.is_none())
            {
                return Err(AiError::Conflict);
            }
            step.actor = Some(actor.clone());
            step.result = Some(ToolResult {
                value: result.clone(),
            });
            record.checkpoint = record
                .checkpoint
                .checked_add(1)
                .ok_or(AiError::InvalidRequest)?;
            record.counters.tool_calls = record
                .counters
                .tool_calls
                .checked_add(1)
                .ok_or(AiError::InvalidRequest)?;
            record.counters.ticks = record
                .counters
                .ticks
                .checked_add(1)
                .ok_or(AiError::InvalidRequest)?;
        }
        ToolChange::Unknown { call_id } => {
            if record.state != RunState::ToolsPending {
                return Err(AiError::Conflict);
            }
            record.observe_recovery(input.now_unix_ms)?;
            let step = record
                .tool_turns
                .last_mut()
                .ok_or(AiError::Conflict)?
                .steps
                .iter_mut()
                .find(|step| step.result.is_none())
                .ok_or(AiError::Conflict)?;
            if step.call.id() != call_id || step.prepared.is_none() || step.kind != ToolKind::Action
            {
                return Err(AiError::Conflict);
            }
            step.unknown = true;
            record.state = RunState::AwaitingReconciliation;
        }
        ToolChange::Recover { call_id } => {
            if record.state != RunState::AwaitingReconciliation {
                return Err(AiError::Conflict);
            }
            record.observe(input.now_unix_ms)?;
            let step = record
                .tool_turns
                .last_mut()
                .ok_or(AiError::Conflict)?
                .steps
                .iter_mut()
                .find(|step| step.result.is_none())
                .ok_or(AiError::Conflict)?;
            if step.call.id() != call_id
                || step.kind != ToolKind::Action
                || step.prepared.is_none()
                || !step.unknown
            {
                return Err(AiError::Conflict);
            }
            step.unknown = false;
            record.state = RunState::ToolsPending;
        }
        ToolChange::Failed { error } => {
            if record.state != RunState::ToolsPending
                || record.tool_turns.last().is_none_or(|turn| {
                    !turn.completed()
                        && !turn.next().is_some_and(|step| {
                            step.kind == ToolKind::Read
                                && step.prepared.is_none()
                                && !step.unknown
                                && (definite_read_failure(error)
                                    || (*error == AiError::DeadlineExceeded
                                        && input.now_unix_ms >= record.expires_at_unix_ms))
                        })
                })
            {
                return Err(AiError::Conflict);
            }
            record.observe_recovery(input.now_unix_ms)?;
            record.state = RunState::Failed;
            record.failure = Some(error.clone());
        }
    }
    record.mark();
    record.validate()?;
    Ok(())
}
fn execute(run: &mut AiRun, input: ToolCheckpoint) -> rom::Result<Vec<rom::Intent>> {
    let mut record = run.record().map_err(codec::rom_error)?;
    apply(&mut record, &input).map_err(codec::rom_error)?;
    let schedule = matches!(
        input.change,
        ToolChange::Admit { .. }
            | ToolChange::Result { .. }
            | ToolChange::Recover { .. }
            | ToolChange::Read {
                change: super::read_checkpoint::ReadChange::Retry { .. }
                    | super::read_checkpoint::ReadChange::Wake { .. }
                    | super::read_checkpoint::ReadChange::Result { .. }
            }
    );
    let intents = if schedule {
        vec![
            FLOW_TICKS.intent(
                FlowTick::new(record.run_id.clone(), u64::from(record.checkpoint) + 1)
                    .map_err(codec::rom_error)?,
            ),
        ]
    } else {
        vec![]
    };
    run.store(record).map_err(codec::rom_error)?;
    Ok(intents)
}
pub(crate) const ADMIT_TOOL_BATCH: rom::Action<AiRun, ToolCheckpoint> =
    rom::Action::new("admit_tools", execute);
pub(crate) const PREPARE_TOOL_ACTION: rom::Action<AiRun, ToolCheckpoint> =
    rom::Action::new("prepare_tool", execute);
pub(crate) const COMMIT_TOOL_RESULT: rom::Action<AiRun, ToolCheckpoint> =
    rom::Action::new("tool_result", execute);
pub(crate) const HOLD_TOOL_ACTION: rom::Action<AiRun, ToolCheckpoint> =
    rom::Action::new("hold_tool", execute);

/// Reconstruct the only permitted delta from frozen facts; historical turns remain exact.
pub(crate) fn transition(after: &RunRecord, before: &RunRecord) -> AiResult<bool> {
    if before.state == RunState::ToolsPending && after.state == RunState::Failed {
        let mut expected = before.clone();
        apply(
            &mut expected,
            &ToolCheckpoint::new(
                ToolChange::Failed {
                    error: after.failure.clone().ok_or(AiError::Conflict)?,
                },
                after.last_observed_unix_ms,
            )?,
        )?;
        return if &expected == after {
            Ok(true)
        } else {
            Err(AiError::Conflict)
        };
    }
    if before.state == RunState::ToolsPending && after.cancel_requested && !before.cancel_requested
    {
        let mut expected = before.clone();
        cancel(&mut expected, after.last_observed_unix_ms)?;
        return if &expected == after {
            Ok(true)
        } else {
            Err(AiError::Conflict)
        };
    }
    let read_change = super::read_checkpoint::derive(after, before)?;
    if before.tool_turns == after.tool_turns && read_change.is_none() {
        return Ok(false);
    }
    let change = if let Some(change) = read_change {
        ToolChange::Read { change }
    } else if after.tool_turns.len() == before.tool_turns.len() + 1 {
        let turn = after.tool_turns.last().ok_or(AiError::Conflict)?.clone();
        let active = before.active_attempt.as_ref().ok_or(AiError::Conflict)?;
        let index = after
            .attempt_evidence()
            .iter()
            .position(|fact| fact.attempt_id() == active.prepared().identity())
            .ok_or(AiError::Conflict)?;
        ToolChange::Admit {
            turn,
            evidence: after.attempt_evidence()[index].clone(),
            usage: after.attempt_usage()[index].clone(),
        }
    } else if after.tool_turns.len() == before.tool_turns.len() {
        let turn = before.tool_turns.last().ok_or(AiError::Conflict)?;
        let index = turn
            .steps
            .iter()
            .position(|step| step.result.is_none())
            .ok_or(AiError::Conflict)?;
        let step = after
            .tool_turns
            .last()
            .and_then(|turn| turn.steps.get(index))
            .ok_or(AiError::Conflict)?;
        if before.state == RunState::AwaitingReconciliation && after.state == RunState::ToolsPending
        {
            ToolChange::Recover {
                call_id: step.call.id().into(),
            }
        } else if let Some(result) = &step.result {
            ToolChange::Result {
                call_id: step.call.id().into(),
                actor: step.actor.clone().ok_or(AiError::Conflict)?,
                result: result.value.clone(),
            }
        } else if step.unknown {
            ToolChange::Unknown {
                call_id: step.call.id().into(),
            }
        } else if step.prepared.is_none() && step.actor.is_some() {
            ToolChange::AuthorizeRead {
                call_id: step.call.id().into(),
                actor: step.actor.clone().ok_or(AiError::Conflict)?,
            }
        } else {
            ToolChange::Prepare {
                prepared: step.prepared.clone().ok_or(AiError::Conflict)?,
            }
        }
    } else {
        return Err(AiError::Conflict);
    };
    let mut expected = before.clone();
    apply(
        &mut expected,
        &ToolCheckpoint::new(change, after.last_observed_unix_ms)?,
    )?;
    if &expected != after {
        return Err(AiError::Conflict);
    }
    Ok(true)
}

/// Cancellation does not certify that a persisted mutating invocation failed.
pub(crate) fn cancel(record: &mut RunRecord, now: u64) -> AiResult<()> {
    if record.state != RunState::ToolsPending || record.cancel_requested {
        return Err(AiError::Conflict);
    }
    record.observe_recovery(now)?;
    record.cancel_requested = true;
    if let Some(step) = record
        .tool_turns
        .last_mut()
        .and_then(|turn| turn.steps.iter_mut().find(|step| step.result.is_none()))
        .filter(|step| step.prepared.is_some())
    {
        step.unknown = true;
        record.state = RunState::AwaitingReconciliation;
    } else {
        record.state = RunState::Cancelled;
    }
    record.mark();
    record.validate()
}
