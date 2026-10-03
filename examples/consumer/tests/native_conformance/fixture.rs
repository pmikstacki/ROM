//! An external fixture owns its private inspection seam and fresh backing store.
use rom::{Error, Result, Storage};
use rom_conformance::{StorageFacts, StorageFixture};
use rom_sqlite::Sqlite;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct Fixture {
    directory: PathBuf,
    store: Option<Arc<Sqlite>>,
    broken_counts: bool,
    reject_reopen: bool,
}
impl Fixture {
    pub fn new(broken_counts: bool) -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "rom-public-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).map_err(|_| Error::Storage)?;
        let store = match Sqlite::open(directory.join("db")) {
            Ok(s) => s,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&directory);
                return Err(e);
            }
        };
        Ok(Self {
            directory,
            store: Some(Arc::new(store)),
            broken_counts,
            reject_reopen: false,
        })
    }
    pub fn reject_reopen(mut self) -> Self {
        self.reject_reopen = true;
        self
    }
}
impl StorageFixture for Fixture {
    fn storage(&self) -> Arc<dyn Storage> {
        self.store.as_ref().unwrap().clone()
    }
    fn facts(&self) -> Result<StorageFacts> {
        let store = self.store.as_ref().ok_or(Error::Storage)?;
        Ok(StorageFacts {
            counts: if self.broken_counts {
                [0; 4]
            } else {
                store.counts()?
            },
            events: store.event_rows()?,
            effects: store.intentions()?,
        })
    }
    fn reopen(&mut self) -> Result<()> {
        if self.reject_reopen {
            return Err(Error::Unsupported("SECRET_REOPEN_ERROR".into()));
        }
        let store = self.store.as_ref().ok_or(Error::Storage)?;
        if Arc::strong_count(store) != 1 {
            return Err(Error::Unsupported("SECRET_REOPEN_ERROR".into()));
        }
        drop(self.store.take());
        self.store = Some(Arc::new(Sqlite::open(self.directory.join("db"))?));
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.store.take());
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
