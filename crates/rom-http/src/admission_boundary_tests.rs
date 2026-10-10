//! Characterize HTTP admission and body timeout; neither result identifies a core overload.
use crate::authentication::Resolver;
use crate::server::Shared;
use crate::{AsyncAuthResolver, AuthFuture, Http, Limits};
use axum::{body::Body, extract::Request, http::StatusCode, response::IntoResponse};
use rom::{Actor, Runtime};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc, oneshot, watch};

const BOUND: Duration = Duration::from_secs(4);

fn runtime() -> Runtime {
    rom_consumer::declarations()
        .limits(rom::Limits {
            actions: 32,
            io_jobs: 32,
            ..Default::default()
        })
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(2).unwrap(),
        )
        .unwrap()
}

struct ActorGate {
    release: Arc<Semaphore>,
    entered: mpsc::UnboundedReceiver<()>,
    resolver: AsyncAuthResolver,
    calls: Arc<AtomicUsize>,
}
impl ActorGate {
    fn new() -> Self {
        let release = Arc::new(Semaphore::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let (sender, entered) = mpsc::unbounded_channel();
        let resolver = {
            let release = release.clone();
            let calls = calls.clone();
            Arc::new(move |_| {
                let release = release.clone();
                let calls = calls.clone();
                let sender = sender.clone();
                Box::pin(async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    sender.send(()).unwrap();
                    release.acquire().await.unwrap().forget();
                    Ok(Actor::trusted("local", "alice"))
                }) as AuthFuture
            }) as AsyncAuthResolver
        };
        Self {
            release,
            entered,
            resolver,
            calls,
        }
    }
    async fn entered(&mut self, count: usize) {
        for _ in 0..count {
            self.entered.recv().await.unwrap();
        }
    }
}

struct Listener {
    address: std::net::SocketAddr,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<rom::Result<()>>,
}
impl Listener {
    async fn start(http: Http) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, receiver) = oneshot::channel();
        let task = tokio::spawn(http.serve(listener, async move {
            let _ = receiver.await;
        }));
        Self {
            address,
            stop,
            task,
        }
    }
    async fn finish(self) {
        self.stop.send(()).unwrap();
        self.task.await.unwrap().unwrap();
    }
}

async fn request(address: std::net::SocketAddr, complete_body: bool) -> String {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(b"POST /discover HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 2\r\n\r\n").await.unwrap();
    if complete_body {
        socket.write_all(b"{}").await.unwrap();
    }
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    response
}
fn status(response: &str) -> u16 {
    response.split_whitespace().nth(1).unwrap().parse().unwrap()
}
fn overloaded(response: &str) {
    assert_eq!(status(response), 429);
    let json: serde_json::Value =
        serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(json, serde_json::json!({"error": "overloaded"}));
}
fn no_core_overloads(runtime: &Runtime) {
    let counts = runtime.core_overload_stats();
    assert_eq!(counts.io_no_permits, 0);
    assert_eq!(counts.action_no_permits, 0);
    assert_eq!(counts.observation_generation_exhausted, 0);
    assert_eq!(counts.actor_generation_exhausted, 0);
    assert!(!counts.overflowed);
}

#[tokio::test]
async fn admission_boundary_default16_router_rejects_other16_before_actor_or_core() {
    tokio::time::timeout(BOUND, async {
        let runtime = runtime();
        let mut gate = ActorGate::new();
        let limits = Limits::default();
        let http = Http::new_async(runtime.clone(), gate.resolver.clone(), limits).unwrap();
        let listener = Listener::start(http).await;
        let admitted: Vec<_> = (0..16)
            .map(|_| tokio::spawn(request(listener.address, true)))
            .collect();
        gate.entered(16).await;
        let rejected: Vec<_> = (0..16)
            .map(|_| tokio::spawn(request(listener.address, true)))
            .collect();
        let mut failures = Vec::new();
        for task in rejected {
            failures.push(task.await.unwrap());
        }
        let calls_while_held = gate.calls.load(Ordering::SeqCst);
        let counts_while_held = runtime.core_overload_stats();
        gate.release.add_permits(16);
        let mut successes = Vec::new();
        for task in admitted {
            successes.push(task.await.unwrap());
        }
        // A later real route reaches authentication, proving release restored admission.
        gate.release.add_permits(1);
        let later = request(listener.address, true).await;
        listener.finish().await;
        assert_eq!(limits.bodies, 16);
        assert_eq!(calls_while_held, 16);
        assert_eq!(counts_while_held.io_no_permits, 0);
        assert_eq!(counts_while_held.action_no_permits, 0);
        assert_eq!(counts_while_held.observation_generation_exhausted, 0);
        assert_eq!(counts_while_held.actor_generation_exhausted, 0);
        assert!(!counts_while_held.overflowed);
        for failure in failures {
            overloaded(&failure);
        }
        for success in successes {
            assert_eq!(status(&success), 200);
        }
        assert_eq!(status(&later), 200);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 17);
        no_core_overloads(&runtime);
    })
    .await
    .unwrap();
}

async fn decode_status(shared: Shared) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri("/discover")
        .body(Body::from("{}"))
        .unwrap();
    match crate::request::decode_empty(&shared, request).await {
        Ok(_) => StatusCode::OK,
        Err(failure) => failure.into_response().status(),
    }
}

#[tokio::test]
async fn admission_boundary_real_decoder_cancellation_returns_owned_body_permits() {
    tokio::time::timeout(BOUND, async {
        let runtime = runtime();
        let mut gate = ActorGate::new();
        let limits = Limits::default();
        let bodies = Arc::new(Semaphore::new(limits.bodies));
        let shared = Shared {
            runtime: runtime.clone(),
            auth: Resolver::Async(gate.resolver.clone()),
            limits,
            bodies: bodies.clone(),
            closed: watch::channel(false).0,
        };
        let mut admitted: Vec<_> = (0..16)
            .map(|_| tokio::spawn(decode_status(shared.clone())))
            .collect();
        gate.entered(16).await;
        let full = bodies.available_permits();
        let rejection = decode_status(shared.clone()).await;
        let calls_before_cancel = gate.calls.load(Ordering::SeqCst);
        let mut cancelled = Vec::new();
        for task in admitted.drain(..8) {
            task.abort();
            cancelled.push(task.await.is_err_and(|error| error.is_cancelled()));
        }
        let after_cancel = bodies.available_permits();
        let replacement: Vec<_> = (0..8)
            .map(|_| tokio::spawn(decode_status(shared.clone())))
            .collect();
        gate.entered(8).await;
        let after_replacement = bodies.available_permits();
        gate.release.add_permits(16);
        let mut results = Vec::new();
        for task in admitted.into_iter().chain(replacement) {
            results.push(task.await.unwrap());
        }
        runtime.shutdown().await.unwrap();
        assert!(cancelled.iter().all(|cancelled| *cancelled));
        assert_eq!(full, 0);
        assert_eq!(rejection, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(calls_before_cancel, 16);
        assert_eq!(after_cancel, 8);
        assert_eq!(after_replacement, 0);
        assert_eq!(gate.calls.load(Ordering::SeqCst), 24);
        assert_eq!(bodies.available_permits(), 16);
        assert!(results.iter().all(|status| *status == StatusCode::OK));
        no_core_overloads(&runtime);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn admission_boundary_body_timeout_reaches_actor_then_429_without_core_overload() {
    tokio::time::timeout(BOUND, async {
        let runtime = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let resolver = {
            let calls = calls.clone();
            Arc::new(move |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(Actor::trusted("local", "alice")) }) as AuthFuture
            }) as AsyncAuthResolver
        };
        let http = Http::new_async(
            runtime.clone(),
            resolver,
            Limits {
                body_timeout: Duration::from_millis(50),
                ..Limits::default()
            },
        )
        .unwrap();
        let listener = Listener::start(http).await;
        let timeout = request(listener.address, false).await;
        let calls_at_timeout = calls.load(Ordering::SeqCst);
        let later = request(listener.address, true).await;
        listener.finish().await;
        overloaded(&timeout);
        assert_eq!(calls_at_timeout, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(status(&later), 200);
        no_core_overloads(&runtime);
    })
    .await
    .unwrap();
}
