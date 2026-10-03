#![allow(dead_code)]
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Directory(pub PathBuf);
impl Directory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn file(&self, name: &str, bytes: impl AsRef<[u8]>) -> String {
        let p = self.0.join(name);
        std::fs::write(&p, bytes).unwrap();
        p.to_str().unwrap().into()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub async fn run(endpoint: &str, auth: Option<&str>, args: &[&str], input: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rom"));
    cmd.args(["--endpoint", endpoint, "--output", "json"]);
    if let Some(auth) = auth {
        cmd.args(["--auth-file", auth]);
    }
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let input = input.as_bytes().to_vec();
    tokio::task::spawn_blocking(move || {
        child.stdin.take().unwrap().write_all(&input).unwrap();
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap()
}
pub fn json(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "{:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
pub async fn request(socket: &mut tokio::net::TcpStream) -> serde_json::Value {
    use tokio::io::AsyncReadExt;
    let mut bytes = Vec::new();
    loop {
        let mut buf = [0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        assert_ne!(n, 0);
        bytes.extend_from_slice(&buf[..n]);
        if let Some(end) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let size = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(|v| v.parse::<usize>().unwrap())
                })
                .unwrap();
            if bytes.len() >= end + 4 + size {
                return serde_json::from_slice(&bytes[end + 4..end + 4 + size]).unwrap();
            }
        }
        assert!(bytes.len() < 70_000);
    }
}
pub async fn response(
    status: u16,
    content_type: &str,
    body: Vec<u8>,
) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let header = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = request(&mut socket).await;
        if socket.write_all(header.as_bytes()).await.is_ok() {
            let _ = socket.write_all(&body).await;
        }
    });
    (endpoint, task)
}
pub struct Server {
    pub endpoint: String,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Server {
    pub async fn new(runtime: rom::Runtime) -> Self {
        let http = rom_http::Http::new(runtime, rom_demo::resolver(), Default::default()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (stop, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            http.serve(listener, async {
                let _ = rx.await;
            })
            .await
            .unwrap()
        });
        Self {
            endpoint,
            stop,
            task,
        }
    }
    pub async fn finish(self) {
        self.stop.send(()).unwrap();
        self.task.await.unwrap();
    }
}
