//! Explicit host policy for bounded offline history reclamation.
use rom::{Key, RetryEpochs};
use serde::Serialize;
use std::collections::BTreeSet;

/// No default TTL or automatic external-delivery settlement is applied.
#[derive(Clone, Serialize)]
pub struct RetentionPolicy {
    pub(crate) epochs: RetryEpochs,
    pub(crate) journal_through: Option<u64>,
    pub(crate) settled_effects: BTreeSet<String>,
    pub(crate) tombstones: BTreeSet<Key>,
}
impl RetentionPolicy {
    pub fn new(epochs: RetryEpochs) -> Self {
        Self {
            epochs,
            journal_through: None,
            settled_effects: BTreeSet::new(),
            tombstones: BTreeSet::new(),
        }
    }
    /// Remove the contiguous journal prefix through this inclusive position.
    pub fn journal_through(mut self, position: u64) -> Self {
        self.journal_through = Some(position);
        self
    }
    /// Assert that all external effects of this expired receipt are settled.
    /// ROM cannot verify an external provider's acceptance from a raw Intent.
    pub fn settle_effects(mut self, receipt_identity: impl Into<String>) -> Self {
        self.settled_effects.insert(receipt_identity.into());
        self
    }
    /// Purge a deleted row only if no retained history, reference or work pins it.
    pub fn purge_tombstone(mut self, key: Key) -> Self {
        self.tombstones.insert(key);
        self
    }
}

/// Payload-free counts for an operator's maintenance record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionReport {
    pub retry_epochs: RetryEpochs,
    pub receipts_removed: usize,
    pub receipts_retained: usize,
    pub events_removed: usize,
    pub effects_removed: usize,
    pub work_records_removed: usize,
    pub work_roots_removed: usize,
    pub tombstones_removed: usize,
}
