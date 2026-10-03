//! Persistent private sidecar acquisition and identity checks around the OS lock.
use super::metadata;
use rom::{Error, Result};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs::{File, Metadata, OpenOptions, TryLockError},
    io,
    path::{Path, PathBuf},
};

#[cfg(test)]
#[path = "lock_tests.rs"]
mod tests;

/// Construct only after successful locking. This covers post-lock validation
/// failures as well as normal NativeOwnership drop with one release path.
#[derive(Debug)]
pub(super) struct NativeLock {
    pub(super) file: File,
}

impl Drop for NativeLock {
    fn drop(&mut self) {
        // Closing alone can retain flock while a fork/exec child temporarily
        // holds a duplicate. Drop cannot report an unlock error; closing remains
        // the fallback, and never remove the persistent ownership sidecar.
        let _ = self.file.unlock();
    }
}

pub(super) fn acquire(database: &Path) -> Result<NativeLock> {
    let mut name = database.as_os_str().to_owned();
    name.push(".rom-owner");
    let path = PathBuf::from(name);
    let file = match metadata::inspect(&path)? {
        Some(before) => open_existing(&path, &before)?,
        None => match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                open_existing(&path, &metadata::inspect(&path)?.ok_or(Error::Storage)?)?
            }
            Err(_) => return Err(Error::Storage),
        },
    };
    verify(&path, &file)?;
    file.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => Error::Conflict,
        TryLockError::Error(error) if error.kind() == io::ErrorKind::Unsupported => {
            Error::Unsupported("filesystem native locking is unsupported".into())
        }
        TryLockError::Error(_) => Error::Storage,
    })?;
    let held = NativeLock { file };
    verify(&path, &held.file)?;
    Ok(held)
}

fn open_existing(path: &Path, before: &Metadata) -> Result<File> {
    let identity = private_file(before)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| Error::Storage)?;
    metadata::unchanged(
        identity,
        private_file(&file.metadata().map_err(|_| Error::Storage)?)?,
    )?;
    Ok(file)
}

fn private_file(metadata: &Metadata) -> Result<metadata::Identity> {
    let identity = metadata::single_file(metadata)?;
    if metadata.mode() & 0o077 != 0 {
        return Err(Error::Unsupported(
            "native ownership sidecars must have private permissions".into(),
        ));
    }
    Ok(identity)
}

fn verify(path: &Path, file: &File) -> Result<()> {
    let opened = private_file(&file.metadata().map_err(|_| Error::Storage)?)?;
    let current = private_file(&metadata::inspect(path)?.ok_or(Error::Storage)?)?;
    metadata::unchanged(opened, current)
}
