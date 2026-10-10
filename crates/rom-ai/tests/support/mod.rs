use rom::{Runtime, Storage};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
enum Db {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Db {
    fn open(redb: bool, path: &std::path::Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    fn storage(&self) -> Arc<dyn Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
        }
    }
}
pub struct Fixture {
    runtime: Option<Runtime>,
    db: Option<Db>,
    path: PathBuf,
    redb: bool,
}
impl Fixture {
    pub fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-ai-budget-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        let db = Db::open(redb, &path.join("database"));
        let runtime = Self::build(&db);
        Self {
            runtime: Some(runtime),
            db: Some(db),
            path,
            redb,
        }
    }
    fn build(db: &Db) -> Runtime {
        struct Clock;
        impl rom::Clock for Clock {
            fn now(&self) -> u64 {
                1
            }
        }
        Runtime::builder()
            .clock(Arc::new(Clock))
            .channel_with(
                rom_ai::flow::FLOW_TICKS
                    .delivery_profile(rom::DeliveryProfile::ReconcileBeforeRetry),
                super::service(),
                |_| async { rom::DeliveryOutcome::Accepted },
            )
            .delivery_timeout(std::time::Duration::from_secs(20))
            .limits(rom::Limits {
                command_bytes: 1024 * 1024,
                ..Default::default()
            })
            .resource(rom_ai::flow::AiRun::definition_for(&super::service()).unwrap())
            .resource(rom_ai::flow::AiBudget::definition_for(&super::service()).unwrap())
            .build(db.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap()
    }
    pub fn runtime(&self) -> &Runtime {
        self.runtime.as_ref().unwrap()
    }
    pub fn counts(&self) -> [u64; 4] {
        self.db.as_ref().unwrap().counts()
    }
    pub fn work(&self) -> Vec<rom::WorkRecord> {
        self.db
            .as_ref()
            .unwrap()
            .storage()
            .reaction_records()
            .unwrap()
    }
    pub async fn reopen(&mut self) {
        self.runtime().shutdown().await.unwrap();
        drop(self.runtime.take());
        drop(self.db.take());
        let db = Db::open(self.redb, &self.path.join("database"));
        self.runtime = Some(Self::build(&db));
        self.db = Some(db);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.runtime.take());
        drop(self.db.take());
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
