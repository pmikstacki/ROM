use rom::*;
use rom_backup::{BackupLimits, MigrationPlan};
use std::{path::Path, sync::Arc};
pub enum Database {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Database {
    pub fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    pub fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    pub fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
        }
    }
    pub fn migrate(
        redb: bool,
        source: &Path,
        destination: &Path,
        plan: &MigrationPlan,
    ) -> Result<Self> {
        if redb {
            rom_redb::Redb::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|db| Self::Redb(Arc::new(db)))
        } else {
            rom_sqlite::Sqlite::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|db| Self::Sqlite(Arc::new(db)))
        }
    }
    pub fn backup(&self, path: &Path) -> rom_backup::Snapshot {
        let backend = match self {
            Self::Sqlite(db) => {
                db.backup_to(path, BackupLimits::default()).unwrap();
                rom_backup::Backend::Sqlite
            }
            Self::Redb(db) => {
                db.backup_to(path, BackupLimits::default()).unwrap();
                rom_backup::Backend::Redb
            }
        };
        rom_backup::read(path, backend, BackupLimits::default())
            .unwrap()
            .1
    }
}
