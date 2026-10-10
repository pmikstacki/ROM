//! Pure revision-checked Resource actions; no storage, provider I/O or scheduler.
use super::{
    budget::AiBudget,
    codec::{self, Validated, input},
    reservation::{ReservationEntry, ReservationStatus, ReserveBudget, SettleBudget},
    resource::{AiRun, RunState},
};
use crate::{AiError, AiResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareRun {
    entry: ReservationEntry,
    now_unix_ms: u64,
}
impl PrepareRun {
    /// The service must verify this exact entry against its committed account receipt first.
    pub fn new(entry: ReservationEntry, now_unix_ms: u64) -> AiResult<Self> {
        let value = Self { entry, now_unix_ms };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for PrepareRun {
    fn validate(&self) -> AiResult<()> {
        self.entry.validate()?;
        if self.entry.status() != &ReservationStatus::Reserved {
            return Err(AiError::InvalidRequest);
        }
        let deadline = self.entry.prepared().deadline();
        if self.now_unix_ms.checked_add(deadline.remaining_ms())
            != Some(deadline.expires_at_unix_ms())
        {
            return Err(AiError::DeadlineExceeded);
        }
        Ok(())
    }
}
input!(PrepareRun);
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartRun {
    step: u32,
    now_unix_ms: u64,
}
impl StartRun {
    pub fn new(step: u32, now_unix_ms: u64) -> AiResult<Self> {
        let value = Self { step, now_unix_ms };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for StartRun {
    fn validate(&self) -> AiResult<()> {
        if !(1..=32).contains(&self.step) {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(StartRun);
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldRun {
    step: u32,
    now_unix_ms: u64,
    evidence: Option<crate::AttemptEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage: Option<crate::Usage>,
}
impl HoldRun {
    pub fn new(step: u32, now_unix_ms: u64) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            evidence: None,
            usage: None,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn with_evidence(mut self, evidence: crate::AttemptEvidence) -> AiResult<Self> {
        self.evidence = Some(evidence);
        self.validate()?;
        Ok(self)
    }
    pub fn with_usage(mut self, usage: crate::Usage) -> AiResult<Self> {
        self.usage = Some(usage);
        self.validate()?;
        Ok(self)
    }
}
impl Validated for HoldRun {
    fn validate(&self) -> AiResult<()> {
        if !(1..=32).contains(&self.step) {
            return Err(AiError::InvalidRequest);
        }
        if let Some(evidence) = &self.evidence {
            evidence.validate()?;
        }
        Ok(())
    }
}
input!(HoldRun);

pub const RESERVE_BUDGET: rom::Action<AiBudget, ReserveBudget> =
    rom::Action::new("reserve", |budget, input| {
        let mut record = budget.record().map_err(codec::rom_error)?;
        record.reserve(input).map_err(|error| {
            if error == AiError::Conflict {
                rom::Error::IdentityMismatch
            } else {
                codec::rom_error(error)
            }
        })?;
        budget.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
pub const SETTLE_BUDGET: rom::Action<AiBudget, SettleBudget> =
    rom::Action::new("settle", |budget, input| {
        let mut record = budget.record().map_err(codec::rom_error)?;
        record.settle(input).map_err(codec::rom_error)?;
        budget.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
pub const PREPARE_RUN: rom::Action<AiRun, PrepareRun> =
    rom::Action::new("prepare", |run, input| {
        let record = apply_prepare(&run.record().map_err(codec::rom_error)?, &input)
            .map_err(codec::rom_error)?;
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
pub(super) fn apply_prepare(
    before: &super::RunRecord,
    input: &PrepareRun,
) -> AiResult<super::RunRecord> {
    input.validate()?;
    if !(matches!(before.state, RunState::Queued | RunState::Waiting { .. })
        || (before.state == RunState::ToolsPending
            && before.tool_turns.last().is_some_and(|t| t.completed())))
        || before.cancel_requested
        || input.entry.key().run_id() != before.run_id
        || input.entry.key().step() != before.checkpoint + 1
        || input.entry.key().attempt_ordinal() != before.counters.generation_attempts + 1
        || input.entry.prepared().request()
            != &before.effective_request(before.counters.generation_attempts + 1)?
        || input.entry.prepared().policy() != &before.policy
        || input.entry.prepared().deadline().expires_at_unix_ms() > before.expires_at_unix_ms
    {
        return Err(AiError::Conflict);
    }
    if let RunState::Waiting { retry_at_unix_ms } = before.state
        && input.now_unix_ms < retry_at_unix_ms
    {
        return Err(AiError::Conflict);
    }
    if let Some(old) = &before.cursor {
        let next = input.entry.prepared().route().next_cursor();
        if old.catalog_identity() != next.catalog_identity()
            || (old.tier() == crate::RoutingTier::Paid && next.tier() == crate::RoutingTier::Free)
            || (old.tier() == next.tier() && next.next_candidate() < old.next_candidate())
            || before
                .policy
                .free()
                .iter()
                .chain(before.policy.paid())
                .any(|m| old.is_rejected(m) && !next.is_rejected(m))
        {
            return Err(AiError::Conflict);
        }
    }
    let mut record = before.clone();
    super::route_continuation::consume(before, &mut record, &input.entry, input.now_unix_ms)?;
    record.observe(input.now_unix_ms)?;
    record.checkpoint += 1;
    record.counters.generation_attempts += 1;
    record.counters.ticks += 1;
    record.cursor = Some(input.entry.prepared().route().next_cursor().clone());
    record.active_attempt = Some(input.entry.clone());
    record.state = RunState::Prepared;
    record.mark();
    Ok(record)
}
pub const START_RUN: rom::Action<AiRun, StartRun> =
    rom::Action::new("start", |run, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = run.record().map_err(codec::rom_error)?;
        if record.state != RunState::Prepared || record.checkpoint != input.step {
            return Err(rom::Error::Conflict);
        }
        record
            .observe(input.now_unix_ms)
            .map_err(codec::rom_error)?;
        if record.active_attempt.as_ref().is_none_or(|entry| {
            input.now_unix_ms >= entry.prepared().deadline().expires_at_unix_ms()
        }) {
            return Err(rom::Error::Conflict);
        }
        record.state = RunState::Executing;
        record.mark();
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
pub const HOLD_RUN: rom::Action<AiRun, HoldRun> = rom::Action::new("hold", |run, input| {
    input.validate().map_err(codec::rom_error)?;
    let mut record = run.record().map_err(codec::rom_error)?;
    if !matches!(
        record.state,
        RunState::Executing | RunState::CancelRequested
    ) || record.checkpoint != input.step
    {
        return Err(rom::Error::Conflict);
    }
    record
        .observe_recovery(input.now_unix_ms)
        .map_err(codec::rom_error)?;
    record.state = RunState::AwaitingReconciliation;
    let identity = record
        .active_attempt
        .as_ref()
        .ok_or(rom::Error::Conflict)?
        .prepared()
        .identity()
        .to_owned();
    let existing = record
        .attempt_evidence()
        .iter()
        .position(|old| old.attempt_id() == identity);
    let evidence = input
        .evidence
        .or_else(|| existing.map(|index| record.attempt_evidence()[index].clone()))
        .unwrap_or(crate::AttemptEvidence::new(identity, None, None).map_err(codec::rom_error)?);
    let usage = input
        .usage
        .or_else(|| existing.map(|index| record.attempt_usage()[index].clone()))
        .unwrap_or_default();
    record
        .note_evidence(evidence, usage)
        .map_err(codec::rom_error)?;
    record.mark();
    run.store(record).map_err(codec::rom_error)?;
    Ok(vec![])
});
