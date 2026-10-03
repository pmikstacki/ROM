//! The additive async resolver uses actual TCP and the existing generic decoder.
use super::*;
use rom_http::{AsyncAuthResolver, AuthFuture};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Notify;
fn immediate() -> AsyncAuthResolver {
    let sync = resolver();
    Arc::new(move |headers| {
        let result = sync(&headers);
        Box::pin(async move { result }) as AuthFuture
    })
}
async fn start(auth: AsyncAuthResolver, limits: Limits) -> (Server, Http) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = declarations()
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let http = Http::new_async(runtime.clone(), auth, limits).unwrap();
    (Server::with_http(runtime, store, http.clone()).await, http)
}
struct Paused {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    calls: Arc<AtomicUsize>,
}
impl Paused {
    fn new() -> Self {
        Self {
            entered: Arc::new(Notify::new()),
            release: Arc::new(Notify::new()),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn resolver(&self) -> AsyncAuthResolver {
        let entered = self.entered.clone();
        let release = self.release.clone();
        let calls = self.calls.clone();
        let sync = resolver();
        Arc::new(move |headers| {
            let entered = entered.clone();
            let release = release.clone();
            let calls = calls.clone();
            let sync = sync.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                if headers
                    .get("authorization")
                    .is_some_and(|value| value == "Bearer delayed-secret")
                {
                    entered.notify_one();
                    release.notified().await;
                    // The owned headers remain usable after the async wait.
                    assert_eq!(headers["authorization"], "Bearer delayed-secret");
                    Ok(Actor::trusted("local", "alice"))
                } else {
                    sync(&headers)
                }
            })
        })
    }
    async fn entered(&self) {
        tokio::time::timeout(Duration::from_secs(2), self.entered.notified())
            .await
            .unwrap();
    }
    fn finish(&self) {
        self.release.notify_one();
    }
}
async fn headers(server: &Server, path: &str, token: &str, length: usize) -> TcpStream {
    let mut socket = TcpStream::connect(server.address).await.unwrap();
    socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nX-Subject: admin\r\nConnection: close\r\nContent-Length: {length}\r\n\r\n").as_bytes()).await.unwrap();
    socket
}
async fn pending(server: &Server, path: &str, token: &str, body: &str) -> TcpStream {
    let mut socket = headers(server, path, token, body.len()).await;
    socket.write_all(body.as_bytes()).await.unwrap();
    socket
}
async fn reply(socket: &mut TcpStream) -> String {
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), socket.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}
#[tokio::test]
async fn sync_and_async_authentication_share_resource_and_body_contracts() {
    for asynchronous in [false, true] {
        let limits = Limits {
            body_bytes: 1024,
            ..Default::default()
        };
        let server = if asynchronous {
            start(immediate(), limits).await.0
        } else {
            Server::start(limits).await
        };
        let response = server
            .post(
                "/invoke",
                "owner-secret",
                &serde_json::to_string(&create()).unwrap(),
            )
            .await;
        assert_eq!(status(&response), 200);
        assert_eq!(body(&response)["value"]["owner"], "alice");
        assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
        assert_eq!(
            status(
                &server
                    .post("/read", "other-secret", r#"{"kind":"tasks","id":"one"}"#)
                    .await
            ),
            403
        );
        assert_eq!(
            status(&server.post("/read", "owner-secret", "not-json").await),
            400
        );
        assert_eq!(
            status(
                &server
                    .post("/read", "owner-secret", &" ".repeat(1025))
                    .await
            ),
            413
        );
        assert_eq!(
            status(&server.post("/read", "invalid-secret", "not-json").await),
            403
        );
        assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
        server.finish().await;
    }
}
#[tokio::test]
async fn delayed_async_authentication_allows_unrelated_tcp_request_to_commit() {
    let paused = Paused::new();
    let (server, _) = start(paused.resolver(), Limits::default()).await;
    let mut waiting = pending(
        &server,
        "/read",
        "delayed-secret",
        r#"{"kind":"tasks","id":"one"}"#,
    )
    .await;
    paused.entered().await;
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&create()).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 200);
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    paused.finish();
    assert_eq!(status(&reply(&mut waiting).await), 200);
    server.finish().await;
}
#[tokio::test]
async fn denied_async_authentication_responds_without_waiting_for_streamed_body() {
    let (server, _) = start(
        immediate(),
        Limits {
            body_bytes: 64,
            body_timeout: Duration::from_secs(30),
            ..Default::default()
        },
    )
    .await;
    let mut socket = headers(&server, "/invoke", "invalid-secret", 10_000).await;
    let response = reply(&mut socket).await;
    assert_eq!(status(&response), 403);
    assert_eq!(body(&response), json!({"error":"denied"}));
    assert_eq!(server.store.counts().unwrap(), [0; 4]);
    server.finish().await;
}
#[tokio::test]
async fn body_admission_rejects_extra_request_before_async_authentication() {
    let paused = Paused::new();
    let (server, _) = start(
        paused.resolver(),
        Limits {
            bodies: 1,
            ..Default::default()
        },
    )
    .await;
    let mut waiting = pending(
        &server,
        "/invoke",
        "delayed-secret",
        &serde_json::to_string(&create()).unwrap(),
    )
    .await;
    paused.entered().await;
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&create()).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 429);
    assert_eq!(paused.calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.store.counts().unwrap(), [0; 4]);
    paused.finish();
    assert_eq!(status(&reply(&mut waiting).await), 200);
    assert_eq!(server.store.counts().unwrap(), [1, 1, 1, 0]);
    server.finish().await;
}
#[tokio::test]
async fn shutdown_closes_admission_and_pending_auth_cannot_bypass_runtime_closure() {
    let paused = Paused::new();
    let (server, http) = start(paused.resolver(), Limits::default()).await;
    let mut waiting = pending(
        &server,
        "/invoke",
        "delayed-secret",
        &serde_json::to_string(&create()).unwrap(),
    )
    .await;
    paused.entered().await;
    http.shutdown().await.unwrap();
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&create()).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 503);
    assert_eq!(body(&response), json!({"error":"closed"}));
    assert_eq!(paused.calls.load(Ordering::SeqCst), 1);
    paused.finish();
    let response = reply(&mut waiting).await;
    assert_eq!(status(&response), 503);
    assert_eq!(body(&response), json!({"error":"closed"}));
    assert_eq!(server.store.counts().unwrap(), [0; 4]);
    server.finish().await;
}
#[test]
fn synchronous_and_asynchronous_constructors_share_limit_validation() {
    for limits in [
        Limits {
            bodies: 0,
            ..Default::default()
        },
        Limits {
            body_bytes: 0,
            ..Default::default()
        },
        Limits {
            body_timeout: Duration::ZERO,
            ..Default::default()
        },
        Limits {
            observation_poll: Duration::ZERO,
            ..Default::default()
        },
    ] {
        let store = Arc::new(Sqlite::open(":memory:").unwrap());
        let runtime = declarations()
            .build(store, Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
        assert!(matches!(
            Http::new(runtime.clone(), resolver(), limits),
            Err(rom::Error::TooLarge)
        ));
        assert!(matches!(
            Http::new_async(runtime, immediate(), limits),
            Err(rom::Error::TooLarge)
        ));
    }
}
