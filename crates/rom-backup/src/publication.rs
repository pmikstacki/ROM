use rom::{Error, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Private sibling staging directory. Atomic hard-link publication refuses overwrite.
/// The parent directory must be trusted and on a filesystem supporting hard links/fsync.
pub struct Stage {
    directory: PathBuf,
    path: PathBuf,
    destination: PathBuf,
}
impl Stage {
    pub fn new(destination: &Path) -> Result<Self> {
        #[cfg(not(unix))]
        {
            let _ = destination;
            return Err(Error::Unsupported("private archives require Unix".into()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            static SEQ: AtomicU64 = AtomicU64::new(0);
            if fs::symlink_metadata(destination).is_ok() {
                return Err(Error::Conflict);
            }
            let parent = destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            for _ in 0..32 {
                let directory = parent.join(format!(
                    ".rom-maintenance-{}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| Error::Storage)?
                        .as_nanos(),
                    SEQ.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::DirBuilder::new().mode(0o700).create(&directory) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(_) => return Err(Error::Storage),
                }
                let stage = Self {
                    path: directory.join("data"),
                    directory,
                    destination: destination.into(),
                };
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&stage.path)
                    .map_err(|_| Error::Storage)?;
                return Ok(stage);
            }
            Err(Error::Storage)
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn publish(&self) -> Result<()> {
        self.publish_with(|| Ok(()))
    }
    /// Sync the staged file, run the observation hook, then publish without overwrite.
    /// A hook error leaves the destination absent. Close native writers before calling.
    pub fn publish_with(&self, before_publish: impl FnOnce() -> Result<()>) -> Result<()> {
        File::open(&self.path)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        before_publish()?;
        fs::hard_link(&self.path, &self.destination).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Conflict
            } else {
                Error::Storage
            }
        })?;
        File::open(
            self.destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unknown)
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
