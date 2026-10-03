//! Offline schema migration with shared validation and native staged publication.
use crate::{
    Sqlite,
    snapshot::{collect_migration_snapshot, read_snapshot},
};
use rom::Result;
use rom_backup::{BackupLimits, MigrationPlan};
use std::path::Path;

impl Sqlite {
    /// Apply an explicit schema migration to an offline format-4 or format-5 source.
    /// Read the source without changes and publish only to a fresh destination.
    /// The shared plan validates transformed records and rebuilds reference edges.
    /// Publication fences old journal cursors and active work claims.
    pub fn migrate_from(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        plan: &MigrationPlan,
        limits: BackupLimits,
    ) -> Result<Self> {
        migrate(source.as_ref(), destination.as_ref(), plan, limits, || {
            Ok(())
        })
    }

    /// Test-only interruption point after the staged file is validated, closed and synced.
    #[cfg(feature = "test-support")]
    pub fn migrate_from_observed(
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        plan: &MigrationPlan,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        migrate(
            source.as_ref(),
            destination.as_ref(),
            plan,
            limits,
            before_publish,
        )
    }
}

fn migrate(
    source: &Path,
    destination: &Path,
    plan: &MigrationPlan,
    limits: BackupLimits,
    before_publish: impl FnOnce() -> Result<()>,
) -> Result<Sqlite> {
    let snapshot = read_snapshot(source, limits, collect_migration_snapshot)?;
    let snapshot = rom_backup::migrate_snapshot(snapshot, plan, limits)?;
    Sqlite::restore_snapshot(snapshot, destination, limits, before_publish)
}
