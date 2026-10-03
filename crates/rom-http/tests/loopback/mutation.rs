use super::support::*;

#[tokio::test]
async fn embedded_and_wire_share_mutation_identity_authorization_and_projection() {
    let server = Server::start(Limits::default()).await;
    let actor = Actor::trusted("local", "alice");
    let embedded = server
        .runtime
        .invoke_projected(&actor, create())
        .await
        .unwrap();
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&create()).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 200);
    assert_eq!(body(&response), serde_json::to_value(embedded).unwrap());
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    let done: Invocation = Command::action("one", COMPLETE, ())
        .at_revision(1)
        .idempotency("done")
        .into();
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "other-secret",
                    &serde_json::to_string(&done).unwrap()
                )
                .await
        ),
        403
    );
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "expired-secret",
                    &serde_json::to_string(&done).unwrap()
                )
                .await
        ),
        403
    );
    let mut forged = serde_json::to_value(&done).unwrap();
    forged["actor"] = json!({"subject":"admin","authority":"local"});
    assert_eq!(
        status(
            &server
                .post("/invoke", "other-secret", &forged.to_string())
                .await
        ),
        400
    );
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "unverified",
                    &serde_json::to_string(&done).unwrap()
                )
                .await
        ),
        403
    );
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    &serde_json::to_string(&done).unwrap()
                )
                .await
        ),
        200
    );
    let mut conflict = done;
    conflict.operation = rom::Operation::Action {
        name: "complete".into(),
        input: json!(false),
    };
    assert_eq!(
        body(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    &serde_json::to_string(&conflict).unwrap()
                )
                .await
        )["error"],
        "identity_mismatch"
    );
    assert_eq!(server.store.counts().unwrap(), [1, 2, 2, 1]);
    server.finish().await;
}

#[tokio::test]
async fn disconnect_after_acceptance_retains_work_permit_and_retry_replays_one_commit() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .limits(rom::Limits {
            actions: 1,
            ..Default::default()
        })
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store.clone(), Limits::default()).await;
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let started = Arc::new(tokio::sync::Notify::new());
    let g = gate.clone();
    let signal = started.clone();
    store.on_commit(Some(Arc::new(move |point| {
        if point == 0 {
            signal.notify_one();
            let mut open = g.0.lock().unwrap();
            while !*open {
                open = g.1.wait(open).unwrap();
            }
        }
        Ok(())
    })));
    let socket = server
        .stream("/invoke", &serde_json::to_string(&create()).unwrap())
        .await;
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    drop(socket);
    assert_eq!(server.runtime.available_capacity(), 0);
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    &serde_json::to_string(&create()).unwrap()
                )
                .await
        ),
        429
    );
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    tokio::time::timeout(Duration::from_secs(2), async {
        while server.runtime.available_capacity() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    store.on_commit(None);
    let retry = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&create()).unwrap(),
        )
        .await;
    assert_eq!(status(&retry), 200);
    assert_eq!(body(&retry)["revision"], 1);
    assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    server.finish().await;
}

#[tokio::test]
async fn lost_commit_ack_is_unknown_on_wire_and_same_identity_resolves_it() {
    let server = Server::start(Limits::default()).await;
    server.store.inject_fault(5);
    let request = serde_json::to_string(&create()).unwrap();
    let uncertain = server.post("/invoke", "owner-secret", &request).await;
    assert_eq!(status(&uncertain), 503);
    assert_eq!(body(&uncertain)["error"], "outcome_unknown");
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", &request).await),
        200
    );
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    server.finish().await;
}
