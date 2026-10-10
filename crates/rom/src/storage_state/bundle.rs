//! Atomic Resource bundle accounting and journal updates.
use super::*;

impl StorageState {
    /// Apply only after native identity/revision arbitration. Returned identities left journal retention.
    pub fn bundle(&mut self, b: &Bundle) -> Result<Vec<String>> {
        let metadata = super::metadata::StorageMetadata::from_state(self);
        let mut work = crate::storage_support::work::WorkImage::from_ledger(
            self.work.clone(),
            self.retry_epochs,
        )?;
        let delta = super::metadata::prepare_bundle(&metadata, &work, b)?;
        let (metadata, work_delta, retired) = delta.into_parts();
        work.apply(work_delta)?;
        *self = metadata.reassemble(work.canonical(), self.operator.clone());
        Ok(retired)
    }
    pub(super) fn count_bundle(&mut self, b: &Bundle) -> Result<()> {
        self.receipts = self.receipts.checked_add(1).ok_or(Error::TooLarge)?;
        self.effects = self
            .effects
            .checked_add(b.effects.len())
            .ok_or(Error::TooLarge)?;
        if self.receipts > self.limits.receipts || self.effects > self.limits.effects {
            return Err(Error::Overloaded);
        }
        Ok(())
    }
}
