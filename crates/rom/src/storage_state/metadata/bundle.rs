//! One prepared native transaction for metadata and affected Work records.
use super::*;
use crate::storage_state::native_journal::{JournalImage, prepare_native_bundle};
use crate::storage_support::work::{WorkDelta, WorkRead};

impl StorageMetadata {
    /// Check before native Resource revision arbitration. Receipt replay bypasses stale claims.
    pub fn check_retry_epoch(
        &self,
        epoch: u64,
        replay_exists: bool,
        completed_work: Option<(&ClaimKey, u64)>,
        work: &impl WorkRead,
    ) -> Result<()> {
        self.retry_epochs.check(epoch, true, false)?;
        if replay_exists {
            return Ok(());
        }
        let mut edit = super::super::native_journal::prepare::new_edit(self.retry_epochs, work)?;
        super::super::native_journal::prepare::check_new_epoch(
            self.retry_epochs,
            epoch,
            completed_work,
            &mut edit,
        )
    }
}

/// Opaque native transaction candidate. Never deserialize this as caller authority.
pub struct BundleDelta {
    before: StorageMetadata,
    after: StorageMetadata,
    work: WorkDelta,
    retired: Vec<String>,
}
impl BundleDelta {
    pub fn before_metadata(&self) -> &StorageMetadata {
        &self.before
    }
    pub fn metadata(&self) -> &StorageMetadata {
        &self.after
    }
    pub fn work(&self) -> &WorkDelta {
        &self.work
    }
    pub fn retired(&self) -> &[String] {
        &self.retired
    }
    pub fn into_parts(self) -> (StorageMetadata, WorkDelta, Vec<String>) {
        (self.after, self.work, self.retired)
    }
}

/// Prepare only after receipt and Resource revision arbitration in the same native transaction.
pub fn prepare_bundle(
    metadata: &StorageMetadata,
    work: &impl WorkRead,
    bundle: &Bundle,
) -> Result<BundleDelta> {
    metadata
        .retry_epochs
        .check(bundle.receipt.retry_epoch, true, false)?;
    let mut image = JournalImage::from_metadata(metadata.clone())?;
    let delta = prepare_native_bundle(&image, work, bundle)?;
    let (journal, work) = delta.into_parts();
    let retired = journal
        .retired()
        .iter()
        .map(|e| e.event.identity.clone())
        .collect();
    image.apply(journal)?;
    Ok(BundleDelta {
        before: metadata.clone(),
        after: image.into_metadata(),
        work,
        retired,
    })
}
