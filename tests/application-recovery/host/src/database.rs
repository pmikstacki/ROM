use rom::{Result, Storage};
use rom_backup::BackupLimits;
use std::{path::Path, sync::Arc};
pub enum Database {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Database {
    pub fn open(adapter: &str, path: &Path) -> Result<Self> {
        match adapter {
            "sqlite" => rom_sqlite::Sqlite::open_with_limits(path, profile_limits())
                .map(|db| Self::Sqlite(Arc::new(db))),
            "redb" => rom_redb::Redb::open_with_limits(path, profile_limits())
                .map(|db| Self::Redb(Arc::new(db))),
            _ => Err(rom::Error::Denied),
        }
    }
    pub fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    pub fn backup(&self, path: &Path) -> Result<()> {
        let limits = BackupLimits {
            max_bytes: 128 * 1024 * 1024,
            max_records: 20_000,
        };
        match self {
            Self::Sqlite(db) => db.backup_to(path, limits),
            Self::Redb(db) => db.backup_to(path, limits),
        }?;
        Ok(())
    }
    pub fn restore(adapter: &str, archive: &Path, path: &Path) -> Result<Self> {
        let limits = BackupLimits {
            max_bytes: 128 * 1024 * 1024,
            max_records: 20_000,
        };
        match adapter {
            "sqlite" => rom_sqlite::Sqlite::restore_from(archive, path, limits)
                .map(|db| Self::Sqlite(Arc::new(db))),
            "redb" => rom_redb::Redb::restore_from(archive, path, limits)
                .map(|db| Self::Redb(Arc::new(db))),
            _ => Err(rom::Error::Denied),
        }
    }
}

fn profile_limits() -> rom::StorageLimits {
    rom::StorageLimits {
        journal_rows: 2048,
        ..rom::StorageLimits::default()
    }
}
