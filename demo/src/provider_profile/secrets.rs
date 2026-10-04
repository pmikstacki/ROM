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
pub(super) fn read_private(path: &std::path::Path, max_bytes: usize) -> Result<String> {
    crate::host_files::read_private(path, max_bytes)
}
