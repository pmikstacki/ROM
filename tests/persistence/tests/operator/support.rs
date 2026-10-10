use rom::operator::*;
use rom::*;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub type Observer = Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
pub enum Db {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
    Closed,
}
impl Db {
    pub fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
    pub fn storage(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(db) => db,
            Self::Redb(db) => db,
            Self::Closed => panic!("closed fixture"),
        }
    }
    pub fn observe(&self, observer: Option<Observer>) {
        match self {
            Self::Sqlite(db) => db.on_commit(observer),
            Self::Redb(db) => db.on_commit(observer),
            Self::Closed => panic!("closed fixture"),
        }
    }
    pub fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
            Self::Closed => panic!("closed fixture"),
        }
    }
}
pub struct Fixture {
    pub db: Db,
    redb: bool,
    files: Files,
}
struct Files(PathBuf);
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Fixture {
    pub fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "rom-operator-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let files = Files(root);
        let db = Db::open(redb, &files.0.join("database"));
        seed(db.storage());
        Self { db, redb, files }
    }
    pub fn reopen(&mut self) {
        drop(std::mem::replace(&mut self.db, Db::Closed));
        self.db = Db::open(self.redb, &self.files.0.join("database"));
    }
}
#[derive(Clone, Resource)]
#[resource(name = "operator-fixtures")]
struct Item {
    value: String,
}
fn seed(storage: &dyn Storage) {
    storage.register(&[Item::descriptor()]).unwrap();
    let row = Row {
        key: Key {
            kind: Item::KIND.into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(
            Item {
                value: "kept".into(),
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    let pending = PendingWork {
        id: "pending-notice".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: None,
            depth: 0,
            started_at: 10,
            path: vec![],
        },
        definition: "notice".into(),
        version: 1,
        service_key: "service".into(),
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        payload: WorkPayload::Notification {
            source: row.clone(),
            payload: json!("private-notice"),
        },
    };
    storage
        .commit(&Bundle {
            expected: None,
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: None,
                identity: "seed".into(),
                fingerprint: "seed".into(),
                row,
            },
            changed: true,
            effects: vec![],
            reactions: vec![pending],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
}
pub fn control(snapshot: &StorageWorkSnapshot, principal: &str, key: &str) -> StorageWorkControl {
    StorageWorkControl {
        principal: principal.into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id(&snapshot.records[0].pending.id),
            expected: snapshot.version(&snapshot.records[0]),
            key: key.into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        },
        decision: WorkControlDecision::Retry,
        now: 20,
    }
}
pub fn receipt_count(snapshot: &StorageWorkSnapshot) -> usize {
    serde_json::to_value(snapshot).unwrap()["operator"]["receipts"]
        .as_object()
        .unwrap()
        .len()
}

impl Fixture {
    pub fn root(&self) -> &Path {
        &self.files.0
    }
    pub fn close(&mut self) {
        drop(std::mem::replace(&mut self.db, Db::Closed));
    }
}
