use rom::{Actor, Command, Error, Patch, Presence, Query, Runtime};
use rom_skill_fixtures::{COMPLETE, Counter, Task, Title, definitions};
use rom_sqlite::Sqlite;
use std::sync::Arc;
fn actor(subject: &str) -> Actor {
    Actor::trusted("fixture", subject)
}
fn runtime() -> Runtime {
    definitions()
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap()
}
fn task(title: &str) -> Task {
    Task {
        title: Title(title.into()),
        done: false,
        memo: Presence::Missing,
    }
}
#[tokio::test]
async fn author_template_handles_two_kinds_canonical_queries_live_patch_and_replay() {
    let runtime = runtime();
    let owner = actor("owner");
    runtime
        .execute(
            &owner,
            Command::create("one", task("  Hello  ")).idempotency("create"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &owner,
            Command::create("count", Counter { count: 0 }).idempotency("count"),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime
            .query(&owner, &Task::title_field().equals(Title(" Hello ".into())))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        runtime
            .query(&owner, &Query::<Counter>::all())
            .await
            .unwrap()[0]
            .value
            .as_ref()
            .unwrap()
            .count,
        0
    );
    let mut live = runtime.live(&owner, Query::<Task>::all()).await.unwrap();
    assert_eq!(
        live.changed().await.unwrap()[0]
            .value
            .as_ref()
            .unwrap()
            .title
            .0,
        "Hello"
    );
    let patch = || {
        Command::patch(
            "one",
            Patch::new().set(Task::memo_field(), Presence::Value(None)),
        )
        .at_revision(1)
        .idempotency("memo")
    };
    assert_eq!(runtime.execute(&owner, patch()).await.unwrap().revision, 2);
    assert_eq!(runtime.execute(&owner, patch()).await.unwrap().revision, 2);
    assert_eq!(
        live.changed().await.unwrap()[0]
            .value
            .as_ref()
            .unwrap()
            .memo,
        Presence::Value(None)
    );
    runtime
        .execute(
            &owner,
            Command::action("one", COMPLETE, ())
                .at_revision(2)
                .idempotency("complete"),
        )
        .await
        .unwrap();
    assert!(
        runtime
            .read::<Task>(&owner, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .done
    );
    assert_eq!(
        runtime
            .journal(&owner, "tasks", None)
            .await
            .unwrap()
            .events
            .len(),
        3
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn invalid_custom_value_changes_no_state_or_success_event() {
    let runtime = runtime();
    let owner = actor("owner");
    assert!(matches!(
        runtime
            .execute(&owner, Command::create("bad", task(" ")).idempotency("bad"))
            .await,
        Err(Error::Invalid { .. })
    ));
    assert!(
        runtime
            .query(&owner, &Query::<Task>::all())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        runtime
            .journal(&owner, "tasks", None)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn denied_actor_and_stale_revision_cannot_change_the_row() {
    let runtime = runtime();
    let owner = actor("owner");
    runtime
        .execute(
            &owner,
            Command::create("one", task("Hello")).idempotency("create"),
        )
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .execute(
                &actor("intruder"),
                Command::action("one", COMPLETE, ())
                    .at_revision(1)
                    .idempotency("denied")
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        runtime
            .execute(
                &owner,
                Command::action("one", COMPLETE, ())
                    .at_revision(0)
                    .idempotency("stale")
            )
            .await,
        Err(Error::Conflict)
    ));
    let row = runtime.read::<Task>(&owner, "one").await.unwrap();
    assert_eq!(row.revision, 1);
    assert!(!row.value.unwrap().done);
    assert_eq!(
        runtime
            .journal(&owner, "tasks", None)
            .await
            .unwrap()
            .events
            .len(),
        1
    );
    runtime.shutdown().await.unwrap();
}
