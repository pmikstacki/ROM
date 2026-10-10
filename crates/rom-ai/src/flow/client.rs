//! Authorized public submission and views; Runtime alone owns worker scheduling.
use super::operation::{
    self, BEGIN_OPERATION, BeginOperation, FINISH_OPERATION, FinishOperation, OperationKind,
    OperationOutcome, OperationPhase, OperationStamp,
};
use super::{
    AiBudget, AiRun, CANCEL_RUN, CancelRun, RunHandle, RunView,
    host::{FREE_ACCOUNT, HostState, Submission},
    projection::{self, map_error},
};
use crate::{AiError, AiResult, UsdNanos};
use std::{fmt, sync::Arc};
pub struct FlowClient {
    pub(crate) runtime: Arc<rom::Runtime>,
    pub(crate) state: Arc<HostState>,
}
impl FlowClient {
    pub async fn submit(&self, actor: &rom::Actor, submission: Submission) -> AiResult<RunHandle> {
        submission.validate()?;
        let actor = projection::current(&self.runtime, actor).await?;
        let owner = self.state.authority.submit(&actor, &submission)?;
        if !owner.matches(&actor) {
            return Err(AiError::Denied);
        }
        let window = submission.policy.budget_reference().unwrap_or(FREE_ACCOUNT);
        let account = match self
            .runtime
            .read::<AiBudget>(&self.state.service, window)
            .await
        {
            Ok(snapshot) => Some(snapshot),
            Err(rom::Error::Missing) => None,
            Err(error) => return Err(map_error(error)),
        };
        if account.is_none() {
            if window != FREE_ACCOUNT || !submission.policy.paid().is_empty() {
                return Err(AiError::Denied);
            }
            self.runtime
                .execute(
                    &self.state.service,
                    rom::Command::create(
                        FREE_ACCOUNT,
                        AiBudget::new(&self.state.service, FREE_ACCOUNT, UsdNanos(0))?,
                    )
                    .idempotency("ai-free-account-v1"),
                )
                .await
                .map_err(map_error)?;
        }
        let existing = match self
            .runtime
            .read::<AiRun>(&self.state.service, &submission.id)
            .await
        {
            Ok(snapshot) => snapshot.value,
            Err(rom::Error::Missing) => None,
            Err(error) => return Err(map_error(error)),
        };
        let (created, tool_registry_version) = if let Some(value) = existing {
            let record = value.record()?;
            self.state.authority.inspect(&actor, record.owner())?;
            if record.owner() != &owner {
                return Err(AiError::Denied);
            }
            (
                record
                    .expires_at_unix_ms()
                    .checked_sub(record.policy().limits().age_seconds * 1000)
                    .ok_or(AiError::InvalidRequest)?,
                record.tool_registry_version,
            )
        } else {
            if let Some(registry) = &self.state.tools {
                registry.validate_requested(&submission.request)?;
            }
            (
                self.state.clock.now_unix_ms(),
                self.state.tools.as_ref().map(|registry| registry.version),
            )
        };
        let mut value = AiRun::queued(
            &submission.id,
            &self.state.service,
            owner,
            submission.request,
            submission.policy,
            created,
        )?;
        let mut record = value.record()?;
        record.tool_registry_version = tool_registry_version;
        value.store(record)?;
        self.runtime
            .execute(
                &self.state.service,
                rom::Command::create(&submission.id, value).idempotency(&submission.idempotency),
            )
            .await
            .map_err(map_error)?;
        RunHandle::new(submission.id)
    }
    pub async fn view(&self, actor: &rom::Actor, run: &RunHandle) -> AiResult<RunView> {
        run.validate()?;
        projection::current(&self.runtime, actor).await?;
        let snapshot = self
            .runtime
            .read::<AiRun>(&self.state.service, &run.0)
            .await
            .map_err(map_error)?;
        let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
        projection::view(
            &self.state,
            &self.runtime,
            actor,
            &record,
            snapshot.revision,
        )
        .await
    }
    pub async fn cancel(
        &self,
        actor: &rom::Actor,
        run: &RunHandle,
        expected_revision: u64,
        idempotency: &str,
    ) -> AiResult<RunView> {
        let (stamp, _) = self
            .admit(
                actor,
                run,
                expected_revision,
                idempotency,
                OperationKind::Cancel,
            )
            .await?;
        if matches!(stamp.phase, OperationPhase::Finished { .. }) {
            super::worker::settle_run(&self.state, &self.runtime, &run.0, None).await?;
            return self.view(actor, run).await;
        }
        self.runtime
            .execute(
                &self.state.service,
                rom::Command::action(
                    &run.0,
                    CANCEL_RUN,
                    CancelRun::new(stamp.checkpoint, stamp.admitted_at_ms)?,
                )
                .at_revision(stamp.original_expected_revision + 1)
                .idempotency(&operation::command_key(&run.0, &stamp, "cancel")),
            )
            .await
            .map_err(map_error)?;
        super::worker::settle_run(&self.state, &self.runtime, &run.0, None).await?;
        self.finish(run, &stamp, OperationOutcome::Cancelled)
            .await?;
        self.view(actor, run).await
    }
    /// Reconcile under current authority using the original run and operation identity.
    /// Trusted side-effect-free reads may admit a bounded retry or same-ordinal wake.
    /// Started provider calls and mutating Actions require original outcome reconciliation.
    pub async fn resume(
        &self,
        actor: &rom::Actor,
        run: &RunHandle,
        expected_revision: u64,
        idempotency: &str,
    ) -> AiResult<RunView> {
        tokio::time::timeout(
            std::time::Duration::from_secs(18),
            Box::pin(self.resume_inner(actor, run, expected_revision, idempotency)),
        )
        .await
        .map_err(|_| AiError::UnknownOutcome)?
    }
    async fn resume_inner(
        &self,
        actor: &rom::Actor,
        run: &RunHandle,
        expected_revision: u64,
        idempotency: &str,
    ) -> AiResult<RunView> {
        let execution =
            crate::ExecutionDeadline::from_remaining(std::time::Duration::from_secs(2))?;
        let lookup_deadline = tokio::time::Instant::from_std(execution.end());
        let _admission = self
            .state
            .lookup_admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| AiError::BudgetExhausted)?;
        let current_actor = projection::current(&self.runtime, actor).await?;
        let read_record = self
            .runtime
            .read::<AiRun>(&self.state.service, &run.0)
            .await
            .map_err(map_error)?
            .value
            .ok_or(AiError::InvalidRequest)?
            .record()?;
        self.state
            .authority
            .inspect(&current_actor, read_record.owner())?;
        if crate::tools::read_resume::handles(&read_record) {
            tokio::time::timeout_at(
                lookup_deadline,
                Box::pin(crate::tools::read_resume::resume(
                    &self.state,
                    &self.runtime,
                    &current_actor,
                    run,
                    expected_revision,
                    idempotency,
                    &execution,
                )),
            )
            .await
            .map_err(|_| AiError::UnknownOutcome)??;
            return self.view(&current_actor, run).await;
        }
        let (stamp, result) = tokio::time::timeout_at(lookup_deadline, async {
            let (stamp, fresh) = self
                .admit(
                    actor,
                    run,
                    expected_revision,
                    idempotency,
                    OperationKind::Reconcile,
                )
                .await?;
            // A lost read response remains unknown. A fresh explicit operation key permits another bounded lookup.
            if !fresh {
                return Ok::<_, AiError>((stamp, None));
            }
            let _provider = tokio::time::timeout_at(
                lookup_deadline,
                self.state.provider_permits.clone().acquire_owned(),
            )
            .await
            .map_err(|_| AiError::UnknownOutcome)?
            .map_err(|_| AiError::Closed)?;
            let snapshot = self
                .runtime
                .read::<AiRun>(&self.state.service, &run.0)
                .await
                .map_err(map_error)?;
            let mut record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
            if record.checkpoint() != stamp.checkpoint {
                return Err(AiError::Conflict);
            }
            if record
                .tool_turns
                .last()
                .is_some_and(|turn| !turn.completed())
            {
                crate::tools::worker::resume(
                    &self.state,
                    &self.runtime,
                    &record,
                    &operation::command_key(&run.0, &stamp, "resume-tool"),
                )
                .await?;
                self.finish(run, &stamp, OperationOutcome::Unresolved)
                    .await?;
                return Ok((stamp, None));
            }
            if matches!(
                record.state(),
                super::RunState::Executing | super::RunState::CancelRequested
            ) {
                projection::current(&self.runtime, actor).await?;
                self.state.authority.inspect(actor, record.owner())?;
                let active = record.active_attempt().ok_or(AiError::Conflict)?;
                let account = self
                    .runtime
                    .read::<AiBudget>(&self.state.service, active.key().account_window())
                    .await
                    .map_err(map_error)?
                    .value
                    .ok_or(AiError::Denied)?
                    .record()?;
                if !account.entries().iter().any(|entry| {
                    entry.key() == active.key() && entry.prepared() == active.prepared()
                }) {
                    return Err(AiError::Denied);
                }
                projection::dispatch_attempt(
                    self.state.clone(),
                    &self.runtime,
                    record.owner(),
                    active.prepared(),
                )
                .await?;
                self.runtime
                    .execute(
                        &self.state.service,
                        rom::Command::action(
                            &run.0,
                            super::HOLD_RUN,
                            super::HoldRun::new(stamp.checkpoint, self.state.clock.now_unix_ms())?,
                        )
                        .at_revision(snapshot.revision)
                        .idempotency(&operation::command_key(&run.0, &stamp, "hold-started")),
                    )
                    .await
                    .map_err(map_error)?;
                super::worker::settle_run(&self.state, &self.runtime, &run.0, None).await?;
                record = self
                    .runtime
                    .read::<AiRun>(&self.state.service, &run.0)
                    .await
                    .map_err(map_error)?
                    .value
                    .ok_or(AiError::InvalidRequest)?
                    .record()?;
            }
            if record.state() != &super::RunState::AwaitingReconciliation {
                return Err(AiError::Conflict);
            }
            projection::current(&self.runtime, actor).await?;
            self.state.authority.inspect(actor, record.owner())?;
            projection::dispatch_actor(self.state.clone(), &self.runtime, record.owner()).await?;
            let active = record.active_attempt().ok_or(AiError::Conflict)?;
            active.prepared().validate()?;
            let account = self
                .runtime
                .read::<AiBudget>(&self.state.service, active.key().account_window())
                .await
                .map_err(map_error)?
                .value
                .ok_or(AiError::Denied)?
                .record()?;
            if !account
                .entries()
                .iter()
                .any(|entry| entry.key() == active.key() && entry.prepared() == active.prepared())
            {
                return Err(AiError::Denied);
            }
            projection::dispatch_attempt(
                self.state.clone(),
                &self.runtime,
                record.owner(),
                active.prepared(),
            )
            .await?;
            let evidence = record
                .attempt_evidence()
                .iter()
                .find(|evidence| evidence.attempt_id() == active.prepared().identity())
                .ok_or(AiError::Conflict)?;
            if tokio::time::Instant::now() >= lookup_deadline {
                return Err(AiError::UnknownOutcome);
            }
            let reconciled = tokio::time::timeout_at(
                lookup_deadline,
                self.state.provider.reconcile_observed_within(
                    active.prepared(),
                    evidence,
                    execution,
                ),
            )
            .await
            .map_err(|_| AiError::UnknownOutcome)??;
            reconciled.validate_for(active.prepared(), evidence)?;
            Ok((stamp, Some((active.clone(), evidence.clone(), reconciled))))
        })
        .await
        .map_err(|_| AiError::UnknownOutcome)??;
        let Some((active, evidence, reconciled)) = result else {
            if stamp.phase == OperationPhase::Pending {
                self.repair_known_settlement(actor, run, &stamp).await?;
            }
            return self.view(actor, run).await;
        };
        let reconciled = match reconciled {
            crate::ReconciliationObservation::Resolved(reconciled) => reconciled,
            crate::ReconciliationObservation::Uncertain { evidence, usage } => {
                super::worker::record_held_observation(
                    &self.state,
                    &self.runtime,
                    &run.0,
                    active.prepared(),
                    &evidence,
                    &usage,
                )
                .await?;
                self.finish(run, &stamp, OperationOutcome::Unresolved)
                    .await?;
                return self.view(actor, run).await;
            }
        };
        match reconciled {
            crate::Reconciliation::Unresolved => {
                self.finish(run, &stamp, OperationOutcome::Unresolved)
                    .await?
            }
            crate::Reconciliation::Accepted { completion } => {
                super::worker::publish_completion(
                    &self.state,
                    &self.runtime,
                    &run.0,
                    active.prepared(),
                    completion,
                    false,
                )
                .await?;
                self.finish(run, &stamp, OperationOutcome::Accepted).await?;
            }
            crate::Reconciliation::NotAccepted => {
                let latest = self
                    .runtime
                    .read::<AiRun>(&self.state.service, &run.0)
                    .await
                    .map_err(map_error)?;
                let current = latest.value.ok_or(AiError::InvalidRequest)?.record()?;
                if current.active_attempt() != Some(&active)
                    || current.checkpoint() != stamp.checkpoint
                {
                    return Err(AiError::Conflict);
                }
                if super::route_continuation::enabled(&current) {
                    let now = self.state.clock.now_unix_ms();
                    super::continuation_worker::confirm(
                        &self.state,
                        &self.runtime,
                        super::continuation_worker::ConfirmationRequest {
                            run: &run.0,
                            attempt: active.prepared(),
                            evidence: &evidence,
                            proof: super::NonacceptanceProof::Reconciled,
                            now,
                            due: now.checked_add(1).ok_or(AiError::DeadlineExceeded)?,
                        },
                    )
                    .await?;
                } else {
                    self.runtime
                        .execute(
                            &self.state.service,
                            rom::Command::action(
                                &run.0,
                                super::CHECKPOINT_RUN,
                                super::CheckpointRun::reconciled_not_accepted(
                                    stamp.checkpoint,
                                    self.state.clock.now_unix_ms(),
                                    evidence.clone(),
                                )?,
                            )
                            .at_revision(latest.revision)
                            .idempotency(&operation::command_key(&run.0, &stamp, "not-accepted")),
                        )
                        .await
                        .map_err(map_error)?;
                    super::worker::settle_run(
                        &self.state,
                        &self.runtime,
                        &run.0,
                        Some(UsdNanos(0)),
                    )
                    .await?;
                }
                self.finish(run, &stamp, OperationOutcome::NotAccepted)
                    .await?;
            }
        }
        self.view(actor, run).await
    }
    // Pending operations cannot repeat a lookup. Durable, exact-attempt cost knowledge
    // can nevertheless finish its interrupted financial commit without external I/O.
    async fn repair_known_settlement(
        &self,
        actor: &rom::Actor,
        run: &RunHandle,
        stamp: &OperationStamp,
    ) -> AiResult<()> {
        let actor = projection::current(&self.runtime, actor).await?;
        let snapshot = self
            .runtime
            .read::<AiRun>(&self.state.service, &run.0)
            .await
            .map_err(map_error)?;
        let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
        self.state.authority.inspect(&actor, record.owner())?;
        if record.state() != &super::RunState::AwaitingReconciliation
            || record.checkpoint() != stamp.checkpoint
        {
            return Ok(());
        }
        let active = record.active_attempt().ok_or(AiError::Conflict)?;
        let Some(index) = record
            .attempt_evidence()
            .iter()
            .position(|evidence| evidence.attempt_id() == active.prepared().identity())
        else {
            return Ok(());
        };
        if record.attempt_usage()[index].cost.is_none() {
            return Ok(());
        }
        super::worker::committed_attempt(&self.state, &self.runtime, &run.0, active.prepared())
            .await?;
        projection::dispatch_attempt(
            self.state.clone(),
            &self.runtime,
            record.owner(),
            active.prepared(),
        )
        .await?;
        super::worker::settle_record(&self.state, &self.runtime, &record, None).await?;
        // Settlement does not recover a pending tool. Preserve its operation stamp
        // and AiRun revision for the caller performing that recovery.
        if record
            .tool_turns
            .last()
            .is_some_and(|turn| !turn.completed())
        {
            return Ok(());
        }
        self.finish(run, stamp, OperationOutcome::Unresolved).await
    }
    async fn admit(
        &self,
        actor: &rom::Actor,
        run: &RunHandle,
        expected_revision: u64,
        key: &str,
        kind: OperationKind,
    ) -> AiResult<(OperationStamp, bool)> {
        run.validate()?;
        if !crate::request::valid_name(key, 128) || expected_revision == 0 {
            return Err(AiError::InvalidRequest);
        }
        let actor = projection::current(&self.runtime, actor).await?;
        let snapshot = self
            .runtime
            .read::<AiRun>(&self.state.service, &run.0)
            .await
            .map_err(map_error)?;
        let record = snapshot.value.ok_or(AiError::InvalidRequest)?.record()?;
        self.state.authority.inspect(&actor, record.owner())?;
        if kind == OperationKind::Cancel {
            self.state.authority.cancel(&actor, record.owner())?;
        }
        let requester = super::OwnerIdentity::from_actor(&actor)?;
        if let Some(old) = record.operations.iter().find(|stamp| stamp.key == key) {
            if old.kind != kind
                || old.original_expected_revision != expected_revision
                || old.requester != requester
            {
                return Err(map_error(rom::Error::IdentityMismatch));
            }
            return Ok((old.clone(), false));
        }
        if snapshot.revision != expected_revision {
            return Err(AiError::Conflict);
        }
        if record.operations.len() >= operation::MAX_OPERATIONS
            || (kind == OperationKind::Reconcile
                && record.counters().ticks() >= record.policy().limits().ticks)
        {
            return Err(AiError::BudgetExhausted);
        }
        let nonce = self
            .state
            .next_nonce
            .try_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |value| value.checked_add(1),
            )
            .map_err(|_| AiError::Closed)?;
        let stamp = OperationStamp {
            requester,
            key: key.into(),
            kind,
            original_expected_revision: expected_revision,
            checkpoint: record.checkpoint(),
            admitted_at_ms: self.state.clock.now_unix_ms(),
            nonce,
            phase: OperationPhase::Pending,
        };
        self.runtime
            .execute(
                &self.state.service,
                rom::Command::action(
                    &run.0,
                    BEGIN_OPERATION,
                    BeginOperation {
                        stamp: stamp.clone(),
                    },
                )
                .at_revision(expected_revision)
                .idempotency(&operation::command_key(&run.0, &stamp, "begin")),
            )
            .await
            .map_err(map_error)?;
        Ok((stamp, true))
    }
    async fn finish(
        &self,
        run: &RunHandle,
        stamp: &OperationStamp,
        category: OperationOutcome,
    ) -> AiResult<()> {
        let snapshot = self
            .runtime
            .read::<AiRun>(&self.state.service, &run.0)
            .await
            .map_err(map_error)?;
        self.runtime
            .execute(
                &self.state.service,
                rom::Command::action(
                    &run.0,
                    FINISH_OPERATION,
                    FinishOperation {
                        stamp: stamp.clone(),
                        category,
                        now_ms: self.state.clock.now_unix_ms(),
                    },
                )
                .at_revision(snapshot.revision)
                .idempotency(&operation::command_key(&run.0, stamp, "finish")),
            )
            .await
            .map_err(map_error)?;
        Ok(())
    }
}
impl fmt::Debug for FlowClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FlowClient { .. }")
    }
}
