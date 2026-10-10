//! Exact pure read transitions; the coordinator supplies no persisted authorization grant.
use super::{
    attempt::{ReadAttempt, ReadPhase},
    transcript::ToolKind,
};
use crate::{
    AiError, AiResult,
    flow::codec::Validated,
    flow::{
        MAX_OPERATIONS, OperationKind, OperationOutcome, OperationPhase, OperationStamp,
        OwnerIdentity, RunRecord, RunState,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum ReadChange {
    Start {
        call_id: String,
        actor: OwnerIdentity,
        ordinal: u32,
    },
    Stopped {
        call_id: String,
        ordinal: u32,
    },
    Retry {
        call_id: String,
        prior_ordinal: u32,
        stamp: OperationStamp,
    },
    Wake {
        call_id: String,
        ordinal: u32,
        stamp: OperationStamp,
    },
    Result {
        call_id: String,
        ordinal: u32,
        actor: OwnerIdentity,
        result: Value,
    },
}
impl ReadChange {
    pub fn validate(&self) -> AiResult<()> {
        let (call, ordinal) = match self {
            Self::Start {
                call_id,
                actor,
                ordinal,
            } => {
                actor.validate_identity()?;
                (call_id, *ordinal)
            }
            Self::Stopped { call_id, ordinal } => (call_id, *ordinal),
            Self::Wake {
                call_id,
                ordinal,
                stamp,
            } => {
                stamp.validate()?;
                (call_id, *ordinal)
            }
            Self::Retry {
                call_id,
                prior_ordinal,
                stamp,
            } => {
                stamp.validate()?;
                (
                    call_id,
                    prior_ordinal
                        .checked_add(1)
                        .ok_or(AiError::BudgetExhausted)?,
                )
            }
            Self::Result {
                call_id,
                ordinal,
                actor,
                result,
            } => {
                actor.validate_identity()?;
                crate::request::bounded_value(result, crate::request::MAX_TOOL_BYTES)?;
                (call_id, *ordinal)
            }
        };
        if !crate::request::valid_name(call, 128) || !(1..=32).contains(&ordinal) {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
fn finish(record: &mut RunRecord, key: Option<&str>, now: u64) -> AiResult<()> {
    if let Some(key) = key {
        let stamp = record
            .operations
            .iter_mut()
            .find(|stamp| stamp.key == key)
            .ok_or(AiError::Conflict)?;
        if stamp.kind != OperationKind::Reconcile
            || stamp.phase != OperationPhase::Pending
            || now < stamp.admitted_at_ms
        {
            return Err(AiError::Conflict);
        }
        stamp.phase = OperationPhase::Finished {
            category: OperationOutcome::Unresolved,
            finished_at_ms: now,
        };
    }
    Ok(())
}
pub(crate) fn apply(record: &mut RunRecord, change: &ReadChange, now: u64) -> AiResult<()> {
    change.validate()?;
    if record.state != RunState::ToolsPending || record.cancel_requested {
        return Err(AiError::Conflict);
    }
    let old = record
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .cloned()
        .ok_or(AiError::Conflict)?;
    if old.kind != ToolKind::Read || old.prepared.is_some() || old.unknown {
        return Err(AiError::Conflict);
    }
    record.observe(now)?;
    let step = record
        .tool_turns
        .last_mut()
        .and_then(|turn| turn.steps.iter_mut().find(|step| step.result.is_none()))
        .ok_or(AiError::Conflict)?;
    match change {
        ReadChange::Wake {
            call_id,
            ordinal,
            stamp,
        } => {
            let attempt = step.read_attempt.as_ref().ok_or(AiError::Conflict)?;
            let original_key = attempt.operation_key.as_ref().ok_or(AiError::Conflict)?;
            if step.call.id() != call_id
                || attempt.phase != ReadPhase::Scheduled
                || attempt.ordinal != *ordinal
                || stamp.kind != OperationKind::Reconcile
                || stamp.phase
                    != (OperationPhase::Finished {
                        category: OperationOutcome::Unresolved,
                        finished_at_ms: now,
                    })
                || stamp.admitted_at_ms != now
                || stamp.checkpoint != record.checkpoint
                || record.operations.len() >= MAX_OPERATIONS
                || record.operations.iter().any(|old| old.key == stamp.key)
                || !record.operations.iter().any(|old| {
                    &old.key == original_key
                        && old.kind == OperationKind::Reconcile
                        && old.phase == OperationPhase::Pending
                })
            {
                return Err(AiError::Conflict);
            }
            record.counters.ticks = record
                .counters
                .ticks
                .checked_add(1)
                .ok_or(AiError::BudgetExhausted)?;
            record.operations.push(stamp.clone());
        }
        ReadChange::Start {
            call_id,
            actor,
            ordinal,
        } => {
            if step.call.id() != call_id || step.actor.as_ref().is_some_and(|old| old != actor) {
                return Err(AiError::Conflict);
            }
            match &step.read_attempt {
                None if step.actor.is_none() && *ordinal == 1 => (),
                Some(attempt)
                    if attempt.phase == ReadPhase::Scheduled && attempt.ordinal == *ordinal => {}
                _ => return Err(AiError::Conflict),
            }
            let operation_key = step
                .read_attempt
                .as_ref()
                .and_then(|attempt| attempt.operation_key.clone());
            step.actor = Some(actor.clone());
            step.read_attempt = Some(ReadAttempt {
                ordinal: *ordinal,
                phase: ReadPhase::Started,
                operation_key,
            });
        }
        ReadChange::Stopped { call_id, ordinal } => {
            let attempt = step.read_attempt.as_mut().ok_or(AiError::Conflict)?;
            if step.call.id() != call_id
                || attempt.phase != ReadPhase::Started
                || attempt.ordinal != *ordinal
            {
                return Err(AiError::Conflict);
            }
            attempt.phase = ReadPhase::Unresolved;
            let key = attempt.operation_key.clone();
            finish(record, key.as_deref(), now)?;
        }
        ReadChange::Retry {
            call_id,
            prior_ordinal,
            stamp,
        } => {
            if step.call.id() != call_id
                || (step.actor.is_none() && (*prior_ordinal != 0 || step.read_attempt.is_some()))
                || stamp.kind != OperationKind::Reconcile
                || stamp.phase != OperationPhase::Pending
                || stamp.checkpoint != record.checkpoint
                || stamp.admitted_at_ms != now
                || record.operations.len() >= MAX_OPERATIONS
                || record.operations.iter().any(|old| old.key == stamp.key)
            {
                return Err(AiError::Conflict);
            }
            let actual = match &step.read_attempt {
                None => {
                    if step.actor.is_none() {
                        0
                    } else {
                        1
                    }
                }
                Some(attempt)
                    if matches!(attempt.phase, ReadPhase::Started | ReadPhase::Unresolved) =>
                {
                    attempt.ordinal
                }
                _ => return Err(AiError::Conflict),
            };
            if actual != *prior_ordinal {
                return Err(AiError::Conflict);
            }
            step.read_attempt = Some(ReadAttempt {
                ordinal: actual.checked_add(1).ok_or(AiError::BudgetExhausted)?,
                phase: ReadPhase::Scheduled,
                operation_key: Some(stamp.key.clone()),
            });
            record.counters.ticks = record
                .counters
                .ticks
                .checked_add(1)
                .ok_or(AiError::BudgetExhausted)?;
            record.operations.push(stamp.clone());
        }
        ReadChange::Result {
            call_id,
            ordinal,
            actor,
            result,
        } => {
            let attempt = step.read_attempt.as_ref().ok_or(AiError::Conflict)?;
            if step.call.id() != call_id
                || step.actor.as_ref() != Some(actor)
                || attempt.ordinal != *ordinal
                || attempt.phase != ReadPhase::Started
            {
                return Err(AiError::Conflict);
            }
            let key = attempt.operation_key.clone();
            step.read_attempt.as_mut().ok_or(AiError::Conflict)?.phase = ReadPhase::Completed;
            step.result = Some(super::transcript::ToolResult {
                value: result.clone(),
            });
            record.checkpoint = record
                .checkpoint
                .checked_add(1)
                .ok_or(AiError::BudgetExhausted)?;
            record.counters.tool_calls = record
                .counters
                .tool_calls
                .checked_add(1)
                .ok_or(AiError::BudgetExhausted)?;
            record.counters.ticks = record
                .counters
                .ticks
                .checked_add(1)
                .ok_or(AiError::BudgetExhausted)?;
            finish(record, key.as_deref(), now)?;
        }
    }
    Ok(())
}
pub(crate) fn derive(after: &RunRecord, before: &RunRecord) -> AiResult<Option<ReadChange>> {
    let Some(old) = before.tool_turns.last().and_then(|turn| turn.next()) else {
        return Ok(None);
    };
    if old.kind != ToolKind::Read || after.tool_turns.len() != before.tool_turns.len() {
        return Ok(None);
    }
    let index = before
        .tool_turns
        .last()
        .ok_or(AiError::Conflict)?
        .steps
        .iter()
        .position(|step| step.result.is_none())
        .ok_or(AiError::Conflict)?;
    let new = after
        .tool_turns
        .last()
        .and_then(|turn| turn.steps.get(index))
        .ok_or(AiError::Conflict)?;
    if old.read_attempt == new.read_attempt {
        if let Some(attempt) = &old.read_attempt
            && attempt.phase == ReadPhase::Scheduled
            && after.operations.len()
                == before
                    .operations
                    .len()
                    .checked_add(1)
                    .ok_or(AiError::BudgetExhausted)?
            && after.operations.last().is_some_and(|stamp| {
                stamp.kind == OperationKind::Reconcile
                    && stamp.phase
                        == (OperationPhase::Finished {
                            category: OperationOutcome::Unresolved,
                            finished_at_ms: after.last_observed_unix_ms,
                        })
            })
        {
            return Ok(Some(ReadChange::Wake {
                call_id: old.call.id().into(),
                ordinal: attempt.ordinal,
                stamp: after.operations.last().cloned().ok_or(AiError::Conflict)?,
            }));
        }
        return Ok(None);
    }
    let attempt = new.read_attempt.as_ref().ok_or(AiError::Conflict)?;
    Ok(Some(match attempt.phase {
        ReadPhase::Started => ReadChange::Start {
            call_id: new.call.id().into(),
            actor: new.actor.clone().ok_or(AiError::Conflict)?,
            ordinal: attempt.ordinal,
        },
        ReadPhase::Unresolved => ReadChange::Stopped {
            call_id: new.call.id().into(),
            ordinal: attempt.ordinal,
        },
        ReadPhase::Scheduled => ReadChange::Retry {
            call_id: new.call.id().into(),
            prior_ordinal: old
                .read_attempt
                .as_ref()
                .map_or(if old.actor.is_none() { 0 } else { 1 }, |attempt| {
                    attempt.ordinal
                }),
            stamp: after.operations.last().cloned().ok_or(AiError::Conflict)?,
        },
        ReadPhase::Completed => ReadChange::Result {
            call_id: new.call.id().into(),
            ordinal: attempt.ordinal,
            actor: new.actor.clone().ok_or(AiError::Conflict)?,
            result: new.result.as_ref().ok_or(AiError::Conflict)?.value.clone(),
        },
    }))
}

/// A decoded phase is coherence metadata; it never supplies a current actor or grant.
pub(crate) fn validate_record(record: &RunRecord) -> AiResult<()> {
    let mut keys = std::collections::BTreeSet::new();
    for turn in &record.tool_turns {
        for (index, step) in turn.steps.iter().enumerate() {
            let Some(attempt) = &step.read_attempt else {
                continue;
            };
            if let Some(key) = &attempt.operation_key {
                if !keys.insert(key) {
                    return Err(AiError::InvalidRequest);
                }
                let stamp = record
                    .operations
                    .iter()
                    .find(|stamp| &stamp.key == key)
                    .ok_or(AiError::InvalidRequest)?;
                let checkpoint = turn
                    .source
                    .step()
                    .checked_add(u32::try_from(index).map_err(|_| AiError::InvalidRequest)?)
                    .ok_or(AiError::InvalidRequest)?;
                let expected = match attempt.phase {
                    ReadPhase::Scheduled | ReadPhase::Started => {
                        stamp.phase == OperationPhase::Pending
                    }
                    ReadPhase::Unresolved | ReadPhase::Completed => matches!(
                        stamp.phase,
                        OperationPhase::Finished {
                            category: OperationOutcome::Unresolved,
                            ..
                        }
                    ),
                };
                if stamp.kind != OperationKind::Reconcile
                    || stamp.checkpoint != checkpoint
                    || !expected
                {
                    return Err(AiError::InvalidRequest);
                }
            } else if attempt.ordinal != 1 || attempt.phase == ReadPhase::Scheduled {
                return Err(AiError::InvalidRequest);
            }
        }
    }
    Ok(())
}
