//! Offline retention through shared policy validation and fresh native publication.
use crate::{
    Redb,
    maintenance::{NativeFormat, read_snapshot},
};
use rom::Result;
use rom_backup::{BackupLimits, RetentionPolicy, RetentionReport};
use std::path::Path;
impl Redb {
    /// Apply retention to an offline current-format source without changing its files.
    /// Publish only to a fresh destination; fence old cursors and active work claims.
    pub fn retain_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        policy: &RetentionPolicy,
        limits: BackupLimits,
    ) -> Result<(Self, RetentionReport)> {
        retain(
            source.as_ref(),
            destination.as_ref(),
            policy,
            limits,
            || Ok(()),
        )
    }
    /// Test-only interruption point after the staged file is validated, closed and synced.
    #[cfg(feature = "test-support")]
    pub fn retain_from_observed(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        policy: &RetentionPolicy,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<(Self, RetentionReport)> {
        retain(
            source.as_ref(),
            destination.as_ref(),
            policy,
            limits,
            before_publish,
        )
    }
}
fn retain(
    source: &Path,
    destination: &Path,
    policy: &RetentionPolicy,
    limits: BackupLimits,
    before_publish: impl FnOnce() -> Result<()>,
) -> Result<(Redb, RetentionReport)> {
    let snapshot = read_snapshot(source, limits, NativeFormat::Current)?;
    let (snapshot, report) = rom_backup::retain_snapshot(snapshot, policy, limits)?;
    let storage = Redb::restore_snapshot(snapshot, destination, limits, before_publish)?;
    Ok((storage, report))
}
