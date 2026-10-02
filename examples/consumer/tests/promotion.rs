use rom::{Action, Actor, Command, Error, Intent, Resource, Result, Runtime};
use rom_consumer::{Task, task_policy};
use rom_sqlite::Sqlite;
use std::sync::Arc;
fn task() -> Task {
    Task {
        owner: "alice".into(),
        title: "title".into(),
        done: false,
        note: None,
    }
}
fn actor() -> Actor {
    Actor::trusted("local", "alice")
}
#[tokio::test]
async fn built_in_and_custom_operation_names_have_distinct_durable_identity() {
    fn fake_delete(t: &mut Task, _: ()) -> Result<Vec<Intent>> {
        t.done = true;
        Ok(vec![])
    }
    let custom = Action::new("delete", fake_delete);
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(
            Task::definition()
                .allow_all_fields()
                .policy(task_policy)
                .action(custom),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    rom.execute(
        &actor(),
        Command::action("t", custom, ())
            .at_revision(1)
            .idempotency("same"),
    )
    .await
    .unwrap();
    let deleted = rom
        .execute(
            &actor(),
            Command::<Task>::delete("t")
                .at_revision(1)
                .idempotency("same"),
        )
        .await;
    assert!(
        matches!(deleted, Err(Error::Conflict)),
        "custom delete must not be replayed as built-in delete: {deleted:?}"
    );
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn revoked_read_does_not_reveal_resource_existence() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(Task::definition().allow_all_fields().policy(task_policy))
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    rom.execute(&actor(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
    rom.revoke(&actor());
    for id in ["t", "missing"] {
        assert!(matches!(
            rom.read::<Task>(&actor(), id).await,
            Err(Error::Denied)
        ));
    }
    rom.shutdown().await.unwrap();
}
#[derive(Clone, Resource)]
#[resource(name = "nested")]
struct Nested {
    field: Option<Option<bool>>,
}
#[test]
fn unsupported_nested_nullable_is_rejected_instead_of_losing_presence() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    assert!(matches!(
        Runtime::builder()
            .resource(Nested::definition().allow_all_fields())
            .build(store, Runtime::shared_cpu_pool(1).unwrap()),
        Err(Error::Unsupported(_))
    ));
}
