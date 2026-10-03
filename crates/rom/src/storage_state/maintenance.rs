//! Atomic offline mapping of Resource values retained in journal and work metadata.
use super::{Error, Key, Result, StorageState, Value};

impl StorageState {
    /// Maintenance only: map Resource values in journal rows and frozen work sources.
    /// Preserves identities, Resource revisions, lifecycle state, budgets and opaque
    /// action and notification payloads. Changed work records advance their revision.
    /// Rejects growth beyond persisted bounds without eviction.
    /// An error leaves this state unchanged; callback side effects are not rolled back.
    pub fn map_resource_values(
        &mut self,
        transform: &mut impl FnMut(&Key, &Value) -> Result<Value>,
    ) -> Result<()> {
        let mut mapped = self.clone();
        let mut bytes = 0usize;
        for event in &mut mapped.events {
            event.row = event.row.map_resource_values(transform)?;
            bytes = bytes
                .checked_add(serde_json::to_vec(event).map_err(|_| Error::Storage)?.len())
                .ok_or(Error::TooLarge)?;
            if bytes > mapped.limits.journal_bytes {
                return Err(Error::TooLarge);
            }
        }
        mapped.work.map_resource_values(transform)?;
        *self = mapped;
        Ok(())
    }
}
