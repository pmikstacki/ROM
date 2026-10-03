//! Transactional work updates under the persisted retry boundaries.
use super::*;

impl StorageState {
    /// Native adapters persist this candidate only after the complete work update
    /// passes epoch integrity checks. An error leaves claims and budgets unchanged.
    pub fn update_work(&mut self, update: WorkUpdate) -> Result<WorkResult> {
        let mut next = self.clone();
        let result = next.work.apply(update)?;
        next.work.validate_retry_epochs(next.retry_epochs)?;
        *self = next;
        Ok(result)
    }
}
