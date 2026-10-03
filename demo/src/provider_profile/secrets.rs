//! Bounded acquisition from approved immutable files in a trusted host directory.
use super::text::bounded;
use rom::{Error, Result};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

/// Approved opaque references to versioned secret files. Paths and material are not
/// exposed in errors or Debug output. Parent directories must be trusted by the host.
#[derive(Clone)]
pub struct SecretFiles {
    entries: Arc<BTreeMap<String, PathBuf>>,
}
impl SecretFiles {
    pub fn new(entries: BTreeMap<String, PathBuf>) -> Result<Self> {
        for key in entries.keys() {
            bounded(key, 2048)?;
        }
        Ok(Self {
            entries: Arc::new(entries),
        })
    }
    pub(super) fn read(&self, reference: &str) -> Result<String> {
        let path = self.entries.get(reference).ok_or(Error::Denied)?;
        read_private(path, 4096)
    }
}
#[cfg(target_os = "linux")]
pub(super) fn read_private(path: &std::path::Path, max_bytes: usize) -> Result<String> {
    use std::io::Read;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let bound = u64::try_from(max_bytes).map_err(|_| Error::Denied)?;
    let read_bound = bound.checked_add(1).ok_or(Error::Denied)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::Denied)?;
    let metadata = file.metadata().map_err(|_| Error::Denied)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 || metadata.len() > bound {
        return Err(Error::Denied);
    }
    let mut bytes = Vec::new();
    file.take(read_bound)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Denied)?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(Error::Denied);
    }
    String::from_utf8(bytes).map_err(|_| Error::Denied)
}
#[cfg(not(target_os = "linux"))]
pub(super) fn read_private(_: &std::path::Path, _: usize) -> Result<String> {
    // No weaker fallback: the reference profile requires Linux opened-handle checks.
    Err(Error::Denied)
}
