//! Host-owned native maintenance. Every mutation still uses the public ROM contracts.
use rom::{Result, Storage};
use rom_backup::{BackupLimits, MigrationPlan};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) enum Database {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Database {
    pub(super) fn open(redb: bool, path: &Path) -> Result<Self> {
        if redb {
            rom_redb::Redb::open(path).map(|s| Self::Redb(Arc::new(s)))
        } else {
            rom_sqlite::Sqlite::open(path).map(|s| Self::Sqlite(Arc::new(s)))
        }
    }
    pub(super) fn migrate(
        redb: bool,
        source: &Path,
        destination: &Path,
        plan: &MigrationPlan,
    ) -> Result<Self> {
        if redb {
            rom_redb::Redb::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|s| Self::Redb(Arc::new(s)))
        } else {
            rom_sqlite::Sqlite::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|s| Self::Sqlite(Arc::new(s)))
        }
    }
    pub(super) fn backup(&self, path: &Path) -> Result<()> {
        match self {
            Self::Sqlite(s) => s.backup_to(path, BackupLimits::default()),
            Self::Redb(s) => s.backup_to(path, BackupLimits::default()),
        }?;
        Ok(())
    }
    pub(super) fn restore(redb: bool, archive: &Path, destination: &Path) -> Result<Self> {
        if redb {
            rom_redb::Redb::restore_from(archive, destination, BackupLimits::default())
                .map(|s| Self::Redb(Arc::new(s)))
        } else {
            rom_sqlite::Sqlite::restore_from(archive, destination, BackupLimits::default())
                .map(|s| Self::Sqlite(Arc::new(s)))
        }
    }
    pub(super) fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(s) => s.clone(),
            Self::Redb(s) => s.clone(),
        }
    }
    pub(super) fn intentions(&self) -> Result<Vec<(String, rom::Intent)>> {
        match self {
            Self::Sqlite(s) => s.intentions(),
            Self::Redb(s) => s.intentions(),
        }
    }
}
/// Exclude lock bookkeeping in SQLite SHM; committed main-file and WAL bytes must not change.
pub(super) fn source_bytes(source: &Path) -> std::io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut files = vec![(source.to_path_buf(), std::fs::read(source)?)];
    let mut wal = source.as_os_str().to_os_string();
    wal.push("-wal");
    let wal = PathBuf::from(wal);
    match std::fs::read(&wal) {
        Ok(bytes) => files.push((wal, bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(files)
}
pub(super) struct Scratch(pub(super) PathBuf);
impl Scratch {
    pub(super) fn new() -> crate::smoke::SmokeResult<Self> {
        let path = std::env::temp_dir().join(format!(
            "rom-demo-upgrade-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
