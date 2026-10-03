use super::support::*;

#[tokio::test]
async fn journal_replays_distinct_facts_and_rejects_wrong_generation_scope_and_future_cursor() {
    let server = Server::start(Limits::default()).await;
    let actor = Actor::trusted("local", "alice");
    server.runtime.invoke(&actor, create()).await.unwrap();
    let initial = server
        .post(
            "/journal",
            "owner-secret",
            "{\"kind\":\"tasks\",\"after\":null}",
        )
        .await;
    assert_eq!(status(&initial), 200);
    let page = body(&initial);
    assert_eq!(page["events"].as_array().unwrap().len(), 1);
    assert!(page["events"][0].get("identity").is_none());
    server
        .runtime
        .execute(
            &actor,
            Command::action("one", COMPLETE, ())
                .at_revision(1)
                .idempotency("done"),
        )
        .await
        .unwrap();
    let resumed = server
        .post(
            "/journal",
            "owner-secret",
            &json!({"kind":"tasks","after":page["cursor"]}).to_string(),
        )
        .await;
    assert_eq!(body(&resumed)["events"].as_array().unwrap().len(), 1);
    assert_eq!(body(&resumed)["events"][0]["view"]["revision"], 2);
    let all = server
        .post(
            "/journal",
            "owner-secret",
            "{\"kind\":\"tasks\",\"after\":null}",
        )
        .await;
    assert_eq!(body(&all)["events"].as_array().unwrap().len(), 2);
    for field in ["generation", "kind", "position"] {
        let mut cursor = page["cursor"].clone();
        cursor[field] = if field == "position" {
            json!(100)
        } else {
            json!("wrong")
        };
        let response = server
            .post(
                "/journal",
                "owner-secret",
                &json!({"kind":"tasks","after":cursor}).to_string(),
            )
            .await;
        assert_eq!(status(&response), 410);
        assert_eq!(body(&response)["error"], "history_gap");
    }
    let hidden = server
        .post(
            "/journal",
            "other-secret",
            "{\"kind\":\"tasks\",\"after\":null}",
        )
        .await;
    assert_eq!(body(&hidden)["events"], json!([]));
    assert!(!hidden.contains("visible"));
    let mut subscription = server
        .stream(
            "/subscribe",
            &json!({"kind":"tasks","after":body(&resumed)["cursor"]}).to_string(),
        )
        .await;
    until(&mut subscription, "\"events\":[]").await;
    server.runtime.revoke(&actor);
    until(&mut subscription, "denied").await;
    drop(subscription);
    server.finish().await;
}

#[tokio::test]
async fn retention_loss_is_explicit_for_both_missing_and_expired_cursor() {
    let store = Arc::new(
        Sqlite::open_with_limits(
            ":memory:",
            rom::StorageLimits {
                journal_rows: 1,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let runtime = declarations()
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("local", "alice");
    runtime.invoke(&actor, create()).await.unwrap();
    let before = runtime.journal(&actor, "tasks", None).await.unwrap().cursor;
    runtime
        .execute(
            &actor,
            Command::action("one", COMPLETE, ())
                .at_revision(1)
                .idempotency("done"),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime.journal(&actor, "tasks", None).await,
        Err(rom::Error::HistoryGap)
    );
    assert_eq!(
        runtime
            .journal(
                &actor,
                "tasks",
                Some(&rom::JournalCursor {
                    position: 0,
                    ..before.clone()
                })
            )
            .await,
        Err(rom::Error::HistoryGap)
    );
    assert_eq!(
        runtime
            .journal(&actor, "tasks", Some(&before))
            .await
            .unwrap()
            .events
            .len(),
        1
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn journal_pages_are_bounded_and_cancelled_wait_does_not_acknowledge_a_fact() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .limits(rom::Limits {
            snapshot_rows: 1,
            ..Default::default()
        })
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("local", "alice");
    runtime.invoke(&actor, create()).await.unwrap();
    let mut handle = runtime.subscribe(&actor, "tasks", None).await.unwrap();
    assert_eq!(handle.next().await.unwrap().events[0].view.revision, 1);
    assert!(
        tokio::time::timeout(Duration::from_millis(5), handle.next())
            .await
            .is_err()
    );
    runtime
        .execute(
            &actor,
            Command::action("one", COMPLETE, ())
                .at_revision(1)
                .idempotency("done"),
        )
        .await
        .unwrap();
    assert_eq!(handle.next().await.unwrap().events[0].view.revision, 2);
    let first = runtime.journal(&actor, "tasks", None).await.unwrap();
    assert_eq!(first.events.len(), 1);
    let second = runtime
        .journal(&actor, "tasks", Some(&first.cursor))
        .await
        .unwrap();
    assert_eq!(second.events.len(), 1);
    assert_eq!(second.events[0].view.revision, 2);
    runtime.shutdown().await.unwrap();
    assert_eq!(handle.next().await, Err(rom::Error::Closed));
}

#[tokio::test]
async fn expired_history_requires_explicit_head_then_snapshot_recovery() {
    let store = Arc::new(
        Sqlite::open_with_limits(
            ":memory:",
            rom::StorageLimits {
                journal_rows: 1,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let runtime = declarations()
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let actor = Actor::trusted("local", "alice");
    server.runtime.invoke(&actor, create()).await.unwrap();
    server
        .runtime
        .execute(
            &actor,
            Command::action("one", COMPLETE, ())
                .at_revision(1)
                .idempotency("done"),
        )
        .await
        .unwrap();
    assert_eq!(
        status(
            &server
                .post(
                    "/journal",
                    "owner-secret",
                    "{\"kind\":\"tasks\",\"after\":null}"
                )
                .await
        ),
        410
    );
    let head = server
        .post("/journal/head", "owner-secret", "{\"kind\":\"tasks\"}")
        .await;
    assert_eq!(status(&head), 200);
    assert_eq!(body(&head)["position"], 2);
    assert_eq!(
        status(
            &server
                .post("/journal/head", "expired-secret", "{\"kind\":\"tasks\"}")
                .await
        ),
        403
    );
    let snapshot = server
        .post(
            "/query",
            "owner-secret",
            "{\"kind\":\"tasks\",\"field\":\"done\",\"value\":true}",
        )
        .await;
    assert_eq!(body(&snapshot)[0]["revision"], 2);
    let resumed = server
        .post(
            "/journal",
            "owner-secret",
            &json!({"kind":"tasks","after":body(&head)}).to_string(),
        )
        .await;
    assert_eq!(status(&resumed), 200);
    assert_eq!(body(&resumed)["events"], json!([]));
    server.finish().await;
}
