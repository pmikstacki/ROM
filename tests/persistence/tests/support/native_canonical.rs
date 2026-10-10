//! Canonical export for offline test fixtures; uses the public adapter boundary.
use rom::StorageLimits;
use rom_backup::{Backend, BackupLimits};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Archive(PathBuf);
impl Drop for Archive {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

pub fn state(redb: bool, source: &Path, limits: StorageLimits) -> serde_json::Value {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = source.parent().unwrap().join(format!(
        "canonical-fixture-{}-{}.rom",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let backend = if redb { Backend::Redb } else { Backend::Sqlite };
    if redb {
        let db = rom_redb::Redb::open_with_limits(source, limits).unwrap();
        db.backup_to(&path, BackupLimits::default()).unwrap();
        drop(db);
    } else {
        let db = rom_sqlite::Sqlite::open_with_limits(source, limits).unwrap();
        db.backup_to(&path, BackupLimits::default()).unwrap();
        drop(db);
    }
    let archive = Archive(path);
    let (_, snapshot) = rom_backup::read(&archive.0, backend, BackupLimits::default()).unwrap();
    serde_json::to_value(snapshot.state).unwrap()
}
