//! Public read recovery admits an original bounded ordinal through existing durable actions.
use super::{
    attempt::ReadPhase,
    checkpoint::{PREPARE_TOOL_ACTION, ToolChange, ToolCheckpoint},
    read_checkpoint::ReadChange,
    transcript::ToolKind,
};
use crate::{
    AiError, AiResult, ExecutionDeadline,
    flow::{
        AiRun, HostState, MAX_OPERATIONS, OperationKind, OperationOutcome, OperationPhase,
        OperationStamp, OwnerIdentity, RunHandle, RunRecord, RunState, codec::Validated,
        committed_attempt, map_error,
    },
};
use std::sync::{Arc, atomic::Ordering};

pub(crate) fn handles(record: &RunRecord) -> bool {
    record.state() == &RunState::ToolsPending
        && record
            .tool_turns
            .last()
            .and_then(|turn| turn.next())
            .is_some_and(|step| step.kind == ToolKind::Read)
}
pub(crate) async fn resume(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    actor: &rom::Actor,
    run: &RunHandle,
    revision: u64,
    key: &str,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    if !crate::request::valid_name(key, 128) || revision == 0 {
        return Err(AiError::InvalidRequest);
    }
    let requester = OwnerIdentity::from_actor(actor)?;
    let initial = runtime
        .read::<AiRun>(&state.service, &run.0)
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::InvalidRequest)?
        .record()?;
    let prepared = initial
        .active_attempt()
        .ok_or(AiError::Conflict)?
        .prepared()
        .clone();
    for _ in 0..2 {
        execution.remaining_ms()?;
        let (snapshot, record) = committed_attempt(state, runtime, &run.0, &prepared).await?;
        state.authority.inspect(actor, record.owner())?;
        if let Err(error) =
            super::worker::authorize_transcript(state, runtime, &record, execution).await
        {
            if error == AiError::Denied {
                return refuse_read(state, runtime, &record, error, execution).await;
            }
            return Err(error);
        }
        if !handles(&record) || record.cancel_requested() {
            return Err(AiError::Conflict);
        }
        super::worker::registry(state, &record)?;
        let step = record
            .tool_turns
            .last()
            .and_then(|turn| turn.next())
            .ok_or(AiError::Conflict)?;
        if state.clock.now_unix_ms() >= record.expires_at_unix_ms() {
            return refuse_read(
                state,
                runtime,
                &record,
                AiError::DeadlineExceeded,
                execution,
            )
            .await;
        }
        if let Err(error) =
            super::worker::current_tool(state, runtime, &record, &step.call, step.actor.as_ref())
                .await
        {
            if error == AiError::Denied {
                return refuse_read(state, runtime, &record, error, execution).await;
            }
            return Err(error);
        }
        if let Some(old) = record.operations.iter().find(|stamp| stamp.key == key) {
            if old.kind != OperationKind::Reconcile
                || old.requester != requester
                || old.original_expected_revision != revision
            {
                return Err(map_error(rom::Error::IdentityMismatch));
            }
            return Ok(());
        }
        if snapshot.revision != revision {
            return Err(AiError::Conflict);
        }
        if step.prepared.is_some() || step.unknown {
            return Err(AiError::Conflict);
        }
        if state
            .read_activity
            .is_active(record.run_id(), prepared.identity(), step.call.id())?
        {
            return Err(AiError::Conflict);
        }
        let ordinal = match &step.read_attempt {
            None => {
                if step.actor.is_none() {
                    0
                } else {
                    1
                }
            }
            Some(attempt)
                if matches!(
                    attempt.phase,
                    ReadPhase::Started | ReadPhase::Unresolved | ReadPhase::Scheduled
                ) =>
            {
                attempt.ordinal
            }
            _ => return Err(AiError::Conflict),
        };
        let wake = step
            .read_attempt
            .as_ref()
            .is_some_and(|attempt| attempt.phase == ReadPhase::Scheduled);
        if record.operations.len() >= MAX_OPERATIONS
            || record.counters().ticks() >= record.policy().limits().ticks
            || (!wake && ordinal >= 32)
        {
            return refuse_read(state, runtime, &record, AiError::BudgetExhausted, execution).await;
        }
        super::worker::settled_original(state, runtime, &record, &prepared).await?;
        let nonce = state
            .next_nonce
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_add(1)
            })
            .map_err(|_| AiError::Closed)?;
        let now = state.clock.now_unix_ms();
        let stamp = OperationStamp {
            requester: requester.clone(),
            key: key.into(),
            kind: OperationKind::Reconcile,
            original_expected_revision: revision,
            checkpoint: record.checkpoint(),
            admitted_at_ms: now,
            nonce,
            phase: if wake {
                OperationPhase::Finished {
                    category: OperationOutcome::Unresolved,
                    finished_at_ms: now,
                }
            } else {
                OperationPhase::Pending
            },
        };
        stamp.validate()?;
        let command_key =
            serde_json::json!(["ai-flow-operation-v1", run.0, requester, key, "begin-read"])
                .to_string();
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    &run.0,
                    PREPARE_TOOL_ACTION,
                    ToolCheckpoint::new(
                        ToolChange::Read {
                            change: if wake {
                                ReadChange::Wake {
                                    call_id: step.call.id().into(),
                                    ordinal,
                                    stamp,
                                }
                            } else {
                                ReadChange::Retry {
                                    call_id: step.call.id().into(),
                                    prior_ordinal: ordinal,
                                    stamp,
                                }
                            },
                        },
                        now,
                    )?,
                )
                .at_revision(revision)
                .idempotency(&command_key),
            )
            .await;
        match result {
            Err(rom::Error::Conflict | rom::Error::IdentityMismatch) => continue,
            other => return other.map(|_| ()).map_err(map_error),
        }
    }
    Err(AiError::Conflict)
}

/// Publish a refusal under current owner authority without executing or releasing an uncertain effect.
async fn refuse_read(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    frozen: &RunRecord,
    error: AiError,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    let prepared = frozen.active_attempt().ok_or(AiError::Conflict)?.prepared();
    let call = frozen
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .ok_or(AiError::Conflict)?
        .call
        .id();
    for _ in 0..2 {
        let (snapshot, record) =
            committed_attempt(state, runtime, frozen.run_id(), prepared).await?;
        if record.state() == &RunState::Failed && record.failure() == Some(&error) {
            return Ok(());
        }
        if record.checkpoint() != frozen.checkpoint()
            || !handles(&record)
            || record.cancel_requested()
        {
            return Err(AiError::Conflict);
        }
        let step = record
            .tool_turns
            .last()
            .and_then(|turn| turn.next())
            .ok_or(AiError::Conflict)?;
        if step.call.id() != call || step.prepared.is_some() || step.unknown {
            return Err(AiError::Conflict);
        }
        super::worker::registry(state, &record)?;
        let now = state.clock.now_unix_ms();
        let retry_exhausted = record.operations.len() >= MAX_OPERATIONS
            || record.counters().ticks() >= record.policy().limits().ticks
            || step.read_attempt.as_ref().is_some_and(|attempt| {
                attempt.ordinal >= 32 && attempt.phase != ReadPhase::Scheduled
            });
        if error != AiError::Denied
            && !(error == AiError::DeadlineExceeded && now >= record.expires_at_unix_ms())
            && !(error == AiError::BudgetExhausted && retry_exhausted)
        {
            return Err(AiError::InvalidRequest);
        }
        if state
            .read_activity
            .is_active(record.run_id(), prepared.identity(), step.call.id())?
        {
            return Err(AiError::Conflict);
        }
        if error == AiError::BudgetExhausted {
            super::worker::authorize_transcript(state, runtime, &record, execution).await?;
            super::worker::current_tool(state, runtime, &record, &step.call, step.actor.as_ref())
                .await?;
        }
        crate::flow::dispatch_actor(state.clone(), runtime, record.owner()).await?;
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    record.run_id(),
                    super::checkpoint::HOLD_TOOL_ACTION,
                    ToolCheckpoint::new(
                        ToolChange::Failed {
                            error: error.clone(),
                        },
                        now,
                    )?,
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-read-refuse-{}-{}-{}",
                    prepared.identity(),
                    record.checkpoint(),
                    snapshot.revision
                )),
            )
            .await;
        match result {
            Err(rom::Error::Conflict) => continue,
            other => return other.map(|_| ()).map_err(map_error),
        }
    }
    Err(AiError::Conflict)
}
