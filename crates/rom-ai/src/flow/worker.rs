//! One bounded channel callback over public Resource commands.
use super::callback_phase::{Confirmation, Dispatch, Outcome, Settlement, Start, Step};
use super::{
    AiBudget, AiRun, CHECKPOINT_RUN, CheckpointRun, FlowTick, HOLD_RUN, HoldRun, PREPARE_RUN,
    PrepareRun, RESERVE_BUDGET, ReservationKey, ReservationStatus, ReserveBudget, RunState,
    START_RUN, StartRun,
    host::{FREE_ACCOUNT, HostState},
    projection::{dispatch_actor, map_error},
};
use crate::{AiError, AiResult, Completion, Deadline, PreparedAttempt, RouteCursor, choose};
use std::{sync::Arc, time::Duration};

pub(crate) async fn deliver(
    state: Arc<HostState>,
    delivery: rom::Delivery<FlowTick>,
) -> rom::DeliveryOutcome {
    let Ok(runtime) = state.runtime() else {
        return rom::DeliveryOutcome::Permanent;
    };
    let run = delivery.payload.run_id.clone();
    let Ok(start_owner) =
        super::start_identity::CallbackIdentity::new(&delivery.id, delivery.attempt)
    else {
        return rom::DeliveryOutcome::Permanent;
    };
    let Ok(callback) = crate::ExecutionDeadline::from_remaining(Duration::from_secs(18)) else {
        return rom::DeliveryOutcome::Unknown;
    };
    let callback_deadline = tokio::time::Instant::from_std(callback.end());
    let Ok(execution) = callback.shortened(Duration::from_secs(2)) else {
        return rom::DeliveryOutcome::Unknown;
    };
    let tick_future = Box::pin(tick(
        state.clone(),
        runtime.clone(),
        delivery.payload,
        execution,
        start_owner,
    ));
    match tokio::time::timeout_at(callback_deadline, tick_future).await {
        Ok(Ok(())) => rom::DeliveryOutcome::Accepted,
        Ok(Err(AiError::UnknownOutcome)) => {
            if tokio::time::timeout_at(
                callback_deadline,
                terminal_cleanup_proven(&state, &runtime, &run),
            )
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
            {
                rom::DeliveryOutcome::Retryable
            } else {
                rom::DeliveryOutcome::Unknown
            }
        }
        Err(_) => rom::DeliveryOutcome::Unknown,
        Ok(Err(_)) => {
            if tokio::time::timeout_at(
                callback_deadline,
                settlement_retry_proven(&state, &runtime, &run),
            )
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
            {
                rom::DeliveryOutcome::Retryable
            } else {
                rom::DeliveryOutcome::Permanent
            }
        }
    }
}
async fn terminal_cleanup_proven(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
) -> AiResult<bool> {
    let record = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Conflict)?
        .record()?;
    dispatch_actor(state.clone(), runtime, record.owner()).await?;
    Ok(record.unstarted.is_some())
}
async fn settlement_retry_proven(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
) -> AiResult<bool> {
    let record = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::InvalidRequest)?
        .record()?;
    dispatch_actor(state.clone(), runtime, record.owner()).await?;
    if record.unstarted.is_some() {
        return Ok(true);
    }
    if !(matches!(
        record.state(),
        RunState::Completed | RunState::Cancelled | RunState::Waiting { .. }
    ) || (super::route_continuation::enabled(&record)
        && record.state() == &RunState::Prepared))
    {
        return Ok(false);
    }
    let Some(active) = record.active_attempt() else {
        return Ok(false);
    };
    active.prepared().validate()?;
    if super::route_continuation::enabled(&record) && record.state() == &RunState::Prepared {
        let account = runtime
            .read::<AiBudget>(&state.service, active.key().account_window())
            .await
            .map_err(map_error)?
            .value
            .ok_or(AiError::Denied)?
            .record()?;
        return Ok(account.entries().iter().any(|e| e == active));
    }
    let Some(index) = record
        .attempt_evidence()
        .iter()
        .position(|evidence| evidence.attempt_id() == active.prepared().identity())
    else {
        return Ok(false);
    };
    if !matches!(record.state(), RunState::Waiting { .. })
        && record.attempt_usage()[index].cost.is_none()
    {
        return Ok(false);
    }
    let account = runtime
        .read::<AiBudget>(&state.service, active.key().account_window())
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Denied)?
        .record()?;
    Ok(account
        .entries()
        .iter()
        .any(|entry| entry.key() == active.key() && entry.prepared() == active.prepared()))
}
async fn tick(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    tick: FlowTick,
    execution: crate::ExecutionDeadline,
    start_owner: super::start_identity::CallbackIdentity,
) -> AiResult<()> {
    super::callback_driver::run(state, runtime, tick, execution, start_owner).await
}
pub(super) async fn tick_phase(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    tick: FlowTick,
    execution: crate::ExecutionDeadline,
    start_owner: &super::start_identity::CallbackIdentity,
) -> AiResult<Step> {
    tick.validate()?;
    let snapshot = runtime
        .read::<AiRun>(&state.service, &tick.run_id)
        .await
        .map_err(map_error)?;
    let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
    if !record.queue_admitted {
        return Err(AiError::Conflict);
    }
    dispatch_actor(state.clone(), &runtime, record.owner()).await?;
    if record.state() == &RunState::ToolsPending
        && record
            .tool_turns
            .last()
            .is_some_and(|turn| !turn.completed())
        && tick.sequence == u64::from(record.checkpoint()) + 1
    {
        return crate::tools::worker::tick(&state, &runtime, &record, &execution)
            .await
            .map(|()| Step::Done);
    }
    if super::route_continuation::enabled(&record) {
        if (record.state() == &RunState::Prepared
            && tick.sequence == u64::from(record.checkpoint()))
            || (super::route_continuation::pending(&record).is_some()
                && matches!(record.state(), RunState::Waiting { .. })
                && tick.sequence == u64::from(record.checkpoint()) + 1)
        {
            return Ok(Step::Continue(tick.run_id));
        }
        if matches!(
            record.state(),
            RunState::Executing | RunState::CancelRequested
        ) && tick.sequence == u64::from(record.checkpoint())
        {
            return hold_unknown(
                &state,
                &runtime,
                &tick.run_id,
                record.active_attempt().ok_or(AiError::Conflict)?.prepared(),
            )
            .await
            .map(|()| Step::Done);
        }
    }
    if matches!(
        record.state(),
        RunState::Completed | RunState::Failed | RunState::Cancelled
    ) || tick.sequence <= u64::from(record.checkpoint())
    {
        settle_record(&state, &runtime, &record, None).await?;
        return Ok(Step::Done);
    }
    if !(matches!(record.state(), RunState::Queued | RunState::Waiting { .. })
        || (record.state() == &RunState::ToolsPending
            && record
                .tool_turns
                .last()
                .is_some_and(|turn| turn.completed())))
        || tick.sequence != u64::from(record.checkpoint()) + 1
    {
        return Err(AiError::Conflict);
    }
    // A persisted waiting checkpoint proves confirmed nonacceptance of the previous attempt.
    if matches!(record.state(), RunState::Waiting { .. }) {
        settle_record(&state, &runtime, &record, Some(crate::UsdNanos(0))).await?;
    }
    let (reservation, now) = match plan_attempt(&state, &runtime, &record, &execution).await {
        Ok(planned) => planned,
        Err(error) => {
            if record.state() == &RunState::Queued && error != AiError::Conflict {
                fail_preparation(&state, &runtime, &record, error.clone()).await?;
            } else if record.state() == &RunState::ToolsPending && error != AiError::Conflict {
                crate::tools::worker::fail_successor(&state, &runtime, &record, error.clone())
                    .await?;
            }
            return Err(error);
        }
    };
    let window = reservation.entry().key().account_window();
    let step = reservation.entry().key().step();
    let account = runtime
        .read::<AiBudget>(&state.service, window)
        .await
        .map_err(map_error)?;
    runtime
        .execute(
            &state.service,
            rom::Command::action(window, RESERVE_BUDGET, reservation.clone())
                .at_revision(account.revision)
                .idempotency(&format!(
                    "ai-reserve-{}",
                    reservation.entry().prepared().identity()
                )),
        )
        .await
        .map_err(map_error)?;
    verify_reservation(&runtime, &state, &reservation).await?;
    let prepared = runtime
        .execute(
            &state.service,
            rom::Command::action(
                &tick.run_id,
                PREPARE_RUN,
                PrepareRun::new(reservation.entry().clone(), now)?,
            )
            .at_revision(snapshot.revision)
            .idempotency(&format!(
                "ai-prepare-{}",
                reservation.entry().prepared().identity()
            )),
        )
        .await
        .map_err(map_error)?;
    observe_committed(
        &state,
        &runtime,
        &tick.run_id,
        crate::ObservationKind::Prepared,
    )
    .await;
    if super::route_continuation::enabled(&record) {
        return Ok(Step::Start(Box::new(Start {
            record: prepared.value.ok_or(AiError::Conflict)?.record()?,
            revision: prepared.revision,
        })));
    }
    super::projection::dispatch_attempt(
        state.clone(),
        &runtime,
        record.owner(),
        reservation.entry().prepared(),
    )
    .await?;
    verify_reservation(&runtime, &state, &reservation).await?;
    runtime
        .execute(
            &state.service,
            rom::Command::action(
                &tick.run_id,
                START_RUN,
                StartRun::new(step, state.clock.now_unix_ms())?,
            )
            .at_revision(prepared.revision)
            .idempotency(&start_owner.start(
                &tick.run_id,
                reservation.entry().prepared().identity(),
                prepared.revision,
            )?),
        )
        .await
        .map_err(map_error)?;
    Ok(Step::Dispatch(Box::new(Dispatch {
        run: tick.run_id,
        attempt: reservation.entry().prepared().clone(),
        now,
    })))
}
pub(super) async fn dispatch_phase(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    run: &str,
    attempt: &PreparedAttempt,
    now: u64,
    execution: crate::ExecutionDeadline,
) -> AiResult<Step> {
    attempt.validate()?;
    let (_, record) = committed_attempt(&state, &runtime, run, attempt).await?;
    if super::projection::dispatch_attempt(state.clone(), &runtime, record.owner(), attempt)
        .await
        .is_err()
    {
        return hold_unknown(&state, &runtime, run, attempt)
            .await
            .map(|()| Step::Done);
    }
    let (_, fresh_record) = committed_attempt(&state, &runtime, run, attempt).await?;
    if fresh_record.state() != &RunState::Executing {
        return Err(AiError::UnknownOutcome);
    }
    if crate::tools::worker::authorize_transcript(&state, &runtime, &fresh_record, &execution)
        .await
        .is_err()
    {
        return hold_unknown(&state, &runtime, run, attempt)
            .await
            .map(|()| Step::Done);
    }
    let remaining = match Deadline::remaining(
        state.clock.now_unix_ms(),
        now,
        attempt.deadline().expires_at_unix_ms(),
    ) {
        Ok(deadline) => deadline,
        Err(_) => {
            return hold_unknown(&state, &runtime, run, attempt)
                .await
                .map(|()| Step::Done);
        }
    };
    let completion = match execution.remaining_ms() {
        Ok(milliseconds) => {
            tokio::time::timeout(
                Duration::from_millis(remaining.remaining_ms().min(milliseconds)),
                state.provider.complete_observed_within(attempt, execution),
            )
            .await
        }
        Err(_) => {
            return hold_unknown(&state, &runtime, run, attempt)
                .await
                .map(|()| Step::Done);
        }
    };
    let outcome = match completion {
        Ok(Ok(outcome)) if outcome.validate_for(attempt).is_ok() => outcome,
        _ => {
            return hold_unknown(&state, &runtime, run, attempt)
                .await
                .map(|()| Step::Done);
        }
    };
    Ok(Step::Outcome(Box::new(Outcome {
        run: run.to_owned(),
        attempt: attempt.clone(),
        outcome,
    })))
}
pub(super) enum PlanningFailure {
    Transient(AiError),
    Final(AiError),
}
impl PlanningFailure {
    fn error(self) -> AiError {
        match self {
            Self::Transient(e) | Self::Final(e) => e,
        }
    }
}
impl From<AiError> for PlanningFailure {
    fn from(error: AiError) -> Self {
        Self::Transient(error)
    }
}
pub(super) async fn plan_attempt(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &super::RunRecord,
    execution: &crate::ExecutionDeadline,
) -> AiResult<(ReserveBudget, u64)> {
    plan_continuation(state, runtime, record, execution)
        .await
        .map_err(PlanningFailure::error)
}
pub(super) async fn plan_continuation(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &super::RunRecord,
    execution: &crate::ExecutionDeadline,
) -> Result<(ReserveBudget, u64), PlanningFailure> {
    crate::tools::worker::authorize_transcript(state, runtime, record, execution).await?;
    if record.counters().generation_attempts() >= record.policy().limits().generation_attempts {
        return Err(PlanningFailure::Final(AiError::BudgetExhausted));
    }
    let now = state.clock.now_unix_ms();
    let request = record.effective_request(
        record
            .counters()
            .generation_attempts()
            .checked_add(1)
            .ok_or(AiError::InvalidRequest)?,
    )?;
    let deadline = Deadline::remaining(
        now,
        record.last_observed_unix_ms(),
        record.expires_at_unix_ms().min(now.saturating_add(18_000)),
    )?;
    let discovery = Deadline::remaining(
        now,
        now,
        deadline.expires_at_unix_ms().min(
            now.checked_add(execution.remaining_ms()?)
                .ok_or(AiError::DeadlineExceeded)?,
        ),
    )?;
    let catalog = tokio::time::timeout(
        Duration::from_millis(discovery.remaining_ms()),
        state
            .provider
            .catalog_for(&request, record.policy(), discovery),
    )
    .await
    .map_err(|_| AiError::DeadlineExceeded)??;
    let route = if let Some(continuation) = super::route_continuation::pending(record) {
        crate::choose_continuation(
            record.policy(),
            &catalog,
            &continuation.forward_cursor,
            &request,
        )
        .map_err(PlanningFailure::Final)?
    } else if matches!(
        record.state(),
        RunState::Waiting { .. } | RunState::ToolsPending
    ) {
        if let RunState::Waiting { retry_at_unix_ms } = record.state()
            && now < *retry_at_unix_ms
        {
            return Err(PlanningFailure::Transient(AiError::Conflict));
        }
        let previous = record.active_attempt().ok_or(AiError::Conflict)?.prepared();
        let derived =
            previous
                .route()
                .for_request(record.policy(), previous.request(), &request)?;
        // Fresh metadata proves current eligibility separately from the frozen run cursor.
        let current = catalog
            .models()
            .iter()
            .find(|model| model.id == derived.model())
            .ok_or(AiError::UnsupportedCapability)?
            .clone();
        let focused = crate::CatalogSnapshot::new(catalog.identity(), vec![current])?;
        let eligible = choose(
            record.policy(),
            &focused,
            &RouteCursor::new(record.policy().version(), catalog.identity())?,
            &request,
        )?;
        if eligible.model() != derived.model()
            || eligible.tier() != derived.tier()
            || eligible.maximum_cost() != derived.maximum_cost()
        {
            return Err(PlanningFailure::Final(AiError::UnsupportedCapability));
        }
        derived
    } else {
        choose(
            record.policy(),
            &catalog,
            &RouteCursor::new(record.policy().version(), catalog.identity())?,
            &request,
        )?
    };
    let now = state.clock.now_unix_ms();
    let deadline = Deadline::remaining(
        now,
        record.last_observed_unix_ms(),
        deadline.expires_at_unix_ms(),
    )?;
    let window = record.policy().budget_reference().unwrap_or(FREE_ACCOUNT);
    let step = record.checkpoint() + 1;
    let key = ReservationKey::new(
        window,
        record.run_id(),
        step,
        record.counters().generation_attempts() + 1,
        record.policy().version(),
    )?;
    let attempt = PreparedAttempt::new(
        key.attempt_identity(),
        request,
        record.policy().clone(),
        route,
        deadline,
    )?;
    let reservation = ReserveBudget::new(key, attempt)?;
    super::projection::dispatch_attempt(
        state.clone(),
        runtime,
        record.owner(),
        reservation.entry().prepared(),
    )
    .await?;
    tokio::time::timeout(
        Duration::from_millis(execution.remaining_ms()?),
        state.provider.preflight(reservation.entry().prepared()),
    )
    .await
    .map_err(|_| AiError::DeadlineExceeded)??;
    execution.remaining_ms()?;
    // Encoding cannot grant permission; refresh current authority after the bounded preflight.
    super::projection::dispatch_attempt(
        state.clone(),
        runtime,
        record.owner(),
        reservation.entry().prepared(),
    )
    .await?;
    Ok((reservation, now))
}
async fn fail_preparation(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    frozen: &super::RunRecord,
    error: AiError,
) -> AiResult<()> {
    for retry in 0..2 {
        let snapshot = runtime
            .read::<AiRun>(&state.service, frozen.run_id())
            .await
            .map_err(map_error)?;
        let current = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
        if current.owner() != frozen.owner()
            || current.request() != frozen.request()
            || current.policy() != frozen.policy()
        {
            return Err(AiError::Conflict);
        }
        dispatch_actor(state.clone(), runtime, current.owner()).await?;
        if current.state() == &RunState::Failed && current.failure() == Some(&error) {
            return Ok(());
        }
        if current.state() != &RunState::Queued
            || current.checkpoint() != 0
            || current.active_attempt().is_some()
        {
            return Err(AiError::Conflict);
        }
        // This action preserves any account-first orphan; absence of an active attempt is not a release proof.
        let input = super::FailPreparation::new(state.clock.now_unix_ms(), error.clone())?;
        match runtime
            .execute(
                &state.service,
                rom::Command::action(frozen.run_id(), super::FAIL_PREPARATION, input)
                    .at_revision(snapshot.revision)
                    .idempotency(&format!(
                        "ai-preparation-failed-{}-{}",
                        frozen.run_id(),
                        snapshot.revision
                    )),
            )
            .await
        {
            Ok(_) => return Ok(()),
            Err(rom::Error::Conflict) if retry == 0 => continue,
            Err(error) => return Err(map_error(error)),
        }
    }
    Err(AiError::Conflict)
}
pub(super) async fn rate_limit_phase(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
    attempt: &PreparedAttempt,
    evidence: &crate::AttemptEvidence,
    retry_floor: u64,
) -> AiResult<Step> {
    let (_, initial) = committed_attempt(state, runtime, run, attempt).await?;
    if super::route_continuation::enabled(&initial) {
        let now = state.clock.now_unix_ms();
        return Ok(Step::Confirm(Box::new(Confirmation {
            run: run.to_owned(),
            attempt: attempt.clone(),
            evidence: evidence.clone(),
            proof: super::NonacceptanceProof::RateLimited,
            now,
            due: retry_floor.max(now.saturating_add(1)),
        })));
    }
    for _ in 0..2 {
        let (snapshot, record) = committed_attempt(state, runtime, run, attempt).await?;
        if !matches!(
            record.state(),
            RunState::Executing | RunState::CancelRequested
        ) {
            return hold_unknown(state, runtime, run, attempt)
                .await
                .map(|()| Step::Done);
        }
        let now = state.clock.now_unix_ms();
        let due = retry_floor.max(now.checked_add(1).ok_or(AiError::DeadlineExceeded)?);
        evidence.validate()?;
        if evidence.attempt_id() != attempt.identity() || evidence.generation_id().is_some() {
            return hold_unknown(state, runtime, run, attempt)
                .await
                .map(|()| Step::Done);
        }
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    run,
                    CHECKPOINT_RUN,
                    CheckpointRun::waiting(record.checkpoint(), now, due, evidence.clone())?,
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-wait-{}-{}",
                    attempt.identity(),
                    snapshot.revision
                )),
            )
            .await;
        match result {
            Ok(_) => {
                observe_committed(state, runtime, run, crate::ObservationKind::Waiting).await;
                return Ok(Step::Settle(Settlement::Run {
                    run: run.to_owned(),
                    confirmed_nonaccepted: Some(crate::UsdNanos(0)),
                }));
            }
            Err(rom::Error::Conflict) => continue,
            Err(error) => return Err(map_error(error)),
        }
    }
    Err(AiError::UnknownOutcome)
}
pub(crate) async fn committed_attempt(
    state: &HostState,
    runtime: &rom::Runtime,
    run: &str,
    attempt: &PreparedAttempt,
) -> AiResult<(rom::Snapshot<AiRun>, super::RunRecord)> {
    attempt.validate()?;
    let snapshot = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?;
    let record = snapshot
        .value
        .as_ref()
        .ok_or(AiError::InvalidRequest)?
        .record()?;
    let active = record
        .active_attempt()
        .filter(|active| active.prepared() == attempt)
        .ok_or(AiError::Conflict)?;
    let account = runtime
        .read::<AiBudget>(&state.service, active.key().account_window())
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Denied)?
        .record()?;
    if !account
        .entries()
        .iter()
        .any(|entry| entry.key() == active.key() && entry.prepared() == attempt)
    {
        return Err(AiError::Denied);
    }
    Ok((snapshot, record))
}
pub(super) async fn hold_unknown(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
    attempt: &PreparedAttempt,
) -> AiResult<()> {
    hold_observed_unknown(state, runtime, run, attempt, None).await
}
pub(crate) async fn record_held_observation(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
    attempt: &PreparedAttempt,
    evidence: &crate::AttemptEvidence,
    usage: &crate::Usage,
) -> AiResult<()> {
    for _ in 0..2 {
        let (snapshot, record) = committed_attempt(state, runtime, run, attempt).await?;
        if record.state() != &RunState::AwaitingReconciliation {
            return Err(AiError::Conflict);
        }
        super::projection::dispatch_attempt(state.clone(), runtime, record.owner(), attempt)
            .await?;
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    run,
                    super::knowledge::RECORD_ATTEMPT_EVIDENCE,
                    super::knowledge::RecordAttemptEvidence::new(
                        record.checkpoint(),
                        state.clock.now_unix_ms(),
                        evidence.clone(),
                        usage.clone(),
                    )?,
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-enrich-{}-{}",
                    attempt.identity(),
                    snapshot.revision
                )),
            )
            .await;
        match result {
            Ok(_) => {
                settle_run(state, runtime, run, None).await?;
                return Ok(());
            }
            Err(rom::Error::Conflict) => continue,
            Err(error) => return Err(map_error(error)),
        }
    }
    Err(AiError::Conflict)
}
pub(super) async fn hold_observed_unknown(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    run: &str,
    attempt: &PreparedAttempt,
    observed: Option<(&crate::AttemptEvidence, &crate::Usage)>,
) -> AiResult<()> {
    // One CAS retry absorbs a concurrent immutable operation stamp; no provider I/O is repeated.
    for _ in 0..2 {
        let (snapshot, record) = committed_attempt(state, runtime, run, attempt).await?;
        if observed.is_some() {
            super::projection::dispatch_attempt(state.clone(), runtime, record.owner(), attempt)
                .await?;
        }
        match record.state() {
            RunState::AwaitingReconciliation => {
                if let Some((evidence, usage)) = observed {
                    record_held_observation(state, runtime, run, attempt, evidence, usage)
                        .await
                        .map_err(|_| AiError::UnknownOutcome)?;
                    return Err(AiError::UnknownOutcome);
                }
                settle_record(state, runtime, &record, None).await?;
                return Err(AiError::UnknownOutcome);
            }
            RunState::Completed
            | RunState::Cancelled
            | RunState::Failed
            | RunState::Waiting { .. } => {
                settle_record(state, runtime, &record, None).await?;
                return Ok(());
            }
            RunState::Executing | RunState::CancelRequested => {}
            _ => return Err(AiError::Conflict),
        }
        let result = runtime
            .execute(
                &state.service,
                rom::Command::action(
                    run,
                    HOLD_RUN,
                    match observed {
                        Some((evidence, usage)) => {
                            HoldRun::new(record.checkpoint(), state.clock.now_unix_ms())?
                                .with_evidence(evidence.clone())?
                                .with_usage(usage.clone())?
                        }
                        None => HoldRun::new(record.checkpoint(), state.clock.now_unix_ms())?,
                    },
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-hold-{}-{}",
                    attempt.identity(),
                    snapshot.revision
                )),
            )
            .await;
        match result {
            Ok(_) => {
                observe_committed(state, runtime, run, crate::ObservationKind::Unknown).await;
                settle_run(state, runtime, run, None).await?;
                return Err(AiError::UnknownOutcome);
            }
            Err(rom::Error::Conflict) => continue,
            Err(_) => return Err(AiError::UnknownOutcome),
        }
    }
    Err(AiError::UnknownOutcome)
}
pub(crate) async fn publish_completion(
    state: &Arc<HostState>,
    runtime: &Arc<rom::Runtime>,
    run: &str,
    attempt: &PreparedAttempt,
    completion: Completion,
    enforce_deadline: bool,
) -> AiResult<()> {
    completion.validate()?;
    let Completion::Output {
        value,
        usage,
        evidence,
    } = completion
    else {
        return Err(AiError::UnsupportedCapability);
    };
    if evidence.attempt_id() != attempt.identity() {
        return Err(AiError::InvalidOutput);
    }
    let snapshot = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?;
    let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
    if record
        .active_attempt()
        .is_none_or(|active| active.prepared() != attempt)
    {
        return Err(AiError::Conflict);
    }
    let mut proven = record.clone();
    proven.note_evidence(evidence.clone(), usage.clone())?;
    if enforce_deadline
        && !record.cancel_requested()
        && state.clock.now_unix_ms() >= attempt.deadline().expires_at_unix_ms()
    {
        runtime
            .execute(
                &state.service,
                rom::Command::action(
                    run,
                    HOLD_RUN,
                    HoldRun::new(record.checkpoint(), state.clock.now_unix_ms())?
                        .with_evidence(evidence)?
                        .with_usage(usage)?,
                )
                .at_revision(snapshot.revision)
                .idempotency(&format!("ai-expired-{}", attempt.identity())),
            )
            .await
            .map_err(map_error)?;
        observe_committed(state, runtime, run, crate::ObservationKind::Unknown).await;
        settle_run(state, runtime, run, None).await?;
        return Err(AiError::UnknownOutcome);
    }
    let output = match state.validator.validate(&value) {
        Ok(output) => output,
        Err(_) => {
            if matches!(
                record.state(),
                RunState::Executing | RunState::CancelRequested
            ) {
                runtime
                    .execute(
                        &state.service,
                        rom::Command::action(
                            run,
                            HOLD_RUN,
                            HoldRun::new(record.checkpoint(), state.clock.now_unix_ms())?
                                .with_evidence(evidence)?
                                .with_usage(usage)?,
                        )
                        .at_revision(snapshot.revision)
                        .idempotency(&format!("ai-invalid-output-{}", attempt.identity())),
                    )
                    .await
                    .map_err(map_error)?;
                observe_committed(state, runtime, run, crate::ObservationKind::Unknown).await;
                settle_run(state, runtime, run, None).await?;
            }
            return Err(AiError::UnknownOutcome);
        }
    };
    let latest = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?;
    let current = latest.value.ok_or(AiError::InvalidRequest)?.record()?;
    if current
        .active_attempt()
        .is_none_or(|active| active.prepared() != attempt)
    {
        return Err(AiError::Conflict);
    }
    if enforce_deadline
        && !current.cancel_requested()
        && state.clock.now_unix_ms() >= attempt.deadline().expires_at_unix_ms()
    {
        runtime
            .execute(
                &state.service,
                rom::Command::action(
                    run,
                    HOLD_RUN,
                    HoldRun::new(current.checkpoint(), state.clock.now_unix_ms())?
                        .with_evidence(evidence)?
                        .with_usage(usage)?,
                )
                .at_revision(latest.revision)
                .idempotency(&format!("ai-expired-{}", attempt.identity())),
            )
            .await
            .map_err(map_error)?;
        observe_committed(state, runtime, run, crate::ObservationKind::Unknown).await;
        settle_run(state, runtime, run, None).await?;
        return Err(AiError::UnknownOutcome);
    }
    runtime
        .execute(
            &state.service,
            rom::Command::action(
                run,
                CHECKPOINT_RUN,
                CheckpointRun::completed(
                    current.checkpoint(),
                    state.clock.now_unix_ms(),
                    evidence,
                    usage,
                    output,
                )?,
            )
            .at_revision(latest.revision)
            .idempotency(&format!("ai-complete-{}", attempt.identity())),
        )
        .await
        .map_err(map_error)?;
    observe_committed(state, runtime, run, crate::ObservationKind::Completed).await;
    settle_run(state, runtime, run, None).await?;
    Ok(())
}
pub(super) async fn observe_committed(
    state: &HostState,
    runtime: &rom::Runtime,
    run: &str,
    requested: crate::ObservationKind,
) {
    if state.observer.is_none() {
        return;
    }
    let Ok(snapshot) = runtime.read::<AiRun>(&state.service, run).await else {
        return;
    };
    let Some(value) = snapshot.value else {
        return;
    };
    let Ok(record) = value.record() else {
        return;
    };
    let category = match requested {
        crate::ObservationKind::Completed if record.state() != &RunState::Completed => {
            crate::ObservationKind::Unknown
        }
        crate::ObservationKind::Waiting if !matches!(record.state(), RunState::Waiting { .. }) => {
            crate::ObservationKind::Rejected
        }
        other => other,
    };
    let created = record
        .expires_at_unix_ms()
        .saturating_sub(record.policy().limits().age_seconds.saturating_mul(1000));
    if let Ok(event) = crate::FlowObservation::new(
        run,
        record.checkpoint(),
        record.counters().generation_attempts(),
        state.clock.now_unix_ms().saturating_sub(created),
        category,
    ) {
        crate::observe_safely(state.observer.as_deref(), &event);
    }
}
pub(crate) async fn settle_run(
    state: &HostState,
    runtime: &rom::Runtime,
    run: &str,
    confirmed_nonaccepted: Option<crate::UsdNanos>,
) -> AiResult<()> {
    let record = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::InvalidRequest)?
        .record()?;
    settle_record(state, runtime, &record, confirmed_nonaccepted).await
}
pub(crate) async fn settle_record(
    state: &HostState,
    runtime: &rom::Runtime,
    record: &super::RunRecord,
    confirmed_nonaccepted: Option<crate::UsdNanos>,
) -> AiResult<()> {
    super::unstarted::settle(state, runtime, record).await?;
    let Some(active) = record.active_attempt() else {
        return Ok(());
    };
    active.prepared().validate()?;
    let Some(index) = record
        .attempt_evidence()
        .iter()
        .position(|evidence| evidence.attempt_id() == active.prepared().identity())
    else {
        return Ok(());
    };
    let continuation = record
        .route_continuations
        .iter()
        .find(|c| c.predecessor == *active.key());
    let fallback = continuation.map_or(confirmed_nonaccepted, |c| c.confirmed_cost);
    let cost = record.attempt_usage()[index].cost.or(fallback);
    let snapshot = runtime
        .read::<AiBudget>(&state.service, active.key().account_window())
        .await
        .map_err(map_error)?;
    let account = snapshot.value.ok_or(AiError::Denied)?.record()?;
    let entry = account
        .entries()
        .iter()
        .find(|entry| entry.key() == active.key() && entry.prepared() == active.prepared())
        .ok_or(AiError::Denied)?;
    if cost.is_some_and(|cost| cost > entry.maximum_cost()) {
        return Err(AiError::InvalidOutput);
    }
    if let ReservationStatus::Settled { actual_cost } = entry.status() {
        return if cost.is_none_or(|cost| cost == *actual_cost) {
            Ok(())
        } else {
            Err(AiError::InvalidOutput)
        };
    }
    if cost.is_none() && entry.status() == &ReservationStatus::Unknown {
        return Ok(());
    }
    let input = match cost {
        Some(cost) => super::SettleBudget::confirmed(active.key().clone(), cost)?,
        None => super::SettleBudget::unknown(active.key().clone())?,
    };
    let knowledge = cost.map_or_else(|| "unknown".to_owned(), |cost| format!("known-{}", cost.0));
    runtime
        .execute(
            &state.service,
            rom::Command::action(active.key().account_window(), super::SETTLE_BUDGET, input)
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-settle-{}-{knowledge}",
                    active.prepared().identity()
                )),
        )
        .await
        .map_err(map_error)?;
    Ok(())
}
pub(super) async fn verify_reservation(
    runtime: &rom::Runtime,
    state: &HostState,
    reservation: &ReserveBudget,
) -> AiResult<()> {
    reservation.entry().prepared().validate()?;
    let key = reservation.entry().key();
    let account = runtime
        .read::<AiBudget>(&state.service, key.account_window())
        .await
        .map_err(map_error)?
        .value
        .ok_or(AiError::Denied)?
        .record()?;
    if !account
        .entries()
        .iter()
        .any(|entry| entry == reservation.entry() && entry.status() == &ReservationStatus::Reserved)
    {
        return Err(AiError::Denied);
    }
    Ok(())
}
