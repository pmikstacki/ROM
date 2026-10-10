//! Account-first continuation execution and exact-command recovery. No provider retry loop.
use super::callback_phase::{Dispatch, Settlement, Start, Step};
use super::{
    AiBudget, AiRun, CONFIRM_NONACCEPTANCE, ConfirmNonacceptance, HostState, NonacceptanceProof,
    PREPARE_RUN, PrepareRun, RESERVE_BUDGET, ReservationStatus, ReserveBudget, RunRecord, RunState,
    STAGE_SUCCESSOR, START_RUN, StageSuccessor, StartRun,
    projection::{dispatch_actor, dispatch_attempt, map_error},
    route_continuation::{self, SuccessorPlan},
};
use crate::{AiError, AiResult, AttemptEvidence, Deadline, ExecutionDeadline, PreparedAttempt};
use std::{sync::Arc, time::Duration};

/// Retry only the identical local command after Unknown. This never repeats provider I/O.
async fn exact<R: rom::Resource>(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    owner: &super::OwnerIdentity,
    command: rom::Command<R>,
) -> AiResult<rom::Snapshot<R>> {
    dispatch_actor(state.clone(), runtime, owner).await?;
    execute_exact(state, runtime, command, None).await
}
async fn execute_exact<R: rom::Resource>(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    command: rom::Command<R>,
    expires: Option<u64>,
) -> AiResult<rom::Snapshot<R>> {
    for retry in 0..2 {
        if expires.is_some_and(|expiry| state.clock.now_unix_ms() >= expiry) {
            return Err(AiError::UnknownOutcome);
        }
        match runtime.execute(&state.service, command.clone()).await {
            Ok(result) => return Ok(result),
            Err(rom::Error::Unknown) if retry == 0 => continue,
            Err(error) => return Err(map_error(error)),
        }
    }
    Err(AiError::UnknownOutcome)
}
/// Borrowed invocation data; the caller retains its owned phase or reconciliation record.
/// Proof and observed times move together without cloning the prepared request or evidence.
pub(super) struct ConfirmationRequest<'a> {
    pub(super) run: &'a str,
    pub(super) attempt: &'a PreparedAttempt,
    pub(super) evidence: &'a AttemptEvidence,
    pub(super) proof: NonacceptanceProof,
    pub(super) now: u64,
    pub(super) due: u64,
}

pub(super) async fn confirm(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    request: ConfirmationRequest<'_>,
) -> AiResult<()> {
    let settlement = confirm_phase(state, runtime, request).await?;
    super::account_settlement::apply(state, runtime, settlement).await
}

pub(super) async fn confirm_phase(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    request: ConfirmationRequest<'_>,
) -> AiResult<Settlement> {
    let ConfirmationRequest {
        run,
        attempt,
        evidence,
        proof,
        now,
        due,
    } = request;
    let (snapshot, record) = super::worker::committed_attempt(state, runtime, run, attempt).await?;
    if record
        .route_continuations
        .iter()
        .any(|c| c.predecessor.attempt_identity() == attempt.identity())
    {
        return Ok(Settlement::Record {
            record: Box::new(record),
            confirmed_nonaccepted: None,
        });
    }
    dispatch_attempt(state.clone(), runtime, record.owner(), attempt).await?;
    let input = ConfirmNonacceptance::new(record.checkpoint(), now, due, evidence.clone(), proof)?;
    // Input, observed time, expected revision and identity freeze once, before either execute.
    let command = rom::Command::action(run, CONFIRM_NONACCEPTANCE, input)
        .at_revision(snapshot.revision)
        .idempotency(&format!("ai-nonaccepted-v1-{}", attempt.identity()));
    exact(state, runtime, record.owner(), command).await?;
    super::worker::observe_committed(state, runtime, run, crate::ObservationKind::Waiting).await;
    Ok(Settlement::Run {
        run: run.to_owned(),
        confirmed_nonaccepted: None,
    })
}

pub(super) async fn prepare_phase(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    run: &str,
    execution: ExecutionDeadline,
) -> AiResult<Step> {
    let snapshot = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?;
    let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
    dispatch_actor(state.clone(), &runtime, record.owner()).await?;
    if let Err(error) =
        crate::tools::worker::authorize_transcript(&state, &runtime, &record, &execution).await
    {
        if error == AiError::Denied {
            fail(&state, &runtime, &record, snapshot.revision, error.clone()).await?;
        }
        return Err(error);
    }
    if record.state() == &RunState::Prepared {
        // Recovery of PREPARE acknowledgement uses the committed entry; it does not plan again.
        return Ok(Step::Start(Box::new(Start {
            record,
            revision: snapshot.revision,
        })));
    }
    if !matches!(record.state(), RunState::Waiting { retry_at_unix_ms } if state.clock.now_unix_ms() >= *retry_at_unix_ms)
    {
        return Err(AiError::Conflict);
    }
    super::worker::settle_record(&state, &runtime, &record, None).await?;
    if state.clock.now_unix_ms() >= record.expires_at_unix_ms() {
        fail(
            &state,
            &runtime,
            &record,
            snapshot.revision,
            AiError::DeadlineExceeded,
        )
        .await?;
        return Err(AiError::DeadlineExceeded);
    }
    let continuation = route_continuation::pending(&record).ok_or(AiError::Conflict)?;
    let plan = if let Some(plan) = &continuation.staged {
        plan.clone()
    } else {
        let (reservation, now) =
            match super::worker::plan_continuation(&state, &runtime, &record, &execution).await {
                Ok(value) => value,
                Err(super::worker::PlanningFailure::Final(error)) => {
                    fail(&state, &runtime, &record, snapshot.revision, error.clone()).await?;
                    return Err(error);
                }
                Err(super::worker::PlanningFailure::Transient(error)) => {
                    if matches!(error, AiError::Denied | AiError::DeadlineExceeded) {
                        fail(&state, &runtime, &record, snapshot.revision, error.clone()).await?;
                    }
                    return Err(error);
                }
            };
        let account = runtime
            .read::<AiBudget>(&state.service, reservation.entry().key().account_window())
            .await
            .map_err(map_error)?;
        let input = StageSuccessor::new(reservation.entry().clone(), account.revision, now)?;
        if let Err(error) = exact(
            &state,
            &runtime,
            record.owner(),
            rom::Command::action(run, STAGE_SUCCESSOR, input)
                .at_revision(snapshot.revision)
                .idempotency(&format!(
                    "ai-stage-v1-{}",
                    reservation.entry().prepared().identity()
                )),
        )
        .await
        {
            if error == AiError::BudgetExhausted {
                fail(&state, &runtime, &record, snapshot.revision, error.clone()).await?;
            }
            return Err(error);
        }
        SuccessorPlan {
            entry: reservation.entry().clone(),
            account_revision: account.revision,
            planned_at_ms: now,
        }
    };
    // Both fresh and reopened staged plans need live eligibility, grants and transcript access.
    if let Err(error) = current_eligibility(&state, &runtime, &record, &plan, &execution).await {
        if matches!(
            error,
            AiError::BudgetExhausted
                | AiError::UnsupportedCapability
                | AiError::DeadlineExceeded
                | AiError::InvalidRequest
                | AiError::Denied
        ) {
            let latest = runtime
                .read::<AiRun>(&state.service, run)
                .await
                .map_err(map_error)?;
            let current = latest.value.ok_or(AiError::Conflict)?.record()?;
            fail(&state, &runtime, &current, latest.revision, error.clone()).await?;
        }
        return Err(error);
    }
    let window = plan.entry.key().account_window();
    let account_snapshot = runtime
        .read::<AiBudget>(&state.service, window)
        .await
        .map_err(map_error)?;
    let account = account_snapshot.value.ok_or(AiError::Denied)?.record()?;
    match account
        .entries()
        .iter()
        .find(|entry| entry.key() == plan.entry.key())
    {
        Some(entry) if entry == &plan.entry && entry.status() == &ReservationStatus::Reserved => {}
        Some(_) => return Err(AiError::Conflict),
        None => {
            if u128::from(account.reserved().0)
                + u128::from(account.settled().0)
                + u128::from(plan.entry.maximum_cost().0)
                > u128::from(account.limit().0)
            {
                let latest = runtime
                    .read::<AiRun>(&state.service, run)
                    .await
                    .map_err(map_error)?;
                let current = latest.value.ok_or(AiError::Conflict)?.record()?;
                fail(
                    &state,
                    &runtime,
                    &current,
                    latest.revision,
                    AiError::BudgetExhausted,
                )
                .await?;
                return Err(AiError::BudgetExhausted);
            }
            // A fresh absent-entry snapshot permits a new CAS identity. If its revision
            // advanced, an older in-flight CAS cannot subsequently commit. Unknown
            // acknowledgements replay the same command; existing exact entries win.
            let input =
                ReserveBudget::new(plan.entry.key().clone(), plan.entry.prepared().clone())?;
            exact(
                &state,
                &runtime,
                record.owner(),
                rom::Command::action(window, RESERVE_BUDGET, input)
                    .at_revision(account_snapshot.revision)
                    .idempotency(&format!(
                        "ai-reserve-cont-v1-{}-{}",
                        plan.entry.prepared().identity(),
                        account_snapshot.revision
                    )),
            )
            .await?;
        }
    }
    let reservation = ReserveBudget::new(plan.entry.key().clone(), plan.entry.prepared().clone())?;
    super::worker::verify_reservation(&runtime, &state, &reservation).await?;
    let latest = runtime
        .read::<AiRun>(&state.service, run)
        .await
        .map_err(map_error)?;
    let current = latest.value.ok_or(AiError::InvalidRequest)?.record()?;
    if !matches!(current.state(), RunState::Waiting { .. })
        || route_continuation::pending(&current).and_then(|c| c.staged.as_ref()) != Some(&plan)
    {
        return Err(AiError::Conflict);
    }
    if let Err(error) = dispatch_attempt(
        state.clone(),
        &runtime,
        current.owner(),
        plan.entry.prepared(),
    )
    .await
    {
        if error == AiError::Denied {
            fail(&state, &runtime, &current, latest.revision, error.clone()).await?;
        }
        return Err(error);
    }
    let prepared = exact(
        &state,
        &runtime,
        current.owner(),
        rom::Command::action(
            run,
            PREPARE_RUN,
            PrepareRun::new(plan.entry.clone(), plan.planned_at_ms)?,
        )
        .at_revision(latest.revision)
        .idempotency(&format!(
            "ai-prepare-cont-v1-{}-{}",
            plan.entry.prepared().identity(),
            latest.revision
        )),
    )
    .await?;
    super::worker::observe_committed(&state, &runtime, run, crate::ObservationKind::Prepared).await;
    Ok(Step::Start(Box::new(Start {
        record: prepared.value.ok_or(AiError::Conflict)?.record()?,
        revision: prepared.revision,
    })))
}
async fn current_eligibility(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &RunRecord,
    plan: &SuccessorPlan,
    execution: &ExecutionDeadline,
) -> AiResult<()> {
    let attempt = plan.entry.prepared();
    let now = state.clock.now_unix_ms();
    let deadline = Deadline::remaining(
        now,
        record.last_observed_unix_ms(),
        attempt.deadline().expires_at_unix_ms(),
    )?;
    let catalog = tokio::time::timeout(
        Duration::from_millis(deadline.remaining_ms().min(execution.remaining_ms()?)),
        state
            .provider
            .catalog_for(attempt.request(), record.policy(), deadline),
    )
    .await
    .map_err(|_| AiError::DeadlineExceeded)??;
    let model = catalog
        .models()
        .iter()
        .find(|m| m.id == attempt.route().model())
        .ok_or(AiError::UnsupportedCapability)?
        .clone();
    let focused = crate::CatalogSnapshot::new(catalog.identity(), vec![model])?;
    let cursor = route_continuation::pending(record)
        .map(|c| &c.forward_cursor)
        .unwrap_or_else(|| attempt.route().next_cursor());
    // Prepared recovery keeps the frozen route; its cursor already points past that model.
    let eligible = if record.state() == &RunState::Prepared {
        crate::choose(
            record.policy(),
            &focused,
            &crate::RouteCursor::new(record.policy().version(), catalog.identity())?,
            attempt.request(),
        )?
    } else {
        crate::choose_continuation(record.policy(), &focused, cursor, attempt.request())?
    };
    if eligible.model() != attempt.route().model()
        || eligible.tier() != attempt.route().tier()
        || eligible.maximum_cost() != attempt.route().maximum_cost()
    {
        return Err(AiError::UnsupportedCapability);
    }
    dispatch_attempt(state.clone(), runtime, record.owner(), attempt).await?;
    crate::tools::worker::authorize_transcript(state, runtime, record, execution).await?;
    tokio::time::timeout(
        Duration::from_millis(execution.remaining_ms()?),
        state.provider.preflight(attempt),
    )
    .await
    .map_err(|_| AiError::DeadlineExceeded)??;
    dispatch_attempt(state.clone(), runtime, record.owner(), attempt).await?;
    Ok(())
}
pub(super) async fn start_phase(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    record: RunRecord,
    revision: u64,
    execution: ExecutionDeadline,
    start_owner: &super::start_identity::CallbackIdentity,
) -> AiResult<Step> {
    let entry = record.active_attempt().ok_or(AiError::Conflict)?.clone();
    let plan = SuccessorPlan {
        planned_at_ms: entry.prepared().deadline().expires_at_unix_ms()
            - entry.prepared().deadline().remaining_ms(),
        entry: entry.clone(),
        account_revision: 0,
    };
    if let Err(error) = current_eligibility(&state, &runtime, &record, &plan, &execution).await {
        if matches!(
            error,
            AiError::DeadlineExceeded
                | AiError::UnsupportedCapability
                | AiError::InvalidRequest
                | AiError::Denied
        ) {
            fail(&state, &runtime, &record, revision, error.clone()).await?;
        }
        return Err(error);
    }
    super::worker::verify_reservation(
        &runtime,
        &state,
        &ReserveBudget::new(entry.key().clone(), entry.prepared().clone())?,
    )
    .await?;
    // Refresh after every asynchronous preflight/account/grant read. START also checks
    // the frozen attempt expiry. A delay after committed START is held, never reposted.
    dispatch_actor(state.clone(), &runtime, record.owner()).await?;
    let now = state.clock.now_unix_ms();
    if Deadline::remaining(
        now,
        record.last_observed_unix_ms(),
        entry.prepared().deadline().expires_at_unix_ms(),
    )
    .is_err()
        || execution.remaining_ms().is_err()
    {
        fail(
            &state,
            &runtime,
            &record,
            revision,
            AiError::DeadlineExceeded,
        )
        .await?;
        return Err(AiError::DeadlineExceeded);
    }
    // Only this callback may replay this command and dispatch once. Competing callback
    // identities reach CAS and lose; they cannot recover this successful START receipt.
    if let Err(error) = execute_exact(
        &state,
        &runtime,
        rom::Command::action(
            record.run_id(),
            START_RUN,
            StartRun::new(record.checkpoint(), now)?,
        )
        .at_revision(revision)
        .idempotency(&start_owner.start(
            record.run_id(),
            entry.prepared().identity(),
            revision,
        )?),
        Some(entry.prepared().deadline().expires_at_unix_ms()),
    )
    .await
    {
        if matches!(error, AiError::Storage | AiError::UnknownOutcome) {
            return super::worker::hold_unknown(
                &state,
                &runtime,
                record.run_id(),
                entry.prepared(),
            )
            .await
            .map(|()| Step::Done);
        }
        return Err(error);
    }
    Ok(Step::Dispatch(Box::new(Dispatch {
        run: record.run_id().to_owned(),
        attempt: entry.prepared().clone(),
        now: plan.planned_at_ms,
    })))
}
async fn fail(
    state: &Arc<HostState>,
    runtime: &rom::Runtime,
    record: &RunRecord,
    revision: u64,
    error: AiError,
) -> AiResult<()> {
    // Current owner resolution is required; partial attempt/tool denial is terminal,
    // while total owner revocation still denies disclosure and mutation.
    dispatch_actor(state.clone(), runtime, record.owner()).await?;
    let command = if record.state() == &RunState::Prepared {
        rom::Command::action(
            record.run_id(),
            super::CHECKPOINT_RUN,
            super::CheckpointRun::confirmed_failure(
                record.checkpoint(),
                state.clock.now_unix_ms(),
                error,
                None,
            )?,
        )
    } else {
        rom::Command::action(
            record.run_id(),
            super::FAIL_CONTINUATION,
            super::FailContinuation::new(state.clock.now_unix_ms(), error)?,
        )
    }
    .at_revision(revision)
    .idempotency(&format!(
        "ai-unstarted-failure-v2-{}-{revision}",
        record.run_id()
    ));
    let failed = exact(state, runtime, record.owner(), command)
        .await?
        .value
        .ok_or(AiError::Conflict)?
        .record()?;
    super::worker::settle_record(state, runtime, &failed, None).await
}
