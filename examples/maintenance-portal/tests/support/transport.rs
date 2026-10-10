use rom::{Runtime, Storage, Value};
use rom_http::{Http, Limits};
use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) value: Value,
}

pub(crate) struct Server {
    address: SocketAddr,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<rom::Result<()>>,
    pub(crate) alice_enabled: Arc<AtomicBool>,
}

impl Server {
    pub(crate) async fn start(store: Arc<dyn Storage>) -> Self {
        let runtime = rom_maintenance_portal::declarations()
            .build(store, Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
        let alice_enabled = Arc::new(AtomicBool::new(true));
        let auth = super::credentials::resolver(alice_enabled.clone());
        let http = Http::new(runtime, auth, Limits::default()).unwrap();
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
            alice_enabled,
        }
    }

    pub(crate) async fn post(&self, path: &str, credential: &str, body: &Value) -> Response {
        self.request(path, Some(credential), body).await
    }

    pub(crate) async fn post_public(&self, path: &str, body: &Value) -> Response {
        self.request(path, None, body).await
    }

    async fn request(&self, path: &str, credential: Option<&str>, body: &Value) -> Response {
        tokio::time::timeout(Duration::from_secs(3), async {
            let body = body.to_string();
            let mut socket = TcpStream::connect(self.address).await.unwrap();
            let authorization = credential.map_or_else(String::new, |value| format!("Authorization: Bearer {value}\r\n"));
            let request = format!("POST {path} HTTP/1.1\r\nHost: localhost\r\n{authorization}X-Subject: alice\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
            socket.write_all(request.as_bytes()).await.unwrap();
            let mut bytes = Vec::new();
            socket.take(131_073).read_to_end(&mut bytes).await.unwrap();
            assert!(bytes.len() <= 131_072, "bounded fixture response exceeded");
            let response = String::from_utf8(bytes).unwrap();
            let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
            let (_, body) = response.split_once("\r\n\r\n").unwrap();
            Response { status, value: rom::parse_json(body.as_bytes()).unwrap() }
        }).await.unwrap()
    }

    pub(crate) async fn finish(self) {
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
