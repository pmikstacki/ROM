use rom::{Actor, Command, Error, Invocation, Operation, Resource, Runtime, json};
use rom_consumer::{COMPLETE, Task, declarations};
use rom_sqlite::Sqlite;
use std::sync::Arc;
#[tokio::test]
async fn typed_and_erased_invocation_share_identity_policy_and_custom_actions() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let actor = Actor::trusted("local", "alice");
    let task = Task {
        owner: "alice".into(),
        title: "one".into(),
        done: false,
        note: None,
    };
    let command = Command::create("one", task.clone()).idempotency("create");
    let typed = runtime.execute(&actor, command.clone()).await.unwrap();
    let erased = runtime.invoke(&actor, command.into()).await.unwrap();
    assert_eq!(typed.revision, erased.revision);
    assert_eq!(erased.value, Some(task.encode()));
    assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    let forged = Actor::trusted("local", "mallory");
    assert_eq!(
        runtime
            .invoke(
                &forged,
                Command::action("one", COMPLETE, ())
                    .at_revision(1)
                    .idempotency("done")
                    .into()
            )
            .await,
        Err(Error::Denied)
    );
    let custom: Invocation = Command::action("one", COMPLETE, ())
        .at_revision(1)
        .idempotency("done")
        .into();
    assert_eq!(
        runtime
            .invoke(&actor, custom.clone())
            .await
            .unwrap()
            .revision,
        2
    );
    let mut changed = custom;
    changed.operation = Operation::Action {
        name: "complete".into(),
        input: json!(false),
    };
    assert_eq!(
        runtime.invoke(&actor, changed).await,
        Err(Error::IdentityMismatch)
    );
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn routing_and_host_identity_are_bounded_before_identity_encoding() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("local", "alice");
    let base = Invocation {
        retry_epoch: 0,
        kind: Task::KIND.into(),
        id: "x".into(),
        expected: None,
        idempotency: "k".into(),
        operation: Operation::Delete,
    };
    for invocation in [
        Invocation {
            retry_epoch: 0,
            kind: "x".repeat(20_000),
            ..base.clone()
        },
        Invocation {
            operation: Operation::Action {
                name: "x".repeat(20_000),
                input: json!(null),
            },
            ..base.clone()
        },
    ] {
        assert_eq!(
            runtime.invoke(&actor, invocation).await,
            Err(Error::TooLarge)
        );
    }
    assert_eq!(
        runtime
            .invoke(&Actor::trusted(&"x".repeat(20_000), "alice"), base)
            .await,
        Err(Error::TooLarge)
    );
    assert_eq!(store.counts().unwrap(), [0, 0, 0, 0]);
    runtime.shutdown().await.unwrap();
}
