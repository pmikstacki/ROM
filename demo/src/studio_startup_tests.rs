use crate::studio_semantic::FieldShowcase;
use crate::{Task, studio_application, studio_startup};
use rom::{Command, Key, Patch, Resource, ResourceRef, Row, Storage};
use std::{path::Path, sync::Arc};

type NativeCounts = Box<dyn Fn() -> [u64; 4]>;
fn open(path: &Path, redb: bool) -> (Arc<dyn Storage>, NativeCounts) {
    if redb {
        let storage = Arc::new(rom_redb::Redb::open(path).unwrap());
        let inspected = storage.clone();
        (storage, Box::new(move || inspected.counts().unwrap()))
    } else {
        let storage = Arc::new(rom_sqlite::Sqlite::open(path).unwrap());
        let inspected = storage.clone();
        (storage, Box::new(move || inspected.counts().unwrap()))
    }
}
fn rows(storage: &dyn Storage, kinds: &[String]) -> Vec<Row> {
    kinds
        .iter()
        .flat_map(|kind| storage.snapshot(kind, 100, 1024 * 1024).unwrap())
        .collect()
}
async fn restart_preserves_deleted_seed(redb: bool) {
    let folder = std::env::temp_dir().join(format!(
        "rom-studio-seed-restart-{}-{redb}",
        std::process::id()
    ));
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("store");
    let (storage, counts) = open(&path, redb);
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    let actor = studio_application::host_actor();
    studio_startup::seed_all(&runtime, storage.as_ref(), "https://fixture.example")
        .await
        .unwrap();
    let kinds = runtime
        .discover(&actor)
        .await
        .unwrap()
        .resources
        .into_iter()
        .map(|r| r.kind)
        .collect::<Vec<_>>();
    // Respect the showcase's required relation before deleting its seeded target.
    runtime
        .execute(
            &actor,
            Command::patch(
                "workshop-sample",
                Patch::new().set(
                    FieldShowcase::task_field(),
                    ResourceRef::<Task>::new("task-b").unwrap(),
                ),
            )
            .at_revision(1)
            .idempotency("move-showcase-relation"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor,
            Command::replace(
                "task-a",
                Task {
                    title: "Preserve operator edit".into(),
                    done: true,
                },
            )
            .at_revision(1)
            .idempotency("edit-seeded-task"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &actor,
            Command::<Task>::delete("task-a")
                .at_revision(2)
                .idempotency("delete-seeded-task"),
        )
        .await
        .unwrap();
    let key = Key {
        kind: Task::KIND.into(),
        id: "task-a".into(),
    };
    let deleted = storage.load(&key).unwrap().unwrap();
    assert_eq!(deleted.revision, 3);
    assert!(deleted.value.is_none());
    let before = rows(storage.as_ref(), &kinds);
    let counts_before = counts();
    let heads = kinds
        .iter()
        .map(|kind| storage.journal_head(kind).unwrap())
        .collect::<Vec<_>>();
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    drop(counts);
    let (storage, counts) = open(&path, redb);
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    studio_startup::seed_all(&runtime, storage.as_ref(), "https://fixture.example")
        .await
        .expect("restart must preserve a deleted seed instead of disclosing its create receipt");
    // A second provisioning pass must also preserve receipts/events and edited relations.
    studio_startup::seed_all(&runtime, storage.as_ref(), "https://fixture.example")
        .await
        .unwrap();
    assert_eq!(counts(), counts_before);
    assert_eq!(storage.load(&key).unwrap(), Some(deleted));
    assert_eq!(rows(storage.as_ref(), &kinds), before);
    assert_eq!(
        kinds
            .iter()
            .map(|kind| storage.journal_head(kind).unwrap())
            .collect::<Vec<_>>(),
        heads
    );
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    drop(counts);
    std::fs::remove_dir_all(folder).unwrap();
}
#[tokio::test]
async fn sqlite_restart_preserves_deleted_seed() {
    restart_preserves_deleted_seed(false).await;
}
#[tokio::test]
async fn redb_restart_preserves_deleted_seed() {
    restart_preserves_deleted_seed(true).await;
}

#[tokio::test]
async fn closed_runtime_cannot_report_existing_seeds_ready() {
    let storage = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    studio_startup::seed_all(&runtime, storage.as_ref(), "https://fixture.example")
        .await
        .unwrap();
    let before = storage.counts().unwrap();
    runtime.shutdown().await.unwrap();
    assert_eq!(
        studio_startup::seed_all(&runtime, storage.as_ref(), "https://fixture.example").await,
        Err(rom::Error::Closed)
    );
    assert_eq!(storage.counts().unwrap(), before);
}

struct FailedLookup(Arc<dyn Storage>);
impl Storage for FailedLookup {
    fn capabilities(&self) -> rom::Capabilities {
        self.0.capabilities()
    }
    fn load(&self, _: &Key) -> rom::Result<Option<Row>> {
        Err(rom::Error::Storage)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> rom::Result<Vec<Row>> {
        self.0.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, identity: &str) -> rom::Result<Option<rom::Receipt>> {
        self.0.receipt(identity)
    }
    fn commit(&self, bundle: &rom::Bundle) -> rom::Result<rom::Receipt> {
        self.0.commit(bundle)
    }
}
#[tokio::test]
async fn failed_seed_lookup_does_not_create_or_replay_resources() {
    let storage = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    let before = storage.counts().unwrap();
    let failed = FailedLookup(storage.clone());
    assert_eq!(
        studio_startup::seed_all(&runtime, &failed, "https://fixture.example").await,
        Err(rom::Error::Storage)
    );
    assert_eq!(storage.counts().unwrap(), before);
    runtime.shutdown().await.unwrap();
}

async fn missing_showcase_preserves_task_deletions(redb: bool, all_deleted: bool) {
    let folder = std::env::temp_dir().join(format!(
        "rom-studio-showcase-upgrade-{}-{redb}-{all_deleted}",
        std::process::id()
    ));
    std::fs::create_dir(&folder).unwrap();
    let path = folder.join("store");
    let (storage, counts) = open(&path, redb);
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    let actor = studio_application::host_actor();
    for id in ["task-a", "task-b", "task-c"] {
        runtime
            .execute(
                &actor,
                Command::create(
                    id,
                    Task {
                        title: format!("Existing {id}"),
                        done: false,
                    },
                )
                .idempotency(&format!("studio-seed-tasks-{id}")),
            )
            .await
            .unwrap();
        if id == "task-a" || all_deleted {
            runtime
                .execute(
                    &actor,
                    Command::<Task>::delete(id)
                        .at_revision(1)
                        .idempotency(&format!("delete-{id}")),
                )
                .await
                .unwrap();
        }
    }
    let before = storage.snapshot(Task::KIND, 10, 10000).unwrap();
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    drop(counts);
    let (storage, counts) = open(&path, redb);
    let runtime = studio_application::build(storage.clone(), Arc::new(rom::SystemClock)).unwrap();
    studio_startup::seed_showcase(&runtime, storage.as_ref())
        .await
        .expect("new showcase must honor deleted Task prerequisites");
    let showcase = storage
        .load(&Key {
            kind: FieldShowcase::KIND.into(),
            id: "workshop-sample".into(),
        })
        .unwrap();
    if all_deleted {
        assert!(showcase.is_none());
    } else {
        assert_eq!(
            showcase.unwrap().value.unwrap()["task"],
            rom::Value::from("task-b")
        );
    }
    assert_eq!(storage.snapshot(Task::KIND, 10, 10000).unwrap(), before);
    let after = counts();
    studio_startup::seed_showcase(&runtime, storage.as_ref())
        .await
        .unwrap();
    assert_eq!(counts(), after);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    drop(storage);
    drop(counts);
    std::fs::remove_dir_all(folder).unwrap();
}
#[tokio::test]
async fn sqlite_new_showcase_uses_existing_live_task() {
    missing_showcase_preserves_task_deletions(false, false).await;
}
#[tokio::test]
async fn redb_new_showcase_uses_existing_live_task() {
    missing_showcase_preserves_task_deletions(true, false).await;
}
#[tokio::test]
async fn sqlite_new_showcase_is_omitted_when_tasks_are_deleted() {
    missing_showcase_preserves_task_deletions(false, true).await;
}
#[tokio::test]
async fn redb_new_showcase_is_omitted_when_tasks_are_deleted() {
    missing_showcase_preserves_task_deletions(true, true).await;
}
