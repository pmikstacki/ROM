//! Current-authority tool coordinator over the existing Resource receipt pipeline.
use super::{
    ReadContext,
    checkpoint::{self, ToolChange, ToolCheckpoint},
    transcript::{ToolKind, ToolStep, ToolTurn},
};
use crate::{
    AiError, AiResult, AttemptEvidence, ExecutionDeadline, ToolCall, Usage,
    flow::{
        HostState, OwnerIdentity, RunRecord, RunState, committed_attempt, dispatch_attempt,
        map_error, settle_record,
    },
};
use std::{sync::Arc, time::Duration};

pub(super) async fn current_tool(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &RunRecord,
    call: &ToolCall,
    persisted: Option<&OwnerIdentity>,
) -> AiResult<rom::Actor> {
    let active = record.active_attempt().ok_or(AiError::Conflict)?;
    dispatch_attempt(state.clone(), runtime, record.owner(), active.prepared()).await?;
    current_call(state, runtime, record.owner(), call, persisted).await
}
pub(super) async fn current_call(
    state: &HostState,
    runtime: &rom::Runtime,
    owner: &OwnerIdentity,
    call: &ToolCall,
    persisted: Option<&OwnerIdentity>,
) -> AiResult<rom::Actor> {
    let authority = state.authority.clone();
    let owner = owner.clone();
    let call = call.clone();
    let actor = runtime
        .establish_actor(move |reads| {
            let actor = authority.resolve(&owner, reads)?;
            if !owner.matches(&actor) {
                return Err(rom::Error::Denied);
            }
            authority.tool_actor(&actor, &owner, &call, reads)
        })
        .await
        .map_err(map_error)?;
    if persisted.is_some_and(|identity| !identity.matches(&actor)) {
        return Err(AiError::Denied);
    }
    Ok(actor)
}
pub(super) fn registry<'a>(
    state: &'a HostState,
    record: &RunRecord,
) -> AiResult<&'a crate::ToolRegistry> {
    let registry = state
        .tools
        .as_deref()
        .ok_or(AiError::UnsupportedCapability)?;
    if record.tool_registry_version != Some(registry.version) {
        return Err(AiError::UnsupportedCapability);
    }
    registry.validate_requested(record.request())?;
    Ok(registry)
}

pub(crate) async fn admit(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    run: &str,
    prepared: &crate::PreparedAttempt,
    calls: Vec<ToolCall>,
    evidence: AttemptEvidence,
    usage: Usage,
) -> AiResult<()> {
    let (snapshot, record) = committed_attempt(state, runtime, run, prepared).await?;
    let registry = registry(state, &record)?;
    dispatch_attempt(state.clone(), runtime, record.owner(), prepared).await?;
    let mut steps = Vec::new();
    for call in calls {
        let kind = if registry.reads.contains_key(call.name()) {
            ToolKind::Read
        } else if registry.actions.contains_key(call.name()) {
            ToolKind::Action
        } else {
            return Err(AiError::UnsupportedCapability);
        };
        steps.push(ToolStep {
            call,
            kind,
            actor: None,
            prepared: None,
            result: None,
            unknown: false,
            read_attempt: None,
        });
    }
    let turn = ToolTurn {
        source: record
            .active_attempt()
            .ok_or(AiError::Conflict)?
            .key()
            .clone(),
        registry_version: registry.version,
        steps,
    };
    runtime
        .execute(
            &state.service,
            rom::Command::action(
                run,
                checkpoint::ADMIT_TOOL_BATCH,
                ToolCheckpoint::new(
                    ToolChange::Admit {
                        turn,
                        evidence,
                        usage,
                    },
                    state.clock.now_unix_ms(),
                )?,
            )
            .at_revision(snapshot.revision)
            .idempotency(&format!("ai-tools-{}", prepared.identity())),
        )
        .await
        .map_err(map_error)?;
    // Provider knowledge commits first. Original account settlement is retried before any tool I/O.
    let (_, record) = committed_attempt(state, runtime, run, prepared).await?;
    settle_record(state, runtime, &record, None).await
}

pub(crate) async fn tick(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    frozen: &RunRecord,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    let read_call = frozen
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .filter(|step| step.kind == ToolKind::Read && step.prepared.is_none() && !step.unknown)
        .map(|step| step.call.id().to_owned());
    let result = tick_inner(state, runtime, frozen, execution).await;
    if let (Some(call), Err(error)) = (read_call, &result)
        && checkpoint::definite_read_failure(error)
    {
        fail_tool(state, runtime, frozen, error.clone(), Some(&call)).await?;
    }
    result
}
async fn tick_inner(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    frozen: &RunRecord,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    let prepared = frozen.active_attempt().ok_or(AiError::Conflict)?.prepared();
    let (mut snapshot, mut record) =
        committed_attempt(state, runtime, frozen.run_id(), prepared).await?;
    if record.state() != &RunState::ToolsPending || record.cancel_requested() {
        return Err(AiError::Conflict);
    }
    let registry = registry(state, &record)?;
    dispatch_attempt(state.clone(), runtime, record.owner(), prepared).await?;
    settled_original(state, runtime, &record, prepared).await?;
    let Some(step) = record
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .cloned()
    else {
        return Err(AiError::Conflict);
    };
    let mut actor = current_tool(state, runtime, &record, &step.call, step.actor.as_ref()).await?;
    let mut read_lease = None;
    let result = match step.kind {
        ToolKind::Read => {
            let registered = registry
                .reads
                .get(step.call.name())
                .ok_or(AiError::UnsupportedCapability)?;
            let ordinal = match &step.read_attempt {
                None if step.actor.is_none() => 1,
                Some(attempt) if attempt.phase == super::attempt::ReadPhase::Scheduled => {
                    attempt.ordinal
                }
                _ => return Err(AiError::UnknownOutcome),
            };
            let physical_lease = state.read_activity.acquire(
                record.run_id(),
                prepared.identity(),
                step.call.id(),
            )?;
            runtime
                .execute(
                    &state.service,
                    rom::Command::action(
                        record.run_id(),
                        checkpoint::PREPARE_TOOL_ACTION,
                        ToolCheckpoint::new(
                            ToolChange::Read {
                                change: super::read_checkpoint::ReadChange::Start {
                                    call_id: step.call.id().into(),
                                    actor: OwnerIdentity::from_actor(&actor)?,
                                    ordinal,
                                },
                            },
                            state.clock.now_unix_ms(),
                        )?,
                    )
                    .at_revision(snapshot.revision)
                    .idempotency(&format!(
                        "ai-tool-read-start-{}-{}-{}",
                        record.run_id(),
                        step.call.id(),
                        ordinal
                    )),
                )
                .await
                .map_err(map_error)?;
            (_, record) = committed_attempt(state, runtime, record.run_id(), prepared).await?;
            actor = current_tool(
                state,
                runtime,
                &record,
                &step.call,
                Some(&OwnerIdentity::from_actor(&actor)?),
            )
            .await?;
            let context = ReadContext {
                runtime: runtime.clone(),
                actor: actor.clone(),
                deadline: execution.clone(),
                lease: Some(physical_lease.clone()),
            };
            let outcome = tokio::time::timeout(
                Duration::from_millis(execution.remaining_ms()?),
                (registered.read)(context, step.call.arguments().clone()),
            )
            .await
            .map_err(|_| AiError::UnknownOutcome)
            .and_then(|result| result);
            let result = match outcome {
                Ok(result) => result,
                Err(error) => {
                    for _ in 0..2 {
                        let (fresh, current) =
                            committed_attempt(state, runtime, record.run_id(), prepared).await?;
                        if current.state() != &RunState::ToolsPending || current.cancel_requested()
                        {
                            return Err(error);
                        }
                        // No result is exposed and no charge changes. Persist the exact stopped ordinal.
                        crate::flow::dispatch_actor(state.clone(), runtime, current.owner())
                            .await?;
                        let stopped = runtime
                            .execute(
                                &state.service,
                                rom::Command::action(
                                    current.run_id(),
                                    checkpoint::HOLD_TOOL_ACTION,
                                    ToolCheckpoint::new(
                                        ToolChange::Read {
                                            change: super::read_checkpoint::ReadChange::Stopped {
                                                call_id: step.call.id().into(),
                                                ordinal,
                                            },
                                        },
                                        state.clock.now_unix_ms(),
                                    )?,
                                )
                                .at_revision(fresh.revision)
                                .idempotency(&format!(
                                    "ai-tool-read-stopped-{}-{}-{}",
                                    current.run_id(),
                                    step.call.id(),
                                    ordinal
                                )),
                            )
                            .await;
                        match stopped {
                            Err(rom::Error::Conflict) => continue,
                            Err(commit_error) => return Err(map_error(commit_error)),
                            Ok(_) => return Err(error),
                        }
                    }
                    return Err(AiError::Conflict);
                }
            };
            // The coordinator keeps this lease through current authorization and durable publication.
            read_lease = Some(physical_lease);
            result
        }
        ToolKind::Action => {
            let registered = registry
                .actions
                .get(step.call.name())
                .ok_or(AiError::UnsupportedCapability)?;
            let command = if let Some(command) = step.prepared {
                command
            } else {
                let epoch = runtime
                    .retry_epochs(&state.service)
                    .await
                    .map_err(map_error)?
                    .current;
                let command = (registered.prepare)(&step.call, record.run_id(), &actor, epoch)?;
                runtime
                    .execute(
                        &state.service,
                        rom::Command::action(
                            record.run_id(),
                            checkpoint::PREPARE_TOOL_ACTION,
                            ToolCheckpoint::new(
                                ToolChange::Prepare {
                                    prepared: command.clone(),
                                },
                                state.clock.now_unix_ms(),
                            )?,
                        )
                        .at_revision(snapshot.revision)
                        .idempotency(&format!(
                            "ai-tool-prepare-{}-{}",
                            record.run_id(),
                            step.call.id()
                        )),
                    )
                    .await
                    .map_err(map_error)?;
                (snapshot, record) =
                    committed_attempt(state, runtime, record.run_id(), prepared).await?;
                command
            };
            registered.validate_prepared(
                registry.version,
                record.run_id(),
                &step.call,
                &command,
            )?;
            actor = current_tool(state, runtime, &record, &step.call, Some(&command.actor)).await?;
            // Invoke exact original input. Runtime checks the original receipt before current codec/revision.
            let outcome = tokio::time::timeout(
                Duration::from_millis(execution.remaining_ms()?),
                runtime.invoke(&actor, command.invocation.clone()),
            )
            .await;
            match outcome {
                Ok(Ok(row)) => serde_json::json!({"id":row.key.id,"revision":row.revision}),
                _ => {
                    runtime
                        .execute(
                            &state.service,
                            rom::Command::action(
                                record.run_id(),
                                checkpoint::HOLD_TOOL_ACTION,
                                ToolCheckpoint::new(
                                    ToolChange::Unknown {
                                        call_id: step.call.id().into(),
                                    },
                                    state.clock.now_unix_ms(),
                                )?,
                            )
                            .at_revision(snapshot.revision)
                            .idempotency(&format!(
                                "ai-tool-hold-{}-{}",
                                record.run_id(),
                                step.call.id()
                            )),
                        )
                        .await
                        .map_err(map_error)?;
                    return Err(AiError::UnknownOutcome);
                }
            }
        }
    };
    let actor = current_tool(
        state,
        runtime,
        &record,
        &step.call,
        Some(&OwnerIdentity::from_actor(&actor)?),
    )
    .await?;
    let (fresh, fresh_record) =
        committed_attempt(state, runtime, record.run_id(), prepared).await?;
    if fresh_record.checkpoint() != record.checkpoint()
        || fresh_record.state() != &RunState::ToolsPending
    {
        return Err(AiError::Conflict);
    }
    current_tool(
        state,
        runtime,
        &fresh_record,
        &step.call,
        Some(&OwnerIdentity::from_actor(&actor)?),
    )
    .await?;
    let _physical_lease = read_lease;
    runtime
        .execute(
            &state.service,
            rom::Command::action(
                record.run_id(),
                checkpoint::COMMIT_TOOL_RESULT,
                ToolCheckpoint::new(
                    if step.kind == ToolKind::Read {
                        ToolChange::Read {
                            change: super::read_checkpoint::ReadChange::Result {
                                call_id: step.call.id().into(),
                                ordinal: record
                                    .tool_turns
                                    .last()
                                    .and_then(|turn| turn.next())
                                    .and_then(|step| step.read_attempt.as_ref())
                                    .ok_or(AiError::Conflict)?
                                    .ordinal,
                                actor: OwnerIdentity::from_actor(&actor)?,
                                result,
                            },
                        }
                    } else {
                        ToolChange::Result {
                            call_id: step.call.id().into(),
                            actor: OwnerIdentity::from_actor(&actor)?,
                            result,
                        }
                    },
                    state.clock.now_unix_ms(),
                )?,
            )
            .at_revision(fresh.revision)
            .idempotency(&format!(
                "ai-tool-result-{}-{}",
                record.run_id(),
                step.call.id()
            )),
        )
        .await
        .map_err(map_error)?;
    Ok(())
}

/// Schedule one currently authorized replay of the exact persisted command; no lookup or invocation here.
pub(crate) async fn resume(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    record: &RunRecord,
    key: &str,
) -> AiResult<()> {
    let installed = registry(state, record)?;
    if record.cancel_requested() {
        return Err(AiError::Conflict);
    }
    let active = record.active_attempt().ok_or(AiError::Conflict)?;
    let (snapshot, current) =
        committed_attempt(state, runtime, record.run_id(), active.prepared()).await?;
    if current.checkpoint() != record.checkpoint()
        || current.state() != &RunState::AwaitingReconciliation
    {
        return Err(AiError::Conflict);
    }
    let step = current
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .ok_or(AiError::Conflict)?;
    let command = step.prepared.as_ref().ok_or(AiError::Conflict)?;
    installed
        .actions
        .get(step.call.name())
        .ok_or(AiError::UnsupportedCapability)?
        .validate_prepared(installed.version, current.run_id(), &step.call, command)?;
    current_tool(state, runtime, &current, &step.call, Some(&command.actor)).await?;
    settle_record(state, runtime, &current, None).await?;
    runtime
        .execute(
            &state.service,
            rom::Command::action(
                current.run_id(),
                checkpoint::HOLD_TOOL_ACTION,
                ToolCheckpoint::new(
                    ToolChange::Recover {
                        call_id: step.call.id().into(),
                    },
                    state.clock.now_unix_ms(),
                )?,
            )
            .at_revision(snapshot.revision)
            .idempotency(key),
        )
        .await
        .map_err(map_error)?;
    Ok(())
}

/// Reauthorize retained private results without replaying tool callbacks or domain actions.
pub(crate) async fn authorize_transcript(
    state: &HostState,
    runtime: &rom::Runtime,
    record: &RunRecord,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    if record.tool_turns.is_empty() {
        return Ok(());
    }
    tokio::time::timeout(Duration::from_millis(execution.remaining_ms()?), async {
        let installed = registry(state, record)?;
        for turn in &record.tool_turns {
            let ledger = runtime
                .read::<crate::flow::AiBudget>(&state.service, turn.source.account_window())
                .await
                .map_err(map_error)?
                .value
                .ok_or(AiError::Denied)?
                .record()?;
            let entry = ledger
                .entries()
                .iter()
                .find(|entry| entry.key() == &turn.source)
                .ok_or(AiError::Denied)?;
            entry.prepared().validate()?;
            if entry.prepared().request()
                != &record.effective_request(turn.source.attempt_ordinal())?
                || entry.prepared().policy() != record.policy()
            {
                return Err(AiError::Denied);
            }
            let index = record
                .attempt_evidence()
                .iter()
                .position(|fact| fact.attempt_id() == entry.prepared().identity())
                .ok_or(AiError::Denied)?;
            let cost = record.attempt_usage()[index]
                .cost
                .ok_or(AiError::UnknownOutcome)?;
            if entry.status() != &(crate::flow::ReservationStatus::Settled { actual_cost: cost }) {
                return Err(AiError::Denied);
            }
            for step in turn.steps.iter().filter(|step| step.result.is_some()) {
                let actor = step.actor.as_ref().ok_or(AiError::Denied)?;
                if let Some(prepared) = &step.prepared {
                    installed
                        .actions
                        .get(step.call.name())
                        .ok_or(AiError::UnsupportedCapability)?
                        .validate_prepared(
                            installed.version,
                            record.run_id(),
                            &step.call,
                            prepared,
                        )?;
                }
                current_call(state, runtime, record.owner(), &step.call, Some(actor)).await?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| AiError::DeadlineExceeded)?
}

/// A refused successor has no new reservation or generation; retain all earlier committed knowledge.
pub(crate) async fn fail_successor(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    frozen: &RunRecord,
    error: AiError,
) -> AiResult<()> {
    fail_tool(state, runtime, frozen, error, None).await
}
async fn fail_tool(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    frozen: &RunRecord,
    error: AiError,
    read_call: Option<&str>,
) -> AiResult<()> {
    let prepared = frozen.active_attempt().ok_or(AiError::Conflict)?.prepared();
    let now = state.clock.now_unix_ms();
    for _ in 0..2 {
        let (snapshot, current) =
            committed_attempt(state, runtime, frozen.run_id(), prepared).await?;
        if current.state() == &RunState::Failed {
            return Ok(());
        }
        if current.checkpoint() != frozen.checkpoint()
            || current.state() != &RunState::ToolsPending
            || current
                .tool_turns
                .last()
                .is_none_or(|turn| match read_call {
                    None => !turn.completed(),
                    Some(call) => !turn.next().is_some_and(|step| {
                        step.call.id() == call
                            && step.kind == ToolKind::Read
                            && step.prepared.is_none()
                            && !step.unknown
                            && checkpoint::definite_read_failure(&error)
                    }),
                })
        {
            return Err(AiError::Conflict);
        }
        if read_call.is_some() {
            dispatch_attempt(state.clone(), runtime, current.owner(), prepared).await?;
        } else {
            crate::flow::dispatch_actor(state.clone(), runtime, current.owner()).await?;
        }
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    current.run_id(),
                    checkpoint::HOLD_TOOL_ACTION,
                    ToolCheckpoint::new(
                        ToolChange::Failed {
                            error: error.clone(),
                        },
                        now,
                    )?,
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-tool-fail-{}-{}",
                    prepared.identity(),
                    current.checkpoint()
                )),
            )
            .await
            .map_err(map_error);
        match result {
            Err(AiError::Conflict) => continue,
            other => return other.map(|_| ()),
        }
    }
    Err(AiError::Conflict)
}

/// Require the exact original authoritative cost settlement before read admission or tool I/O.
pub(super) async fn settled_original(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &RunRecord,
    prepared: &crate::PreparedAttempt,
) -> AiResult<()> {
    settle_record(state, runtime, record, None).await?;
    // A known original generation cost must be settled before any tool side effect.
    let account = runtime
        .read::<crate::flow::AiBudget>(
            &state.service,
            record
                .active_attempt()
                .ok_or(AiError::Conflict)?
                .key()
                .account_window(),
        )
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Denied)?
        .record()?;
    let key = record.active_attempt().ok_or(AiError::Conflict)?.key();
    let evidence_index = record
        .attempt_evidence()
        .iter()
        .position(|fact| fact.attempt_id() == prepared.identity())
        .ok_or(AiError::Conflict)?;
    let cost = record.attempt_usage()[evidence_index]
        .cost
        .ok_or(AiError::UnknownOutcome)?;
    if !account.entries().iter().any(|entry| {
        entry.key() == key
            && entry.prepared() == prepared
            && entry.status() == &crate::flow::ReservationStatus::Settled { actual_cost: cost }
    }) {
        return Err(AiError::Denied);
    }
    Ok(())
}
