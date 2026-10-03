//! Inspect the native format without changing an unsupported source.
use crate::format::{FORMAT, META};
use redb::{Database, DatabaseError, ReadOnlyDatabase, ReadableDatabase};
use rom::{Error, Result};
use std::{fs::File, path::Path};

pub(super) fn check(path: &Path) -> Result<()> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == 0 => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(Error::Storage),
    }
    inspect(path, marker)
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

fn marker(tx: &redb::ReadTransaction) -> Result<()> {
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
        return Ok(());
    }
    let table = tx
        .open_table(META)
        .map_err(|_| Error::Unsupported("redb format marker missing".into()))?;
    if table
        .get("format")
        .map_err(|_| Error::Storage)?
        .map(|value| value.value())
        != Some(FORMAT)
    {
        return Err(Error::Unsupported("redb storage format".into()));
    }
    Ok(())
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
