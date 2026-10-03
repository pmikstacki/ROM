//! Explicit native format upgrade through a read-only coherent legacy snapshot.
use crate::{Sqlite, snapshot::collect_legacy_snapshot};
use rom::{Descriptor, Error, Result};
use rom_backup::BackupLimits;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

impl Sqlite {
    /// Upgrade a format-3 database into a fresh format-4 destination.
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
    let snapshot = {
        let mut connection = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| Error::Storage)?;
        let transaction = connection.transaction().map_err(|_| Error::Storage)?;
        collect_legacy_snapshot(&transaction, limits)?
    };
    let snapshot = rom_backup::bind_legacy_schema(snapshot, descriptors, limits)?;
    Sqlite::restore_snapshot(snapshot, destination, limits, before_publish)
}
