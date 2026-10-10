//! Scheduling and verified resolution under original lifecycle budgets.
use super::delivery_profile::requires_reconciliation;
use super::*;
use crate::operator::{WorkControlOperation, WorkControlOutcome};

fn age_stop_reason(record: &WorkRecord, limits: &ReactionLimits, now: u64) -> Option<StopReason> {
    (now.saturating_sub(record.pending.cause.started_at) >= limits.max_age_seconds)
        .then_some(StopReason::Age)
}
fn attempt_budget_stop_reason(
    record: &WorkRecord,
    limits: &ReactionLimits,
    used: u32,
) -> Option<StopReason> {
    if record.attempts >= limits.max_attempts {
        Some(StopReason::Attempts)
    } else if used >= limits.max_work {
        Some(StopReason::WorkBudget)
    } else {
        None
    }
}
pub(super) fn retry_stop_reason(
    record: &WorkRecord,
    limits: &ReactionLimits,
    used: u32,
    now: u64,
) -> Option<StopReason> {
    if record.pending.cause.depth > limits.max_depth {
        Some(StopReason::Depth)
    } else {
        age_stop_reason(record, limits, now)
            .or_else(|| attempt_budget_stop_reason(record, limits, used))
    }
}
/// Preserve completion reason precedence independently of claim admission ordering.
pub(super) fn finish_retry_stop_reason(
    record: &WorkRecord,
    limits: &ReactionLimits,
    used: u32,
    now: u64,
) -> Option<StopReason> {
    attempt_budget_stop_reason(record, limits, used)
        .or_else(|| age_stop_reason(record, limits, now))
}
impl WorkLedger {
    pub(crate) fn policy(&self) -> Option<ReactionLimits> {
        self.limits.clone()
    }
    pub(crate) fn root_usage(&self) -> BTreeMap<String, u32> {
        self.roots.clone()
    }
    pub(crate) fn control(
        &mut self,
        id: &str,
        control: &StorageWorkControl,
    ) -> Result<WorkControlOutcome> {
        self.check_compatible_capacity()?;
        let limits = self.limits.as_ref().ok_or(Error::Storage)?;
        let record = self.work.get(id).ok_or(Error::Missing)?;
        // Expired leases go through ordinary recovery; operator control cannot steal them.
        if matches!(record.state, WorkState::Leased { .. } | WorkState::Done) {
            return Err(Error::Conflict);
        }
        let notification = matches!(record.pending.payload, WorkPayload::Notification { .. });
        let used = *self
            .roots
            .get(&record.pending.cause.root)
            .ok_or(Error::Storage)?;
        let reason =
            retry_stop_reason(record, limits, used, control.now).or_else(|| match &record.state {
                WorkState::Stopped(
                    reason @ (StopReason::Depth
                    | StopReason::Fanout
                    | StopReason::Attempts
                    | StopReason::WorkBudget
                    | StopReason::Age),
                ) => Some(reason.clone()),
                _ => None,
            });
        let (state, outcome, delivery) = match (&control.request.operation, &control.decision) {
            (WorkControlOperation::Retry, WorkControlDecision::Retry) => {
                if reason.is_some()
                    || record.state == WorkState::AwaitingReconciliation
                    || requires_reconciliation(record)
                {
                    return Err(Error::Conflict);
                }
                (
                    WorkState::Pending,
                    WorkControlOutcome::Scheduled,
                    record.delivery.clone(),
                )
            }
            (WorkControlOperation::Reconcile { .. }, WorkControlDecision::ActionCommitted)
                if matches!(record.pending.payload, WorkPayload::Action(_)) =>
            {
                (WorkState::Done, WorkControlOutcome::Completed, None)
            }
            (
                WorkControlOperation::Reconcile { .. },
                WorkControlDecision::DeliveryAccepted { .. },
            ) if notification => (
                WorkState::Done,
                WorkControlOutcome::Completed,
                Some(DeliveryOutcome::Accepted),
            ),
            (
                WorkControlOperation::Reconcile { .. },
                WorkControlDecision::DeliveryNotAccepted { .. },
            ) if notification => match reason {
                Some(reason) => (
                    WorkState::Stopped(reason.clone()),
                    WorkControlOutcome::Stopped(reason),
                    Some(DeliveryOutcome::Permanent),
                ),
                None => (
                    WorkState::Pending,
                    WorkControlOutcome::Scheduled,
                    Some(DeliveryOutcome::Permanent),
                ),
            },
            _ => return Err(Error::Conflict),
        };
        let mut next = self.clone();
        let record = next.work.get_mut(id).ok_or(Error::Storage)?;
        record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
        record.state = state;
        record.delivery = delivery;
        if outcome == WorkControlOutcome::Scheduled {
            record.due = control.now.max(record.pending.eligibility_floor());
        }
        self.publish(next)?;
        Ok(outcome)
    }
}
