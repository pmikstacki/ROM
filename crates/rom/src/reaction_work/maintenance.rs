//! Mapping of frozen Resource snapshots inside a private migration candidate.
use super::{Key, Result, Value, WorkLedger, WorkPayload};

impl WorkLedger {
    /// Called only on the enclosing storage state's uncommitted clone.
    pub(crate) fn map_resource_values(
        &mut self,
        transform: &mut impl FnMut(&Key, &Value) -> Result<Value>,
    ) -> Result<()> {
        for record in self.work.values_mut() {
            match &mut record.pending.payload {
                WorkPayload::Source(row) | WorkPayload::Notification { source: row, .. } => {
                    *row = row.map_resource_values(transform)?;
                }
                WorkPayload::Action(_) => {}
            }
        }
        self.check_bounds()
    }
}
