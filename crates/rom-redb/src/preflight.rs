//! Admit existing native state before a writable engine can change allocator metadata.
use crate::format::META;
use redb::{Database, DatabaseError, ReadOnlyDatabase, ReadTransaction, ReadableDatabase};
use rom::{Error, Result, StorageLimits};
use rom_backup::BackupLimits;
use std::{fs::File, path::Path};

pub(super) fn check(
    path: &Path,
    format: u64,
    storage_limits: &StorageLimits,
    validation_limits: BackupLimits,
) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(Error::Storage),
    }
    inspect(path, |tx| {
        validate_existing(tx, format, storage_limits, validation_limits).map(|_| ())
    })
}

/// Read a native snapshot without modifying its source, recovering only a private copy if needed.
pub(super) fn inspect<T>(
    path: &Path,
    inspect: impl FnOnce(&redb::ReadTransaction) -> Result<T>,
) -> Result<T> {
    match ReadOnlyDatabase::open(path) {
        Ok(database) => inspect(&database.begin_read().map_err(|_| Error::Storage)?),
        // redb cannot read a dirty allocator in read-only mode. Recover only a private
        // copy for inspection; the original remains untouched on success or rejection.
        Err(DatabaseError::RepairAborted) => recovered_inspect(path, inspect),
        Err(_) => Err(Error::Storage),
    }
}

/// Shared by readonly admission and the actual opened transaction.
/// Returns true only when the engine has no native tables to validate.
pub(super) fn validate_existing(
    tx: &ReadTransaction,
    format: u64,
    storage_limits: &StorageLimits,
    validation_limits: BackupLimits,
) -> Result<bool> {
    if tx
        .list_tables()
        .map_err(|_| Error::Storage)?
        .next()
        .is_none()
        && tx
            .list_multimap_tables()
            .map_err(|_| Error::Storage)?
            .next()
            .is_none()
    {
        return Ok(true);
    }
    let table = tx
        .open_table(META)
        .map_err(|_| Error::Unsupported("redb format marker missing".into()))?;
    if table
        .get("format")
        .map_err(|_| Error::Storage)?
        .map(|value| value.value())
        != Some(format)
    {
        return Err(Error::Unsupported("redb storage format".into()));
    }
    let snapshot = crate::maintenance::snapshot_in_format(
        tx,
        validation_limits,
        crate::maintenance::NativeFormat::Exact(format),
    )?;
    snapshot.state.check_limits(storage_limits)?;
    snapshot.validate()?;
    Ok(false)
}

fn recovered_inspect<T>(
    source: &Path,
    inspect: impl FnOnce(&redb::ReadTransaction) -> Result<T>,
) -> Result<T> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Storage)?
        .as_nanos();
    let destination =
        std::env::temp_dir().join(format!(".rom-format-probe-{}-{nonce}", std::process::id()));
    let stage = rom_backup::Stage::new(&destination)?;
    let mut input = File::open(source).map_err(|_| Error::Storage)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(stage.path())
        .map_err(|_| Error::Storage)?;
    // Stream the physical file to private disk; do not retain it in memory or copy
    // source permissions onto the writable probe. No logical records are repaired.
    std::io::copy(&mut input, &mut output).map_err(|_| Error::Storage)?;
    drop(output);
    let database = Database::open(stage.path()).map_err(|_| Error::Storage)?;
    inspect(&database.begin_read().map_err(|_| Error::Storage)?)
}
