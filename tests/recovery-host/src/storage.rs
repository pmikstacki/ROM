use rom::{Result, Storage};
use std::{path::Path, sync::Arc};

pub struct FixtureStorage {
    pub store: Arc<dyn Storage>,
    pub counts: Arc<dyn Fn() -> Result<[u64; 4]> + Send + Sync>,
}

pub fn open(adapter: &str, path: &Path) -> Result<FixtureStorage> {
    match adapter {
        "sqlite" => {
            let store = Arc::new(rom_sqlite::Sqlite::open(path)?);
            let counted = store.clone();
            Ok(FixtureStorage {
                store,
                counts: Arc::new(move || counted.counts()),
            })
        }
        "redb" => {
            let store = Arc::new(rom_redb::Redb::open(path)?);
            let counted = store.clone();
            Ok(FixtureStorage {
                store,
                counts: Arc::new(move || counted.counts()),
            })
        }
        _ => Err(rom::Error::invalid("adapter", "sqlite or redb required")),
    }
}

pub fn retire(adapter: &str, source: &Path, destination: &Path) -> Result<()> {
    let policy = rom_backup::RetentionPolicy::new(rom::RetryEpochs {
        current: 1,
        admission_floor: 1,
        replay_floor: 1,
    });
    match adapter {
        "sqlite" => {
            let _ = rom_sqlite::Sqlite::retain_from(
                source,
                destination,
                &policy,
                rom_backup::BackupLimits::default(),
            )?;
        }
        "redb" => {
            let _ = rom_redb::Redb::retain_from(
                source,
                destination,
                &policy,
                rom_backup::BackupLimits::default(),
            )?;
        }
        _ => return Err(rom::Error::invalid("adapter", "sqlite or redb required")),
    }
    Ok(())
}
