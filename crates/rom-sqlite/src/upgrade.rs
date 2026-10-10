//! Explicit native format upgrade through a read-only coherent legacy snapshot.
use crate::{
    Sqlite,
    snapshot::{collect_upgrade_snapshot, read_snapshot},
};
use rom::{Descriptor, Result};
use rom_backup::BackupLimits;
use std::path::Path;

impl Sqlite {
    /// Upgrade a format-3 through format-8 database into a fresh current destination.
    /// The source is opened read-only. Descriptors must cover every stored kind;
    /// incompatible values or dangling live references prevent publication.
    /// Row values, receipts and pending work are retained; restore fences claims.
    pub fn upgrade_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        descriptors: &[Descriptor],
        limits: BackupLimits,
    ) -> Result<Self> {
        upgrade(
            source.as_ref(),
            destination.as_ref(),
            descriptors,
            limits,
            || Ok(()),
        )
    }

    /// Test-only interruption point after the native stage is closed and synced,
    /// immediately before publication. An error leaves the destination absent.
    #[cfg(feature = "test-support")]
    pub fn upgrade_from_observed(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        descriptors: &[Descriptor],
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        upgrade(
            source.as_ref(),
            destination.as_ref(),
            descriptors,
            limits,
            before_publish,
        )
    }
}

fn upgrade(
    source: &Path,
    destination: &Path,
    descriptors: &[Descriptor],
    limits: BackupLimits,
    before_publish: impl FnOnce() -> Result<()>,
) -> Result<Sqlite> {
    let source_owner =
        rom_backup::NativeOwnership::acquire(source, rom_backup::NativeAccess::Existing)?;
    let destination_owner =
        rom_backup::NativeOwnership::acquire(destination, rom_backup::NativeAccess::Fresh)?;
    let snapshot = read_snapshot(source_owner.path(), limits, |connection, limits| {
        collect_upgrade_snapshot(connection, limits, descriptors)
    })?;
    Sqlite::restore_snapshot(snapshot, destination_owner, limits, before_publish)
}
