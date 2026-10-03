//! Persisted uncertainty and fencing for opt-in delivery reconciliation.
use super::*;

pub(super) fn requires_reconciliation(record: &WorkRecord) -> bool {
    record.pending.delivery_profile == DeliveryProfile::ReconcileBeforeRetry
        && matches!(record.pending.payload, WorkPayload::Notification { .. })
        && matches!(
            record.delivery,
            Some(DeliveryOutcome::Unknown | DeliveryOutcome::TimedOut | DeliveryOutcome::Panicked)
        )
}

pub(super) fn await_reconciliation(record: &mut WorkRecord) -> Result<()> {
    record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
    record.state = WorkState::AwaitingReconciliation;
    Ok(())
}
