use rom::*;
use rom_backup::{BackupLimits, RetentionPolicy};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

#[derive(Clone, Resource)]
#[resource(name = "epoch-sources")]
struct Source {
    count: u64,
}
#[derive(Clone, Resource)]
#[resource(name = "epoch-totals")]
struct Total {
    count: u64,
}
const NOTICE: Channel<String> = Channel::new("epoch-notice", 1);
const CHANGE: Action<Source, u64> = Action::new("change", |source, value| {
    source.count = value;
    Ok(vec![NOTICE.intent("source".into())])
});
const ADD: Action<Total, u64> = Action::new("add", |total, value| {
    total.count += value;
    Ok(vec![NOTICE.intent("total".into())])
});
fn actor() -> Actor {
    Actor::trusted("epoch-host", "owner")
}
fn worker() -> Actor {
    Actor::trusted("epoch-host", "worker").with_kind(PrincipalKind::Service)
}
fn builder(calls: Arc<AtomicUsize>, reactions: bool) -> Builder {
    let builder = Runtime::builder()
        .resource(
            Source::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(CHANGE),
        )
        .resource(
            Total::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(ADD),
        )
        .channel(NOTICE, worker(), move |_: Delivery<String>| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { DeliveryOutcome::Accepted }
        });
    if reactions {
        builder.reaction(Reaction::new(
            "epoch-sum",
            1,
            worker(),
            ADD,
            |source: &Snapshot<Source>| {
                Ok(vec![Target::new(
                    "total",
                    source.value.as_ref().unwrap().count,
                )])
            },
        ))
    } else {
        builder
    }
}
fn epochs(current: u64, admission_floor: u64, replay_floor: u64) -> RetryEpochs {
    RetryEpochs {
        current,
        admission_floor,
        replay_floor,
    }
}
struct Paths(PathBuf);
impl Paths {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "rom-runtime-epochs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Paths {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn open(redb: bool, path: &Path) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn retain(redb: bool, source: &Path, destination: &Path, epochs: RetryEpochs) -> Arc<dyn Storage> {
    let policy = RetentionPolicy::new(epochs);
    if redb {
        Arc::new(
            rom_redb::Redb::retain_from(source, destination, &policy, BackupLimits::default())
                .unwrap()
                .0,
        )
    } else {
        Arc::new(
            rom_sqlite::Sqlite::retain_from(source, destination, &policy, BackupLimits::default())
                .unwrap()
                .0,
        )
    }
}
#[tokio::test]
async fn sealed_epoch_drains_nonzero_root_actions_and_notifications_without_fresh_admission() {
    for redb in [false, true] {
        let paths = Paths::new();
        let origin = paths.at("origin");
        let active = paths.at("active");
        let sealed = paths.at("sealed");
        drop(open(redb, &origin));
        let storage = retain(redb, &origin, &active, epochs(1, 1, 1));
        let calls = Arc::new(AtomicUsize::new(0));
        let runtime = builder(calls.clone(), false)
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        runtime
            .execute(
                &actor(),
                Command::create("source", Source { count: 0 })
                    .idempotency("source")
                    .retry_epoch(1),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &actor(),
                Command::create("total", Total { count: 0 })
                    .idempotency("total")
                    .retry_epoch(1),
            )
            .await
            .unwrap();
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let runtime = builder(calls.clone(), true)
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let original: Invocation = Command::action("source", CHANGE, 3)
            .at_revision(1)
            .idempotency("change")
            .retry_epoch(1)
            .into();
        runtime.invoke(&actor(), original.clone()).await.unwrap();
        let pending = storage.reaction_records().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|r| r.pending.cause.retry_epoch == 1));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        runtime.shutdown().await.unwrap();
        drop(runtime);
        drop(storage);
        let storage = retain(redb, &active, &sealed, epochs(2, 2, 1));
        let runtime = builder(calls.clone(), true)
            .retry_fence(epochs(2, 2, 1))
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        runtime.invoke(&actor(), original.clone()).await.unwrap();
        let mut fresh = original;
        fresh.idempotency = "new-command".into();
        fresh.expected = Some(2);
        assert_eq!(
            runtime.invoke(&actor(), fresh).await,
            Err(Error::IdentityExpired)
        );
        runtime.process_work(16).await.unwrap();
        assert_eq!(
            runtime
                .read::<Total>(&actor(), "total")
                .await
                .unwrap()
                .value
                .unwrap()
                .count,
            3
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let records = storage.reaction_records().unwrap();
        assert!(
            records
                .iter()
                .all(|record| record.state == WorkState::Done
                    && record.pending.cause.retry_epoch == 1)
        );
        let actions: Vec<_> = records
            .iter()
            .filter_map(|record| {
                if let WorkPayload::Action(value) = &record.pending.payload {
                    Some(serde_json::from_value::<Invocation>(value.clone()).unwrap())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].retry_epoch, 1);
        runtime.invoke(&worker(), actions[0].clone()).await.unwrap();
        assert_eq!(runtime.process_work(16).await.unwrap(), 0);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        runtime.shutdown().await.unwrap();
    }
}

#[test]
fn trusted_fence_rejects_an_older_restored_backup() {
    for redb in [false, true] {
        let paths = Paths::new();
        let source = paths.at("source");
        let archive = paths.at("backup");
        let restored = paths.at("restored");
        let limits = BackupLimits::default();
        let storage: Arc<dyn Storage> = if redb {
            let storage = rom_redb::Redb::open(&source).unwrap();
            storage.backup_to(&archive, limits).unwrap();
            drop(storage);
            Arc::new(rom_redb::Redb::restore_from(&archive, &restored, limits).unwrap())
        } else {
            let storage = rom_sqlite::Sqlite::open(&source).unwrap();
            storage.backup_to(&archive, limits).unwrap();
            drop(storage);
            Arc::new(rom_sqlite::Sqlite::restore_from(&archive, &restored, limits).unwrap())
        };
        let result = builder(Arc::new(AtomicUsize::new(0)), false)
            .retry_fence(epochs(1, 1, 1))
            .build(storage, Runtime::shared_cpu_pool(1).unwrap());
        assert!(matches!(result, Err(Error::Unsupported(_))));
    }
}
