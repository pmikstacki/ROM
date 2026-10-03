use super::support::*;

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
