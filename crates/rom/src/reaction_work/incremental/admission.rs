//! Frozen policy admission and exact identity arbitration.
use super::*;
impl<R: WorkRead> Engine<'_, R> {
    pub(super) fn enqueue(
        &mut self,
        limits: &ReactionLimits,
        work: Vec<PendingWork>,
    ) -> Result<()> {
        limits.validate()?;
        self.compatible_capacity()?;
        if self
            .header
            .limits
            .as_ref()
            .is_some_and(|prior| prior != limits)
        {
            return Err(Error::Unsupported(
                "reaction policy differs from persisted policy".into(),
            ));
        }
        if self.header.limits.is_none() {
            self.header.accounting = LedgerBytes::empty(Some(limits))?;
            self.header.limits = Some(limits.clone());
        }
        for pending in work {
            if !matches!(pending.payload, WorkPayload::Notification { .. })
                && pending.delivery_profile != DeliveryProfile::AtLeastOnce
            {
                return Err(Error::Conflict);
            }
            if let Some(existing) = self.record(&pending.id)? {
                if existing.pending != pending {
                    return Err(Error::IdentityMismatch);
                }
                continue;
            }
            let due = pending.eligibility_floor();
            self.put(WorkRecord {
                pending,
                state: WorkState::Pending,
                attempts: 0,
                generation: 0,
                revision: 0,
                due,
                delivery: None,
            })?;
        }
        self.root_epochs()?;
        self.check_bounds()
    }
}
