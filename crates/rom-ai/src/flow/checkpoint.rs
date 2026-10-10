//! Pure checkpoint and cancellation transitions; no provider dispatch or worker.
use super::{
    codec::{self, Validated, input},
    resource::{AiRun, RunState},
};
use crate::{
    AiError, AiResult, AttemptEvidence, Usage,
    request::{MAX_OUTPUT_BYTES, bounded_value, valid_name},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowTick {
    pub run_id: String,
    pub sequence: u64,
}
impl FlowTick {
    pub fn new(run_id: impl Into<String>, sequence: u64) -> AiResult<Self> {
        let value = Self {
            run_id: run_id.into(),
            sequence,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> AiResult<()> {
        <Self as Validated>::validate(self)
    }
}
impl Validated for FlowTick {
    fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.run_id, 64) || !(1..=32).contains(&self.sequence) {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(FlowTick);
impl fmt::Debug for FlowTick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlowTick")
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}
pub const FLOW_TICKS: rom::Channel<FlowTick> = rom::Channel::new("ai_flow_ticks", 1);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum CheckpointOutcome {
    Completed {
        evidence: AttemptEvidence,
        usage: Usage,
        validated_output: Value,
    },
    Waiting {
        retry_at_unix_ms: u64,
        evidence: AttemptEvidence,
    },
    ReconciledNotAccepted {
        evidence: AttemptEvidence,
    },
    Failed {
        error: AiError,
        evidence: Option<AttemptEvidence>,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointRun {
    step: u32,
    now_unix_ms: u64,
    outcome: CheckpointOutcome,
}
impl CheckpointRun {
    /// The trusted service must apply its semantic OutputValidator before this action.
    pub fn completed(
        step: u32,
        now_unix_ms: u64,
        evidence: AttemptEvidence,
        usage: Usage,
        validated_output: Value,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            outcome: CheckpointOutcome::Completed {
                evidence,
                usage,
                validated_output,
            },
        };
        value.validate()?;
        Ok(value)
    }
    /// Only confirmed nonaccepted rate limits may schedule another attempt.
    pub fn waiting(
        step: u32,
        now_unix_ms: u64,
        retry_at_unix_ms: u64,
        evidence: AttemptEvidence,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            outcome: CheckpointOutcome::Waiting {
                retry_at_unix_ms,
                evidence,
            },
        };
        value.validate()?;
        Ok(value)
    }
    /// The trusted service must establish provider nonacceptance through reconciliation.
    /// Serialized input is neither evidence of nonacceptance nor an authorization grant.
    pub fn reconciled_not_accepted(
        step: u32,
        now_unix_ms: u64,
        evidence: AttemptEvidence,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            outcome: CheckpointOutcome::ReconciledNotAccepted { evidence },
        };
        value.validate()?;
        Ok(value)
    }
    /// The service must establish nonacceptance before failing a started request.
    pub fn confirmed_failure(
        step: u32,
        now_unix_ms: u64,
        error: AiError,
        evidence: Option<AttemptEvidence>,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            outcome: CheckpointOutcome::Failed { error, evidence },
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for CheckpointRun {
    fn validate(&self) -> AiResult<()> {
        if self.step > 32 {
            return Err(AiError::InvalidRequest);
        }
        match &self.outcome {
            CheckpointOutcome::Completed {
                evidence,
                validated_output,
                ..
            } => {
                if self.step == 0 {
                    return Err(AiError::InvalidRequest);
                }
                evidence.validate()?;
                bounded_value(validated_output, MAX_OUTPUT_BYTES)
                    .map_err(|_| AiError::InvalidOutput)?;
            }
            CheckpointOutcome::Waiting {
                retry_at_unix_ms,
                evidence,
            } => {
                if self.step == 0 || *retry_at_unix_ms <= self.now_unix_ms {
                    return Err(AiError::InvalidRequest);
                }
                evidence.validate()?;
            }
            CheckpointOutcome::ReconciledNotAccepted { evidence } => {
                if self.step == 0 {
                    return Err(AiError::InvalidRequest);
                }
                evidence.validate()?;
            }
            CheckpointOutcome::Failed { error, evidence } => {
                if matches!(error, AiError::UnknownOutcome | AiError::RateLimited { .. }) {
                    return Err(AiError::InvalidRequest);
                }
                if let Some(evidence) = evidence {
                    evidence.validate()?;
                }
            }
        }
        Ok(())
    }
}
input!(CheckpointRun);
impl fmt::Debug for CheckpointRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckpointRun")
            .field("step", &self.step)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRun {
    step: u32,
    now_unix_ms: u64,
}
impl CancelRun {
    pub fn new(step: u32, now_unix_ms: u64) -> AiResult<Self> {
        let value = Self { step, now_unix_ms };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for CancelRun {
    fn validate(&self) -> AiResult<()> {
        if self.step > 32 {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(CancelRun);
impl fmt::Debug for CancelRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelRun")
            .field("step", &self.step)
            .finish_non_exhaustive()
    }
}

pub const CHECKPOINT_RUN: rom::Action<AiRun, CheckpointRun> =
    rom::Action::new("checkpoint", |run, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = run.record().map_err(codec::rom_error)?;
        if record.checkpoint != input.step {
            return Err(rom::Error::Conflict);
        }
        let mut intents = Vec::new();
        match input.outcome {
            CheckpointOutcome::Completed {
                evidence,
                usage,
                validated_output,
            } => {
                if !matches!(
                    record.state,
                    RunState::Executing
                        | RunState::AwaitingReconciliation
                        | RunState::CancelRequested
                ) {
                    return Err(rom::Error::Conflict);
                }
                record
                    .observe_recovery(input.now_unix_ms)
                    .map_err(codec::rom_error)?;
                record
                    .note_evidence(evidence, usage)
                    .map_err(codec::rom_error)?;
                record.validated_output = Some(validated_output);
                record.failure = None;
                record.state = if record.cancel_requested {
                    RunState::Cancelled
                } else {
                    RunState::Completed
                };
            }
            CheckpointOutcome::Waiting {
                retry_at_unix_ms,
                evidence,
            } => {
                if !matches!(
                    record.state,
                    RunState::Executing | RunState::CancelRequested
                ) {
                    return Err(rom::Error::Conflict);
                }
                record
                    .observe_recovery(input.now_unix_ms)
                    .map_err(codec::rom_error)?;
                record
                    .note_evidence(evidence, Usage::default())
                    .map_err(codec::rom_error)?;
                intents.extend(schedule_next(&mut record, retry_at_unix_ms)?);
            }
            CheckpointOutcome::ReconciledNotAccepted { evidence } => {
                if record.state != RunState::AwaitingReconciliation {
                    return Err(rom::Error::Conflict);
                }
                record
                    .observe_recovery(input.now_unix_ms)
                    .map_err(codec::rom_error)?;
                let usage = record
                    .attempt_evidence()
                    .iter()
                    .position(|old| old.attempt_id() == evidence.attempt_id())
                    .map(|index| record.attempt_usage()[index].clone())
                    .unwrap_or_default();
                record
                    .note_evidence(evidence, usage)
                    .map_err(codec::rom_error)?;
                let retry_at = input.now_unix_ms.saturating_add(1);
                intents.extend(schedule_next(&mut record, retry_at)?);
            }
            CheckpointOutcome::Failed { error, evidence } => {
                if matches!(
                    record.state,
                    RunState::Completed | RunState::Failed | RunState::Cancelled
                ) {
                    return Err(rom::Error::Conflict);
                }
                if matches!(
                    record.state,
                    RunState::Executing
                        | RunState::AwaitingReconciliation
                        | RunState::CancelRequested
                ) && evidence.is_none()
                {
                    return Err(rom::Error::Conflict);
                }
                record
                    .observe_recovery(input.now_unix_ms)
                    .map_err(codec::rom_error)?;
                if let Some(evidence) = evidence {
                    record
                        .note_evidence(evidence, Usage::default())
                        .map_err(codec::rom_error)?;
                }
                record.failure = Some(error);
                record.state = if record.cancel_requested {
                    RunState::Cancelled
                } else {
                    RunState::Failed
                };
            }
        }
        record.mark();
        run.store(record).map_err(codec::rom_error)?;
        Ok(intents)
    });
pub(super) fn schedule_next(
    record: &mut super::resource::RunRecord,
    retry_at_unix_ms: u64,
) -> rom::Result<Vec<rom::Intent>> {
    let due_seconds = retry_at_unix_ms.div_ceil(1000);
    let due_ms = due_seconds.checked_mul(1000);
    if record.cancel_requested {
        record.state = RunState::Cancelled;
    } else if due_ms.is_none_or(|due| due >= record.expires_at_unix_ms)
        || record.last_observed_unix_ms >= record.expires_at_unix_ms
    {
        record.state = RunState::Failed;
        record.failure = Some(AiError::DeadlineExceeded);
    } else if record.counters.generation_attempts >= record.policy.limits().generation_attempts
        || record.counters.ticks >= record.policy.limits().ticks
    {
        record.state = RunState::Failed;
        record.failure = Some(AiError::BudgetExhausted);
    } else {
        record.state = RunState::Waiting {
            retry_at_unix_ms: due_ms.ok_or(rom::Error::Conflict)?,
        };
        return Ok(vec![
            FLOW_TICKS.intent_at(
                FlowTick::new(record.run_id.clone(), u64::from(record.checkpoint) + 1)
                    .map_err(codec::rom_error)?,
                due_seconds,
            ),
        ]);
    }
    Ok(vec![])
}
pub const CANCEL_RUN: rom::Action<AiRun, CancelRun> = rom::Action::new("cancel", |run, input| {
    input.validate().map_err(codec::rom_error)?;
    let mut record = run.record().map_err(codec::rom_error)?;
    if record.checkpoint != input.step
        || matches!(record.state, RunState::Completed | RunState::Failed)
    {
        return Err(rom::Error::Conflict);
    }
    if record.cancel_requested {
        return Ok(vec![]);
    }
    if record.state == RunState::ToolsPending {
        crate::tools::checkpoint::cancel(&mut record, input.now_unix_ms)
            .map_err(codec::rom_error)?;
        run.store(record).map_err(codec::rom_error)?;
        return Ok(vec![]);
    }
    record
        .observe_recovery(input.now_unix_ms)
        .map_err(codec::rom_error)?;
    record.cancel_requested = true;
    record.state = match record.state {
        RunState::Executing => RunState::CancelRequested,
        RunState::AwaitingReconciliation => RunState::AwaitingReconciliation,
        RunState::CancelRequested => RunState::CancelRequested,
        _ => RunState::Cancelled,
    };
    record.mark();
    run.store(record).map_err(codec::rom_error)?;
    Ok(vec![])
});
