use rom::{Error, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

/// Private sibling staging directory. Atomic hard-link publication refuses overwrite.
/// The parent directory must be trusted and on a filesystem supporting hard links/fsync.
pub struct Stage {
    directory: PathBuf,
    path: PathBuf,
    destination: PathBuf,
    published: AtomicBool,
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
                    published: AtomicBool::new(false),
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
        self.published.store(true, Ordering::Release);
        File::open(
            self.destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unknown)
    }

    /// Remove the known private hard link after successful native publication.
    /// Keep the destination's native ownership guard until its engine closes.
    /// Returns Storage before publication and Unknown for any later failure.
    /// Unknown links and the destination are never removed by this method.
    pub fn finish_native_publication(&self) -> Result<()> {
        if !self.published.load(Ordering::Acquire) {
            return Err(Error::Storage);
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let private = fs::symlink_metadata(&self.path).map_err(|_| Error::Unknown)?;
            let published = fs::symlink_metadata(&self.destination).map_err(|_| Error::Unknown)?;
            if !same_regular_file(&private, &published)
                || private.nlink() != 2
                || published.nlink() != 2
            {
                return Err(Error::Unknown);
            }
            fs::remove_file(&self.path).map_err(|_| Error::Unknown)?;
            File::open(&self.directory)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| Error::Unknown)?;
            let final_file = fs::symlink_metadata(&self.destination).map_err(|_| Error::Unknown)?;
            if !same_regular_file(&published, &final_file) || final_file.nlink() != 1 {
                return Err(Error::Unknown);
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(Error::Unknown)
        }
    }
}

#[cfg(target_os = "linux")]
fn same_regular_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.is_file() && right.is_file() && left.dev() == right.dev() && left.ino() == right.ino()
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
