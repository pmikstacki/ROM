use rom::{Result, Storage};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "../support/child_process.rs"]
mod child_process;
pub use child_process::Process;

#[derive(Clone, Copy, Debug)]
pub enum Backend {
    Sqlite,
    Redb,
}
pub const BACKENDS: [Backend; 2] = [Backend::Sqlite, Backend::Redb];
impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Redb => "redb",
        }
    }
    pub fn open(self, path: &Path) -> Result<Box<dyn Storage>> {
        match self {
            Self::Sqlite => rom_sqlite::Sqlite::open(path).map(|s| Box::new(s) as _),
            Self::Redb => rom_redb::Redb::open(path).map(|s| Box::new(s) as _),
        }
    }
}

pub fn assert_saved(storage: &dyn Storage, value: &str, revision: u64) {
    use super::MemoryRecord;
    use rom::{Key, Resource};
    let row = storage
        .load(&Key {
            kind: MemoryRecord::KIND.into(),
            id: "saved".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(row.revision, revision);
    assert_eq!(
        row.value,
        Some(
            MemoryRecord {
                value: value.into()
            }
            .encode()
        )
    );
    assert!(storage.receipt("owner-process-commit").unwrap().is_some());
    assert_eq!(
        storage
            .snapshot(MemoryRecord::KIND, 10, 100_000)
            .unwrap()
            .len(),
        1
    );
}
pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-native-owner-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn path(&self) -> PathBuf {
        self.0.join("database")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn seed(storage: &dyn Storage, value: &str) {
    use super::MemoryRecord;
    use rom::{Bundle, Key, Receipt, Resource, Row};
    storage.register(&[MemoryRecord::descriptor()]).unwrap();
    storage
        .commit(&Bundle {
            expected: None,
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: None,
                identity: "owner-process-commit".into(),
                fingerprint: "owner-process-fingerprint".into(),
                row: Row {
                    key: Key {
                        kind: MemoryRecord::KIND.into(),
                        id: "saved".into(),
                    },
                    revision: 1,
                    value: Some(
                        MemoryRecord {
                            value: value.into(),
                        }
                        .encode(),
                    ),
                    protected: Default::default(),
                },
            },
        })
        .unwrap();
}

pub fn spawn_child(test: &str, root: &Path, backend: Backend) -> Process {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .env("ROM_OWNERSHIP_TEST_ROOT", root)
        .env("ROM_OWNERSHIP_TEST_BACKEND", backend.name())
        .stdin(std::process::Stdio::null());
    Process::spawn(command)
}
