use rom::{Access, Actor, Command, Error, Limits, Query, QuerySpec, Resource, Runtime};
use rom_consumer::{Setting, Task, setting_policy, task_policy};
use rom_sqlite::Sqlite;
use std::sync::Arc;

fn actor(subject: &str) -> Actor {
    Actor::trusted("local", subject)
}

async fn fixture(limits: Limits) -> Runtime {
    let runtime = Runtime::builder()
        .limits(limits)
        .resource(
            Task::definition()
                .policy(task_policy)
                .field_policy(|a, access, name, _| {
                    a.subject != "restricted" || matches!(access, Access::Write) || name != "note"
                }),
        )
        .resource(
            Setting::definition()
                .policy(setting_policy)
                .allow_all_fields(),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    for (id, owner) in [
        ("c", "alice"),
        ("a", "alice"),
        ("b", "bob"),
        ("d", "restricted"),
    ] {
        runtime
            .execute(
                &actor("admin"),
                Command::create(
                    id,
                    Task {
                        owner: owner.into(),
                        title: id.into(),
                        done: false,
                        note: None,
                    },
                )
                .idempotency(id),
            )
            .await
            .unwrap();
    }
    runtime
        .execute(
            &actor("admin"),
            Command::create(
                "config",
                Setting {
                    owner: "alice".into(),
                    enabled: false,
                    attempts: 0,
                },
            )
            .idempotency("setting"),
        )
        .await
        .unwrap();
    runtime
}

#[tokio::test]
async fn all_queries_preserve_row_fields_pages_and_live_authorization() {
    let runtime = fixture(Limits::default()).await;
    let query = Query::<Task>::all().limit(1);
    let first = runtime.query(&actor("alice"), &query).await.unwrap();
    assert_eq!(
        first.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["a"]
    );
    let page = query.after_id("a");
    let typed = runtime.query(&actor("alice"), &page).await.unwrap();
    let projected = runtime
        .query_spec_projected(
            &actor("alice"),
            "tasks",
            QuerySpec::all().limit(1).after_id("a"),
        )
        .await
        .unwrap();
    assert_eq!(typed[0].id, "c");
    assert_eq!(projected[0].key.id, "c");
    let mut live = runtime.live(&actor("alice"), page).await.unwrap();
    assert_eq!(live.changed().await.unwrap()[0].id, "c");
    assert_eq!(
        runtime
            .query(&actor("alice"), &Query::<Setting>::default())
            .await
            .unwrap()[0]
            .id,
        "config"
    );
    assert!(matches!(
        runtime
            .query(&actor("restricted"), &Query::<Task>::default())
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        runtime
            .live(&actor("restricted"), Query::<Task>::all())
            .await,
        Err(Error::Denied)
    ));
    // No predicate permission was granted for Task. Listing is still allowed;
    // adding a predicate must use the existing query-policy checks.
    assert!(matches!(
        runtime
            .query(
                &actor("alice"),
                &Query::<Task>::all().and(Task::done_field(), false)
            )
            .await,
        Err(Error::Denied)
    ));
    runtime.revoke(&actor("alice"));
    assert!(matches!(live.changed().await, Err(Error::Denied)));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn all_queries_do_not_turn_page_limits_into_scan_limits() {
    let runtime = fixture(Limits {
        snapshot_rows: 3,
        ..Limits::default()
    })
    .await;
    assert!(matches!(
        runtime
            .query(&actor("alice"), &Query::<Task>::all().limit(1))
            .await,
        Err(Error::TooLarge)
    ));
    runtime.shutdown().await.unwrap();
}
