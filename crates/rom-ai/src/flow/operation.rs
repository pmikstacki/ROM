//! Bounded run-local operation admission facts; ROM remains the receipt authority.
use super::{
    AiRun, OwnerIdentity, RunState,
    codec::{self, Validated, input},
};
use crate::{AiError, AiResult, request::valid_name};
use serde::{Deserialize, Serialize};
pub(crate) const MAX_OPERATIONS: usize = 32;
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OperationKind {
    Cancel,
    Reconcile,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OperationOutcome {
    Cancelled,
    Unresolved,
    Accepted,
    NotAccepted,
    Rejected,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum OperationPhase {
    Pending,
    Finished {
        category: OperationOutcome,
        finished_at_ms: u64,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationStamp {
    pub requester: OwnerIdentity,
    pub key: String,
    pub kind: OperationKind,
    pub original_expected_revision: u64,
    pub checkpoint: u32,
    pub admitted_at_ms: u64,
    pub nonce: u64,
    pub phase: OperationPhase,
}
impl Validated for OperationStamp {
    fn validate(&self) -> AiResult<()> {
        self.requester.validate()?;
        if !valid_name(&self.key, 128)
            || self.original_expected_revision == 0
            || self.original_expected_revision == u64::MAX
            || self.checkpoint > 32
            || self.nonce == 0
        {
            return Err(AiError::InvalidRequest);
        }
        if self.kind == OperationKind::Reconcile && self.checkpoint == 0 {
            return Err(AiError::InvalidRequest);
        }
        if let OperationPhase::Finished { finished_at_ms, .. } = self.phase
            && finished_at_ms < self.admitted_at_ms
        {
            return Err(AiError::DeadlineExceeded);
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BeginOperation {
    pub stamp: OperationStamp,
}
impl Validated for BeginOperation {
    fn validate(&self) -> AiResult<()> {
        self.stamp.validate()?;
        if self.stamp.phase != OperationPhase::Pending {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(BeginOperation);
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FinishOperation {
    pub stamp: OperationStamp,
    pub category: OperationOutcome,
    pub now_ms: u64,
}
impl Validated for FinishOperation {
    fn validate(&self) -> AiResult<()> {
        self.stamp.validate()?;
        if self.stamp.phase != OperationPhase::Pending || self.now_ms < self.stamp.admitted_at_ms {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
input!(FinishOperation);
pub(crate) fn command_key(run: &str, stamp: &OperationStamp, phase: &str) -> String {
    serde_json::json!([
        "ai-flow-operation-v1",
        run,
        stamp.requester,
        stamp.key,
        phase
    ])
    .to_string()
}
pub(crate) const BEGIN_OPERATION: rom::Action<AiRun, BeginOperation> =
    rom::Action::new("ai_begin_operation", |run, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = run.record().map_err(codec::rom_error)?;
        if record
            .operations
            .iter()
            .any(|old| old.key == input.stamp.key)
        {
            return Err(rom::Error::IdentityMismatch);
        }
        if record.operations.len() >= MAX_OPERATIONS {
            return Err(codec::rom_error(AiError::BudgetExhausted));
        }
        if record.checkpoint() != input.stamp.checkpoint {
            return Err(rom::Error::Conflict);
        }
        match input.stamp.kind {
            OperationKind::Cancel => {
                if matches!(record.state(), RunState::Completed | RunState::Failed) {
                    return Err(rom::Error::Conflict);
                }
            }
            OperationKind::Reconcile => {
                if !matches!(
                    record.state(),
                    RunState::AwaitingReconciliation
                        | RunState::Executing
                        | RunState::CancelRequested
                ) {
                    return Err(rom::Error::Conflict);
                }
                record.counters.ticks = record
                    .counters
                    .ticks
                    .checked_add(1)
                    .ok_or(rom::Error::TooLarge)?;
            }
        }
        if input.stamp.admitted_at_ms < record.last_observed_unix_ms() {
            return Err(codec::rom_error(AiError::DeadlineExceeded));
        }
        if input.stamp.kind == OperationKind::Reconcile {
            record
                .observe_recovery(input.stamp.admitted_at_ms)
                .map_err(codec::rom_error)?;
        }
        record.operations.push(input.stamp);
        record.mark();
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
pub(crate) const FINISH_OPERATION: rom::Action<AiRun, FinishOperation> =
    rom::Action::new("ai_finish_operation", |run, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = run.record().map_err(codec::rom_error)?;
        let index = record
            .operations
            .iter()
            .position(|old| old.key == input.stamp.key)
            .ok_or(rom::Error::IdentityMismatch)?;
        if record.operations[index] != input.stamp {
            return Err(rom::Error::IdentityMismatch);
        }
        if (input.stamp.kind == OperationKind::Cancel
            && input.category != OperationOutcome::Cancelled)
            || (input.category == OperationOutcome::Cancelled && !record.cancel_requested())
            || (input.category == OperationOutcome::Accepted && record.validated_output.is_none())
            || (input.category == OperationOutcome::NotAccepted
                && !matches!(
                    record.state(),
                    RunState::Waiting { .. } | RunState::Failed | RunState::Cancelled
                ))
        {
            return Err(rom::Error::Conflict);
        }
        record
            .observe_recovery(input.now_ms)
            .map_err(codec::rom_error)?;
        record.operations[index].phase = OperationPhase::Finished {
            category: input.category,
            finished_at_ms: input.now_ms,
        };
        record.mark();
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });

/// Permit only admission or one immutable pending-to-finished phase change.
pub(crate) fn transition(after: &super::RunRecord, before: &super::RunRecord) -> AiResult<bool> {
    if after.operations == before.operations {
        return Ok(false);
    }
    let mut expected = before.clone();
    if after.operations.len() == before.operations.len() + 1
        && after.operations[..before.operations.len()] == before.operations
    {
        let added = after.operations.last().ok_or(AiError::Conflict)?;
        if added.phase != OperationPhase::Pending || added.checkpoint != before.checkpoint() {
            return Err(AiError::Conflict);
        }
        match added.kind {
            OperationKind::Cancel => {
                if matches!(before.state(), RunState::Completed | RunState::Failed) {
                    return Err(AiError::Conflict);
                }
            }
            OperationKind::Reconcile => {
                if !matches!(
                    before.state(),
                    RunState::Executing
                        | RunState::AwaitingReconciliation
                        | RunState::CancelRequested
                ) {
                    return Err(AiError::Conflict);
                }
                expected.counters.ticks += 1;
            }
        }
        if added.kind == OperationKind::Reconcile {
            expected.last_observed_unix_ms = added.admitted_at_ms;
        }
    } else if after.operations.len() == before.operations.len() {
        let mut changes = 0;
        for (old, new) in before.operations.iter().zip(&after.operations) {
            if old == new {
                continue;
            }
            changes += 1;
            let mut finished = old.clone();
            let OperationPhase::Finished { finished_at_ms, .. } = new.phase else {
                return Err(AiError::Conflict);
            };
            if old.phase != OperationPhase::Pending {
                return Err(AiError::Conflict);
            }
            if let OperationPhase::Finished { category, .. } = new.phase
                && ((old.kind == OperationKind::Cancel && category != OperationOutcome::Cancelled)
                    || (category == OperationOutcome::Cancelled && !before.cancel_requested())
                    || (category == OperationOutcome::Accepted
                        && before.validated_output.is_none())
                    || (category == OperationOutcome::NotAccepted
                        && !matches!(
                            before.state(),
                            RunState::Waiting { .. } | RunState::Failed | RunState::Cancelled
                        )))
            {
                return Err(AiError::Conflict);
            }
            finished.phase = new.phase.clone();
            if &finished != new {
                return Err(AiError::Conflict);
            }
            expected.last_observed_unix_ms = finished_at_ms;
        }
        if changes != 1 {
            return Err(AiError::Conflict);
        }
    } else {
        return Err(AiError::Conflict);
    }
    expected.operations = after.operations.clone();
    expected.mark();
    if &expected != after {
        return Err(AiError::Conflict);
    }
    Ok(true)
}
