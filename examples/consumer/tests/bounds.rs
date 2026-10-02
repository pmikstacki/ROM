use rom::{Actor, Command, Error, Limits, Resource, Runtime, Storage};
use rom_consumer::{Task, task_policy};
use rom_sqlite::Sqlite;
use std::sync::Arc;
fn actor() -> Actor {
    Actor::trusted("local", "alice")
}
fn task() -> Task {
    Task {
        owner: "alice".into(),
        title: "one".into(),
        done: false,
        note: None,
    }
}
fn setup(limits: Limits) -> (Runtime, Arc<Sqlite>) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .limits(limits)
        .resource(Task::definition().allow_all_fields().policy(task_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    (runtime, store)
}
#[tokio::test]
async fn snapshot_limits_reject_before_return() {
    let (rom, store) = setup(Limits {
        snapshot_rows: 1,
        ..Limits::default()
    });
    rom.execute(&actor(), Command::create("a", task()).idempotency("a"))
        .await
        .unwrap();
    let rows = store.snapshot("tasks", 1, usize::MAX).unwrap();
    let bytes = rom::json!(rows[0]).to_string().len();
    assert_eq!(store.snapshot("tasks", 1, bytes).unwrap().len(), 1);
    assert!(matches!(
        store.snapshot("tasks", 1, bytes - 1),
        Err(Error::TooLarge)
    ));
    assert_eq!(
        rom.query(&actor(), &Task::done_field().equals(false))
            .await
            .unwrap()
            .len(),
        1
    );
    rom.execute(&actor(), Command::create("b", task()).idempotency("b"))
        .await
        .unwrap();
    assert!(matches!(
        store.snapshot("tasks", 1, usize::MAX),
        Err(Error::TooLarge)
    ));
    assert!(matches!(
        rom.query(&actor(), &Task::done_field().equals(false)).await,
        Err(Error::TooLarge)
    ));
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn subscription_capacity_released_on_drop() {
    let (rom, _) = setup(Limits {
        subscriptions: 1,
        ..Limits::default()
    });
    let live = rom
        .live(&actor(), Task::done_field().equals(false))
        .await
        .unwrap();
    assert!(matches!(
        rom.live(&actor(), Task::done_field().equals(false)).await,
        Err(Error::Overloaded)
    ));
    drop(live);
    let mut live = rom
        .live(&actor(), Task::done_field().equals(false))
        .await
        .unwrap();
    assert!(live.changed().await.unwrap().is_empty());
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn live_closes_on_shutdown() {
    let (rom, _) = setup(Limits::default());
    let mut live = rom
        .live(&actor(), Task::done_field().equals(false))
        .await
        .unwrap();
    live.changed().await.unwrap();
    rom.shutdown().await.unwrap();
    assert!(matches!(live.changed().await, Err(Error::Closed)));
}
#[tokio::test]
async fn zero_and_payload_limits_are_explicit() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    assert!(matches!(
        Runtime::builder()
            .limits(Limits {
                io_jobs: 0,
                ..Limits::default()
            })
            .build(store, Runtime::shared_cpu_pool(1).unwrap()),
        Err(Error::Unsupported(_))
    ));
    let (rom, store) = setup(Limits {
        command_bytes: 1,
        ..Limits::default()
    });
    assert!(matches!(
        rom.execute(&actor(), Command::create("a", task()).idempotency("a"))
            .await,
        Err(Error::TooLarge)
    ));
    assert_eq!(store.counts().unwrap(), [0; 4]);
    rom.shutdown().await.unwrap();
}
