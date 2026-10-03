//! Explicit read-only legacy native upgrade into the current format database.
use crate::{Redb, maintenance::read_upgrade_snapshot};
use rom::{Descriptor, Result};
use rom_backup::BackupLimits;
use std::path::Path;

impl Redb {
    /// Upgrade an offline format-3 through format-6 source into a fresh database.
    /// Format 3 binds the supplied schema; catalogued formats require an exact match.
    /// Format 6 retains retry epochs and receipt origins without rebinding.
    /// Values are preserved; invalid layouts and dangling references require separate repair.
    /// The source must be offline. Existing destinations are never overwritten.
    /// An unclean source needs temporary disk space approximately equal to its size
    /// for a private recovery copy; the original source remains unchanged.
    pub fn upgrade_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        descriptors: &[Descriptor],
        limits: BackupLimits,
    ) -> Result<Self> {
        Self::upgrade_snapshot(
            source.as_ref(),
            destination.as_ref(),
            descriptors,
            limits,
            || Ok(()),
        )
    }

    /// Test-only interruption point after the staged file is validated, closed and synced.
    #[cfg(feature = "test-support")]
    pub fn upgrade_from_observed(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        descriptors: &[Descriptor],
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        Self::upgrade_snapshot(
            source.as_ref(),
            destination.as_ref(),
            descriptors,
            limits,
            before_publish,
        )
    }

    fn upgrade_snapshot(
        source: &Path,
        destination: &Path,
        descriptors: &[Descriptor],
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        let snapshot = read_upgrade_snapshot(source, limits, descriptors)?;
        Self::restore_snapshot(snapshot, destination, limits, before_publish)
    }
}
