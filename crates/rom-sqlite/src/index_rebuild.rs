//! Explicit offline reconstruction of derived indexes into a fresh destination.
use crate::{
    Sqlite,
    snapshot::{collect_rebuild_snapshot, read_snapshot},
};
use rom::Result;
use rom_backup::BackupLimits;
use std::path::Path;

impl Sqlite {
    /// Rebuild indexes from bounded, validated authoritative records into a fresh path.
    /// Keep the source offline. Its known table inventory must remain intact.
    /// Source contents are not changed; publication fences cursors and pending claims.
    pub fn rebuild_indexes_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
    ) -> Result<Self> {
        rebuild(source.as_ref(), destination.as_ref(), limits, || Ok(()))
    }
    /// Test-only interruption immediately before publication of the complete rebuilt stage.
    #[cfg(feature = "test-support")]
    pub fn rebuild_indexes_from_observed(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        rebuild(
            source.as_ref(),
            destination.as_ref(),
            limits,
            before_publish,
        )
    }
}
fn rebuild(
    source: &Path,
    destination: &Path,
    limits: BackupLimits,
    before_publish: impl FnOnce() -> Result<()>,
) -> Result<Sqlite> {
    let snapshot = read_snapshot(source, limits, collect_rebuild_snapshot)?;
    Sqlite::restore_snapshot(snapshot, destination, limits, before_publish)
}
