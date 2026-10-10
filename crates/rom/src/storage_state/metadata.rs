//! Bounded native metadata, separate from retained Work and operator records.
use super::*;
mod bundle;
pub use bundle::{BundleDelta, prepare_bundle};

/// Trusted persistence data. This is not a transport or authorization descriptor.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageMetadata {
    pub(super) retry_epochs: RetryEpochs,
    pub(super) limits: StorageLimits,
    pub(super) generation: String,
    pub(super) head: u64,
    pub(super) floor: u64,
    pub(super) receipts: usize,
    pub(super) effects: usize,
    pub(super) events: Vec<JournalEvent>,
}

impl StorageMetadata {
    /// Move a validated canonical state into native persistence components.
    pub fn split(state: StorageState) -> (Self, WorkLedger, OperatorLedger) {
        let StorageState {
            work,
            operator,
            retry_epochs,
            limits,
            generation,
            head,
            floor,
            receipts,
            effects,
            events,
        } = state;
        (
            Self {
                retry_epochs,
                limits,
                generation,
                head,
                floor,
                receipts,
                effects,
                events,
            },
            work,
            operator,
        )
    }

    /// Reconstruct canonical data for complete archive validation, not ordinary writes.
    pub fn reassemble(self, work: WorkLedger, operator: OperatorLedger) -> StorageState {
        let Self {
            retry_epochs,
            limits,
            generation,
            head,
            floor,
            receipts,
            effects,
            events,
        } = self;
        StorageState {
            work,
            operator,
            retry_epochs,
            limits,
            generation,
            head,
            floor,
            receipts,
            effects,
            events,
        }
    }

    /// Copy only bounded metadata; retained Work and operator history are not copied.
    pub fn from_state(state: &StorageState) -> Self {
        Self {
            retry_epochs: state.retry_epochs,
            limits: state.limits.clone(),
            generation: state.generation.clone(),
            head: state.head,
            floor: state.floor,
            receipts: state.receipts,
            effects: state.effects,
            events: state.events.clone(),
        }
    }

    pub fn retry_epochs(&self) -> RetryEpochs {
        self.retry_epochs
    }
    pub fn storage_limits(&self) -> StorageLimits {
        self.limits.clone()
    }
    pub fn journal_head(&self, kind: &str) -> JournalCursor {
        JournalCursor {
            generation: self.generation.clone(),
            kind: kind.into(),
            position: self.head,
        }
    }

    /// Use shared paging rules through an admitted canonical journal image.
    pub fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        let image = super::native_journal::JournalImage::from_metadata(self.clone())?;
        super::native_journal::journal_page(&image, kind, after, max_rows, max_bytes)
    }
}
