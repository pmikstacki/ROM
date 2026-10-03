//! Native deployment ownership for cooperating processes on trusted local storage.
#[cfg(all(test, target_os = "linux"))]
#[path = "native_ownership/lifetime_tests.rs"]
mod lifetime_tests;
#[cfg(target_os = "linux")]
mod lock;
#[cfg(target_os = "linux")]
mod metadata;
#[cfg(target_os = "linux")]
mod path;
#[cfg(all(test, target_os = "linux"))]
#[path = "../tests/native_ownership/support.rs"]
mod test_support;

use rom::Result;
use std::path::{Path, PathBuf};

/// Filesystem intent checked while holding the native ownership reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeAccess {
    /// Permit an existing database or reserve its absent path for engine creation.
    OpenOrCreate,
    /// Require an existing source without creating a missing source or its sidecar.
    Existing,
    /// Reserve a destination and reject an existing database after locking it.
    Fresh,
}

/// Non-cloneable native ownership guard. Keep it until the native engine closes.
///
/// The adjacent `.rom-owner` file persists after drop. Never delete or replace it
/// to recover ownership. Only local Linux storage is currently supported.
/// Drop explicitly unlocks before closing the descriptor, so incidental descriptor
/// inheritance cannot extend the lifetime of this guard's ownership.
#[derive(Debug)]
pub struct NativeOwnership {
    path: PathBuf,
    #[cfg(target_os = "linux")]
    _lock: lock::NativeLock,
}

impl NativeOwnership {
    /// Acquire nonblocking ownership of a canonical database path.
    ///
    /// The parent directory must be trusted. Live renames, replacement, hard-link
    /// aliases and network filesystems are outside this deployment contract.
    pub fn acquire(path: &Path, access: NativeAccess) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let prepared = path::Prepared::new(path, access)?;
            let file = lock::acquire(&prepared.path)?;
            prepared.recheck(access)?;
            Ok(Self {
                path: prepared.path,
                _lock: file,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (path, access);
            Err(rom::Error::Unsupported(
                "native ownership requires validated local Linux storage".into(),
            ))
        }
    }

    /// Canonical filesystem path to pass to the database engine or publication.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
