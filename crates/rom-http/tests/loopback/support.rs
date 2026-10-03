//! Shared bounded HTTP listener, authentication and socket fixtures.
pub(super) use rom::{Actor, Command, Invocation, Runtime, json};
pub(super) use rom_consumer::{COMPLETE, Task, declarations};
pub(super) use rom_http::{Http, Limits};
pub(super) use rom_sqlite::Sqlite;
pub(super) use std::{sync::Arc, time::Duration};
pub(super) use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
pub(super) struct Server {
    pub(super) address: std::net::SocketAddr,
    pub(super) runtime: Runtime,
    pub(super) store: Arc<Sqlite>,
    pub(super) stop: oneshot::Sender<()>,
    pub(super) task: tokio::task::JoinHandle<rom::Result<()>>,
}
pub(super) fn resolver() -> rom_http::AuthResolver {
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
    pub(super) async fn start(limits: Limits) -> Self {
        let store = Arc::new(Sqlite::open(":memory:").unwrap());
        let runtime = declarations()
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self::with_runtime(runtime, store, limits).await
    }
    pub(super) async fn with_runtime(runtime: Runtime, store: Arc<Sqlite>, limits: Limits) -> Self {
        let http = Http::new(runtime.clone(), resolver(), limits).unwrap();
        Self::with_http(runtime, store, http).await
    }
    pub(super) async fn with_http(runtime: Runtime, store: Arc<Sqlite>, http: Http) -> Self {
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
    pub(super) async fn finish(self) {
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    pub(super) async fn raw(&self, request: String) -> String {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(3), socket.read_to_string(&mut response))
            .await
            .unwrap()
            .unwrap();
        response
    }
    pub(super) async fn post(&self, path: &str, token: &str, body: &str) -> String {
        self.raw(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nX-Subject: admin\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len())).await
    }
    pub(super) async fn stream(&self, path: &str, body: &str) -> TcpStream {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer owner-secret\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        socket
    }
}
pub(super) fn create() -> Invocation {
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
pub(super) fn status(response: &str) -> u16 {
    response.split_whitespace().nth(1).unwrap().parse().unwrap()
}
pub(super) fn body(response: &str) -> serde_json::Value {
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}
pub(super) async fn until(socket: &mut TcpStream, needle: &str) -> String {
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
