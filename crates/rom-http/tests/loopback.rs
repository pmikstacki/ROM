use rom::{Actor, Command, Invocation, Runtime, json};
use rom_consumer::{COMPLETE, Task, declarations};
use rom_http::{Http, Limits};
use rom_sqlite::Sqlite;
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
struct SlowGate;
impl rom::ActorGate for SlowGate {
    fn check(&self, _: &Actor, _: &mut dyn rom::AuthorizationRead) -> rom::Result<()> {
        std::thread::sleep(Duration::from_millis(20));
        Ok(())
    }
}
async fn slow_observation_delivers(path: &str, payload: &str) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .actor_gate(Arc::new(SlowGate))
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let server = Server::with_runtime(
        runtime,
        store,
        Limits {
            observation_poll: Duration::from_millis(10),
            ..Default::default()
        },
    )
    .await;
    let mut socket = server.stream(path, payload).await;
    let response = until(&mut socket, "event: data").await;
    assert!(response.contains("keepalive"));
    assert!(!response.contains("overloaded"));
    drop(socket);
    server.finish().await;
}
#[tokio::test]
async fn live_keeps_one_pending_read_across_keepalives() {
    slow_observation_delivers("/live", r#"{"kind":"tasks","field":"done","value":false}"#).await;
}
#[tokio::test]
async fn journal_keeps_one_pending_read_across_keepalives() {
    slow_observation_delivers("/subscribe", r#"{"kind":"tasks","after":null}"#).await;
}
struct Server {
    address: std::net::SocketAddr,
    runtime: Runtime,
    store: Arc<Sqlite>,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<rom::Result<()>>,
}
fn resolver() -> rom_http::AuthResolver {
    Arc::new(
        |headers| match headers.get("authorization").and_then(|v| v.to_str().ok()) {
            Some("Bearer owner-secret") => Ok(Actor::trusted("local", "alice")),
            Some("Bearer other-secret") => Ok(Actor::trusted("local", "mallory")),
            Some("Bearer soon-secret") => Ok(Actor::trusted("local", "alice").expires_at(5)),
            Some("Bearer expired-secret") => Ok(Actor::trusted("local", "alice").expires_at(1)),
            _ => Err(rom::Error::Denied),
        },
    )
}
impl Server {
    async fn start(limits: Limits) -> Self {
        let store = Arc::new(Sqlite::open(":memory:").unwrap());
        let runtime = declarations()
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self::with_runtime(runtime, store, limits).await
    }
    async fn with_runtime(runtime: Runtime, store: Arc<Sqlite>, limits: Limits) -> Self {
        let http = Http::new(runtime.clone(), resolver(), limits).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, receiver) = oneshot::channel();
        let task = tokio::spawn(http.serve(listener, async move {
            let _ = receiver.await;
        }));
        Self {
            address,
            runtime,
            store,
            stop,
            task,
        }
    }
    async fn finish(self) {
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    async fn raw(&self, request: String) -> String {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(3), socket.read_to_string(&mut response))
            .await
            .unwrap()
            .unwrap();
        response
    }
    async fn post(&self, path: &str, token: &str, body: &str) -> String {
        self.raw(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nX-Subject: admin\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len())).await
    }
    async fn stream(&self, path: &str, body: &str) -> TcpStream {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        socket
    }
}
fn create() -> Invocation {
    Command::create(
        "one",
        Task {
            owner: "alice".into(),
            title: "visible".into(),
            done: false,
            note: None,
        },
    )
    .idempotency("create")
    .into()
}
fn status(response: &str) -> u16 {
    response.split_whitespace().nth(1).unwrap().parse().unwrap()
}
fn body(response: &str) -> serde_json::Value {
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
async fn until(socket: &mut TcpStream, needle: &str) -> String {
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut accumulated = String::new();
        let mut buffer = [0; 4096];
        while !accumulated.contains(needle) {
            let n = socket.read(&mut buffer).await.unwrap();
            assert!(n > 0, "closed before {needle}: {accumulated}");
            accumulated.push_str(std::str::from_utf8(&buffer[..n]).unwrap());
        }
        accumulated
    })
    .await
    .unwrap()
}
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
async fn strict_json_and_declared_and_chunked_body_bounds_reject_before_execution() {
    let server = Server::start(Limits {
        body_bytes: 512,
        ..Limits::default()
    })
    .await;
    let valid = serde_json::to_string(&create()).unwrap();
    let duplicate = valid.replace(
        "\"owner\":\"alice\"",
        "\"owner\":\"mallory\",\"owner\":\"alice\"",
    );
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", &duplicate).await),
        400
    );
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    "{\"kind\":\"tasks\",\"id\":\"one\",\"unexpected\":false}"
                )
                .await
        ),
        400
    );
    assert_eq!(
        status(
            &server
                .post("/invoke", "owner-secret", &" ".repeat(513))
                .await
        ),
        413
    );
    let huge = " ".repeat(513);
    let chunked = format!(
        "POST /invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
        huge.len(),
        huge
    );
    assert_eq!(status(&server.raw(chunked).await), 413);
    assert_eq!(server.store.counts().unwrap(), [0, 0, 0, 0]);
    server.finish().await;
}
#[tokio::test]
async fn slow_body_owns_independent_admission_until_timeout() {
    let server = Server::start(Limits {
        bodies: 1,
        body_timeout: Duration::from_millis(100),
        ..Limits::default()
    })
    .await;
    let mut slow = TcpStream::connect(server.address).await.unwrap();
    slow.write_all(b"POST /invoke HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nConnection: close\r\nContent-Length: 100\r\n\r\n{").await.unwrap();
    // Readiness is established by an actual second request receiving overload.
    let mut overloaded = false;
    for _ in 0..20 {
        if status(
            &server
                .post(
                    "/read",
                    "owner-secret",
                    "{\"kind\":\"tasks\",\"id\":\"one\"}",
                )
                .await,
        ) == 429
        {
            overloaded = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(overloaded);
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), slow.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status(&response), 429);
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
        200
    );
    server.finish().await;
}
#[tokio::test]
async fn live_is_fresh_coalescing_state_and_revocation_and_shutdown_close_streams() {
    let server = Server::start(Limits::default()).await;
    server
        .runtime
        .invoke(&Actor::trusted("local", "alice"), create())
        .await
        .unwrap();
    let mut socket = server
        .stream(
            "/live",
            "{\"kind\":\"tasks\",\"field\":\"done\",\"value\":false}",
        )
        .await;
    assert!(until(&mut socket, "visible").await.contains("200 OK"));
    server
        .runtime
        .execute(
            &Actor::trusted("local", "alice"),
            Command::action("one", COMPLETE, ())
                .at_revision(1)
                .idempotency("done"),
        )
        .await
        .unwrap();
    until(&mut socket, "data: []").await;
    server.runtime.revoke(&Actor::trusted("local", "alice"));
    until(&mut socket, "denied").await;
    drop(socket);
    server.finish().await;
    let server = Server::start(Limits::default()).await;
    let mut idle = server
        .stream(
            "/live",
            "{\"kind\":\"tasks\",\"field\":\"done\",\"value\":false}",
        )
        .await;
    until(&mut idle, "data: []").await;
    server.finish().await;
}

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
async fn idle_stream_expiry_rechecks_without_a_resource_write() {
    struct Clock(std::sync::atomic::AtomicU64);
    impl rom::Clock for Clock {
        fn now(&self) -> u64 {
            self.0.load(std::sync::atomic::Ordering::SeqCst)
        }
    }
    let clock = Arc::new(Clock(std::sync::atomic::AtomicU64::new(0)));
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .clock(clock.clone())
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(
        runtime,
        store,
        Limits {
            observation_poll: Duration::from_millis(10),
            ..Default::default()
        },
    )
    .await;
    let mut socket = TcpStream::connect(server.address).await.unwrap();
    let payload = "{\"kind\":\"tasks\",\"field\":\"done\",\"value\":false}";
    socket.write_all(format!("POST /live HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer soon-secret\r\nContent-Length: {}\r\n\r\n{payload}",payload.len()).as_bytes()).await.unwrap();
    until(&mut socket, "data: []").await;
    clock.0.store(5, std::sync::atomic::Ordering::SeqCst);
    until(&mut socket, "denied").await;
    drop(socket);
    server.finish().await;
}
#[tokio::test]
async fn stream_capacity_is_bounded_and_disconnect_releases_the_subscription() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .limits(rom::Limits {
            subscriptions: 1,
            ..Default::default()
        })
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(
        runtime,
        store,
        Limits {
            observation_poll: Duration::from_millis(10),
            ..Default::default()
        },
    )
    .await;
    let payload = "{\"kind\":\"tasks\",\"field\":\"done\",\"value\":false}";
    let mut first = server.stream("/live", payload).await;
    until(&mut first, "data: []").await;
    assert_eq!(
        status(&server.post("/live", "owner-secret", payload).await),
        429
    );
    drop(first);
    // Establish release by core admission; avoid a race-dependent fixed sleep.
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match server
                .runtime
                .live_projected(
                    &Actor::trusted("local", "alice"),
                    "tasks",
                    "done",
                    json!(false),
                )
                .await
            {
                Ok(handle) => {
                    drop(handle);
                    break;
                }
                Err(rom::Error::Overloaded) => tokio::task::yield_now().await,
                Err(e) => panic!("{e}"),
            }
        }
    })
    .await
    .unwrap();
    let mut next = server.stream("/live", payload).await;
    until(&mut next, "data: []").await;
    drop(next);
    server.finish().await;
}

#[tokio::test]
async fn all_wire_observation_forms_apply_field_projection_and_predicate_authority() {
    use rom::Resource;
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .policy(rom_consumer::task_policy)
                .field_policy(|_, access, name, _| {
                    matches!(access, rom::Access::Write) || name != "note"
                })
                .query_policy(|_, name| name != "note"),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let mut command = create();
    if let rom::Operation::Create(ref mut value) = command.operation {
        value["note"] = json!("SECRET-FIELD");
    }
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&command).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 200);
    assert!(!response.contains("SECRET-FIELD"));
    assert!(body(&response)["value"].get("note").is_none());
    for (route, payload) in [
        ("/read", json!({"kind":"tasks","id":"one"})),
        (
            "/query",
            json!({"kind":"tasks","field":"done","value":false}),
        ),
        ("/journal", json!({"kind":"tasks","after":null})),
    ] {
        let response = server
            .post(route, "owner-secret", &payload.to_string())
            .await;
        assert_eq!(status(&response), 200);
        assert!(!response.contains("SECRET-FIELD"));
        assert!(!response.contains("note"));
    }
    let response = server
        .post(
            "/query",
            "owner-secret",
            &json!({"kind":"tasks","field":"note","value":"SECRET-FIELD"}).to_string(),
        )
        .await;
    assert_eq!(status(&response), 403);
    assert!(!response.contains("SECRET-FIELD"));
    let mut live = server
        .stream(
            "/live",
            &json!({"kind":"tasks","field":"done","value":false}).to_string(),
        )
        .await;
    let response = until(&mut live, "visible").await;
    assert!(!response.contains("SECRET-FIELD"));
    drop(live);
    server.finish().await;
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
#[tokio::test]
async fn structured_queries_and_patches_use_the_generic_wire_contract() {
    let server = Server::start(Limits::default()).await;
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
        200
    );
    let query =
        r#"{"kind":"tasks","query":{"filters":[{"field":"done","value":false}],"limit":1}}"#;
    let response = server.post("/query", "owner-secret", query).await;
    assert_eq!(status(&response), 200);
    assert_eq!(body(&response).as_array().unwrap().len(), 1);
    let patch = r#"{"kind":"tasks","id":"one","expected":1,"idempotency":"patch","operation":{"type":"patch","input":{"done":{"op":"set","value":true}}}}"#;
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", patch).await),
        200
    );
    assert_eq!(
        body(&server.post("/query", "owner-secret", query).await)
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let mut stream = server.stream("/live", query).await;
    assert!(until(&mut stream, "event: data").await.contains("data: []"));
    drop(stream);
    let ambiguous = r#"{"kind":"tasks","field":"done","value":true,"query":{"filters":[]}}"#;
    assert_eq!(
        status(&server.post("/query", "owner-secret", ambiguous).await),
        400
    );
    server.finish().await;
}

#[tokio::test]
async fn discovery_route_uses_explicit_grants_and_strict_authenticated_input() {
    use rom::{DiscoveryTarget, Resource};
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .action(COMPLETE)
                .discovery_policy(|actor, target| {
                    actor.subject == "alice"
                        && match target {
                            DiscoveryTarget::Resource => true,
                            DiscoveryTarget::Field(name) => name == "done",
                            DiscoveryTarget::Action(name) => name == "complete",
                        }
                }),
        )
        .resource(rom_consumer::Setting::definition())
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(
        runtime,
        store,
        Limits {
            body_bytes: 128,
            ..Limits::default()
        },
    )
    .await;
    let response = server.post("/discover", "owner-secret", "{}").await;
    assert_eq!(status(&response), 200);
    assert_eq!(
        body(&response),
        json!({"version":1,"resources":[{
            "kind":"tasks","version":1,"fields":[{"name":"done","shape":{"type":"bool"}}],"actions":["complete"]
        }]})
    );
    assert_eq!(server.store.counts().unwrap(), [0; 4]);
    let hidden = server.post("/discover", "other-secret", "{}").await;
    assert_eq!(status(&hidden), 200);
    assert_eq!(body(&hidden), json!({"version":1,"resources":[]}));
    for token in ["invalid-secret", "expired-secret"] {
        let denied = server.post("/discover", token, "{}").await;
        assert_eq!(status(&denied), 403);
        assert_eq!(body(&denied), json!({"error":"denied"}));
    }
    for input in ["[]", "null", r#"{"extra":true}"#, r#"{"x":1,"x":2}"#] {
        let invalid = server.post("/discover", "owner-secret", input).await;
        assert_eq!(status(&invalid), 400, "{input}");
    }
    assert_eq!(
        status(
            &server
                .post("/discover", "owner-secret", &" ".repeat(129))
                .await
        ),
        413
    );
    server.runtime.revoke(&Actor::trusted("local", "alice"));
    assert_eq!(
        status(&server.post("/discover", "owner-secret", "{}").await),
        403
    );
    server.finish().await;
}

#[tokio::test]
async fn discovery_http_never_returns_an_oversized_partial_catalog() {
    use rom::Resource;
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .limits(rom::Limits {
            snapshot_bytes: 1,
            ..rom::Limits::default()
        })
        .resource(Task::definition().discovery_policy(|_, _| true))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let response = server.post("/discover", "owner-secret", "{}").await;
    assert_eq!(status(&response), 413);
    assert_eq!(body(&response), json!({"error":"too_large"}));
    server.finish().await;
}
