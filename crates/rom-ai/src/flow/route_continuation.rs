//! Versioned forward decisions. An old active cursor remains coupled to its old attempt.
use super::{
    AiRun, ReservationEntry, ReservationKey, RunRecord, RunState,
    codec::{self, Validated, input},
};
use crate::{
    AiError, AiResult, AttemptEvidence, FailoverPolicy, RouteCursor, RouteDecision, Usage, UsdNanos,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NonacceptanceProof {
    RateLimited,
    Reconciled,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmNonacceptance {
    pub(crate) step: u32,
    pub(crate) observed_at_ms: u64,
    pub(crate) not_before_ms: u64,
    pub(crate) evidence: AttemptEvidence,
    pub(crate) proof: NonacceptanceProof,
}
impl ConfirmNonacceptance {
    /// Trusted host only: these existing Provider contracts prove uncharged nonacceptance.
    /// Raw status codes, Uncertain outcomes, and missing reconciliation IDs are not proofs.
    pub fn new(
        step: u32,
        now: u64,
        due: u64,
        evidence: AttemptEvidence,
        proof: NonacceptanceProof,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            observed_at_ms: now,
            not_before_ms: due,
            evidence,
            proof,
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for ConfirmNonacceptance {
    fn validate(&self) -> AiResult<()> {
        self.evidence.validate()?;
        if !(1..=32).contains(&self.step)
            || self.not_before_ms <= self.observed_at_ms
            || (self.proof == NonacceptanceProof::RateLimited
                && (self.evidence.generation_id().is_some()
                    || self.not_before_ms - self.observed_at_ms > 300_000))
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(ConfirmNonacceptance);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SuccessorPlan {
    pub(crate) entry: ReservationEntry,
    pub(crate) account_revision: u64,
    pub(crate) planned_at_ms: u64,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsumedSuccessor {
    pub(crate) identity: String,
    pub(crate) eligibility_catalog_identity: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RouteContinuation {
    version: u32,
    pub(crate) predecessor: ReservationKey,
    predecessor_route: RouteDecision,
    pub(crate) proof: ConfirmNonacceptance,
    // None explicitly holds the original ceiling. Default Usage alone never means zero.
    pub(crate) confirmed_cost: Option<UsdNanos>,
    pub(crate) forward_cursor: RouteCursor,
    pub(crate) staged: Option<SuccessorPlan>,
    pub(crate) consumed: Option<ConsumedSuccessor>,
}
pub(crate) fn pending(record: &RunRecord) -> Option<&RouteContinuation> {
    record
        .route_continuations
        .last()
        .filter(|c| c.consumed.is_none())
}
pub(crate) fn enabled(record: &RunRecord) -> bool {
    record.policy.failover() == FailoverPolicy::AdvanceOnConfirmedNonacceptance
}
pub(crate) fn apply_confirmed(
    before: &RunRecord,
    input: &ConfirmNonacceptance,
) -> rom::Result<(RunRecord, Vec<rom::Intent>)> {
    input.validate().map_err(codec::rom_error)?;
    let active = before.active_attempt().ok_or(rom::Error::Conflict)?;
    if !enabled(before)
        || pending(before).is_some()
        || before.checkpoint != input.step
        || active.prepared().identity() != input.evidence.attempt_id()
        || before
            .route_continuations
            .iter()
            .any(|c| c.predecessor == *active.key())
        || match input.proof {
            NonacceptanceProof::RateLimited => !matches!(
                before.state,
                RunState::Executing | RunState::CancelRequested
            ),
            NonacceptanceProof::Reconciled => before.state != RunState::AwaitingReconciliation,
        }
    {
        return Err(rom::Error::Conflict);
    }
    let mut after = before.clone();
    after
        .observe_recovery(input.observed_at_ms)
        .map_err(codec::rom_error)?;
    let previous = before
        .attempt_evidence()
        .iter()
        .position(|e| e.attempt_id() == input.evidence.attempt_id());
    let usage = previous
        .map(|i| before.attempt_usage()[i].clone())
        .unwrap_or_default();
    // HTTP refusal cannot override observed generation or usage. An authenticated
    // reconciliation may prove nonacceptance while preserving a known positive fee.
    if input.proof == NonacceptanceProof::RateLimited
        && (usage != Usage::default()
            || previous.is_some_and(|i| before.attempt_evidence()[i].generation_id().is_some()))
    {
        return Err(rom::Error::Conflict);
    }
    after
        .note_evidence(input.evidence.clone(), usage.clone())
        .map_err(codec::rom_error)?;
    let forward_cursor = active
        .prepared()
        .route()
        .next_cursor()
        .reject(active.prepared().route().model())
        .map_err(codec::rom_error)?;
    after.route_continuations.push(RouteContinuation {
        version: 1,
        predecessor: active.key().clone(),
        predecessor_route: active.prepared().route().clone(),
        proof: input.clone(),
        confirmed_cost: Some(usage.cost.unwrap_or(UsdNanos(0))),
        forward_cursor,
        staged: None,
        consumed: None,
    });
    let intents = super::checkpoint::schedule_next(&mut after, input.not_before_ms)?;
    after.mark();
    Ok((after, intents))
}
pub const CONFIRM_NONACCEPTANCE: rom::Action<AiRun, ConfirmNonacceptance> =
    rom::Action::new("confirm_nonacceptance", |run, input| {
        let (record, intents) = apply_confirmed(&run.record().map_err(codec::rom_error)?, &input)?;
        run.store(record).map_err(codec::rom_error)?;
        Ok(intents)
    });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageSuccessor {
    plan: SuccessorPlan,
}
impl StageSuccessor {
    pub fn new(
        entry: ReservationEntry,
        account_revision: u64,
        planned_at_ms: u64,
    ) -> AiResult<Self> {
        let value = Self {
            plan: SuccessorPlan {
                entry,
                account_revision,
                planned_at_ms,
            },
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for StageSuccessor {
    fn validate(&self) -> AiResult<()> {
        self.plan.entry.validate()?;
        let deadline = self.plan.entry.prepared().deadline();
        if self.plan.entry.status() != &super::ReservationStatus::Reserved
            || self.plan.planned_at_ms.checked_add(deadline.remaining_ms())
                != Some(deadline.expires_at_unix_ms())
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(StageSuccessor);
pub(crate) fn validate_successor(before: &RunRecord, entry: &ReservationEntry) -> AiResult<()> {
    let continuation = pending(before).ok_or(AiError::Conflict)?;
    let route = entry.prepared().route();
    let next = route.next_cursor();
    let old = &continuation.forward_cursor;
    entry.validate()?;
    if entry.key().run_id() != before.run_id()
        || entry.key().step() != before.checkpoint + 1
        || entry.key().attempt_ordinal() != before.counters.generation_attempts + 1
        || entry.prepared().request()
            != &before.effective_request(before.counters.generation_attempts + 1)?
        || entry.prepared().policy() != before.policy()
        || route.eligibility_catalog_identity().is_none()
        || next.catalog_identity() != old.catalog_identity()
        || (old.tier() == crate::RoutingTier::Paid && next.tier() == crate::RoutingTier::Free)
        || (old.tier() == next.tier() && next.next_candidate() <= old.next_candidate())
        || before
            .policy
            .free()
            .iter()
            .chain(before.policy.paid())
            .any(|m| old.is_rejected(m) && !next.is_rejected(m))
        || !next.is_rejected(continuation.predecessor_route.model())
        || entry.prepared().deadline().expires_at_unix_ms() > before.expires_at_unix_ms
    {
        return Err(AiError::Conflict);
    }
    Ok(())
}
fn apply_stage(before: &RunRecord, input: &StageSuccessor) -> AiResult<RunRecord> {
    input.validate()?;
    validate_successor(before, &input.plan.entry)?;
    if !matches!(before.state, RunState::Waiting { retry_at_unix_ms } if input.plan.planned_at_ms >= retry_at_unix_ms)
        || before.cancel_requested
        || pending(before).is_none_or(|c| c.staged.is_some())
    {
        return Err(AiError::Conflict);
    }
    let mut after = before.clone();
    // Staging is not an attempt or a scheduler tick. Preserve the Waiting milestone
    // time; PREPARE_RUN will observe the frozen plan time after account admission.
    if input.plan.planned_at_ms < before.last_observed_unix_ms
        || input.plan.planned_at_ms >= before.expires_at_unix_ms
    {
        return Err(AiError::DeadlineExceeded);
    }
    after
        .route_continuations
        .last_mut()
        .ok_or(AiError::Conflict)?
        .staged = Some(input.plan.clone());
    Ok(after)
}
pub const STAGE_SUCCESSOR: rom::Action<AiRun, StageSuccessor> =
    rom::Action::new("stage_successor", |run, input| {
        let record = apply_stage(&run.record().map_err(codec::rom_error)?, &input)
            .map_err(codec::rom_error)?;
        run.store(record).map_err(|error| {
            if error == AiError::BudgetExhausted {
                rom::Error::TooLarge
            } else {
                codec::rom_error(error)
            }
        })?;
        Ok(vec![])
    });
pub(crate) fn consume(
    before: &RunRecord,
    after: &mut RunRecord,
    entry: &ReservationEntry,
    now: u64,
) -> AiResult<()> {
    let Some(c) = pending(before) else {
        return Ok(());
    };
    validate_successor(before, entry)?;
    let plan = c.staged.as_ref().ok_or(AiError::Conflict)?;
    if plan.entry != *entry || plan.planned_at_ms != now {
        return Err(AiError::Conflict);
    }
    let c = after
        .route_continuations
        .last_mut()
        .ok_or(AiError::Conflict)?;
    c.staged = None;
    c.consumed = Some(ConsumedSuccessor {
        identity: entry.prepared().identity().into(),
        eligibility_catalog_identity: entry
            .prepared()
            .route()
            .eligibility_catalog_identity()
            .ok_or(AiError::Conflict)?
            .into(),
    });
    Ok(())
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailContinuation {
    now: u64,
    error: AiError,
}
impl FailContinuation {
    pub fn new(now: u64, error: AiError) -> AiResult<Self> {
        let value = Self { now, error };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for FailContinuation {
    fn validate(&self) -> AiResult<()> {
        if !matches!(
            self.error,
            AiError::BudgetExhausted
                | AiError::ProviderUnavailable
                | AiError::UnsupportedCapability
                | AiError::Denied
                | AiError::DeadlineExceeded
                | AiError::InvalidRequest
        ) {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(FailContinuation);
fn apply_failed(before: &RunRecord, input: &FailContinuation) -> AiResult<RunRecord> {
    input.validate()?;
    if pending(before).is_none()
        || !matches!(before.state, RunState::Waiting { .. })
        || before.cancel_requested
    {
        return Err(AiError::Conflict);
    }
    let mut after = before.clone();
    after.observe_recovery(input.now)?;
    after.state = RunState::Failed;
    after.failure = Some(input.error.clone());
    after.mark();
    Ok(after)
}
pub const FAIL_CONTINUATION: rom::Action<AiRun, FailContinuation> =
    rom::Action::new("fail_continuation", |run, input| {
        let record = apply_failed(&run.record().map_err(codec::rom_error)?, &input)
            .map_err(codec::rom_error)?;
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });

pub(crate) fn transition(after: &RunRecord, before: &RunRecord) -> AiResult<bool> {
    if after.route_continuations == before.route_continuations {
        if pending(before).is_some()
            && matches!(before.state, RunState::Waiting { .. })
            && after.state == RunState::Failed
        {
            let input = FailContinuation::new(
                after.last_observed_unix_ms,
                after.failure.clone().ok_or(AiError::Conflict)?,
            )?;
            if after != &apply_failed(before, &input)? {
                return Err(AiError::Conflict);
            }
            return Ok(true);
        }
        return Ok(false);
    }
    let expected = if after.route_continuations.len() == before.route_continuations.len() + 1 {
        apply_confirmed(
            before,
            &after
                .route_continuations
                .last()
                .ok_or(AiError::Conflict)?
                .proof,
        )
        .map_err(|_| AiError::Conflict)?
        .0
    } else if after.route_continuations.len() == before.route_continuations.len() {
        let latest = after.route_continuations.last().ok_or(AiError::Conflict)?;
        if latest.consumed.is_some() {
            let plan = pending(before)
                .and_then(|c| c.staged.as_ref())
                .ok_or(AiError::Conflict)?;
            super::actions::apply_prepare(
                before,
                &super::PrepareRun::new(plan.entry.clone(), plan.planned_at_ms)?,
            )?
        } else {
            let plan = latest.staged.clone().ok_or(AiError::Conflict)?;
            apply_stage(before, &StageSuccessor { plan })?
        }
    } else {
        return Err(AiError::Conflict);
    };
    if after != &expected {
        return Err(AiError::Conflict);
    }
    Ok(true)
}
pub(crate) fn validate(record: &RunRecord) -> AiResult<()> {
    if enabled(record)
        && matches!(record.state, RunState::Waiting { .. })
        && pending(record).is_none()
    {
        return Err(AiError::Conflict);
    }
    if record.route_continuations.len() > record.policy.limits().generation_attempts as usize
        || (!record.route_continuations.is_empty() && !enabled(record))
    {
        return Err(AiError::InvalidRequest);
    }
    for (i, c) in record.route_continuations.iter().enumerate() {
        c.proof.validate()?;
        c.predecessor.validate()?;
        c.forward_cursor.validate()?;
        let pos = record
            .attempt_evidence()
            .iter()
            .position(|e| e.attempt_id() == c.predecessor.attempt_identity())
            .ok_or(AiError::Conflict)?;
        if c.version != 1
            || c.predecessor.run_id() != record.run_id()
            || c.predecessor.policy_version() != record.policy.version()
            || c.predecessor.attempt_ordinal() > record.counters.generation_attempts
            || c.proof.step != c.predecessor.step()
            || c.confirmed_cost
                .is_some_and(|v| v > c.predecessor_route.maximum_cost())
            || c.proof.evidence != record.attempt_evidence()[pos]
            || c.forward_cursor
                != c.predecessor_route
                    .next_cursor()
                    .reject(c.predecessor_route.model())?
            || record.attempt_usage()[pos]
                .cost
                .is_some_and(|cost| c.confirmed_cost != Some(cost))
            || record.route_continuations[..i].iter().any(|old| {
                old.predecessor == c.predecessor
                    || old.predecessor.attempt_ordinal() >= c.predecessor.attempt_ordinal()
            })
            || (c.consumed.is_none() && i + 1 != record.route_continuations.len())
            || (c.consumed.is_some() && c.staged.is_some())
        {
            return Err(AiError::InvalidRequest);
        }
        if let Some(consumed) = &c.consumed {
            if !crate::request::valid_name(&consumed.identity, 160)
                || !crate::request::valid_name(&consumed.eligibility_catalog_identity, 128)
                || consumed.identity
                    != format!(
                        "{}:{}:{}",
                        record.run_id(),
                        c.proof.step + 1,
                        c.predecessor.attempt_ordinal() + 1
                    )
                || c.predecessor.attempt_ordinal() >= record.counters.generation_attempts
            {
                return Err(AiError::InvalidRequest);
            }
        } else {
            let active = record.active_attempt().ok_or(AiError::Conflict)?;
            if active.key() != &c.predecessor || active.prepared().route() != &c.predecessor_route {
                return Err(AiError::Conflict);
            }
            if let Some(plan) = &c.staged {
                StageSuccessor { plan: plan.clone() }.validate()?;
                validate_successor(record, &plan.entry)?;
            }
        }
    }
    Ok(())
}
