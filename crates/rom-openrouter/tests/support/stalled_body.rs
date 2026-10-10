//! Header-first stalled-body fixture; waits are finite and Drop interrupts the stall.
use super::Credentials;
use rom_openrouter::{OpenRouter, OpenRouterConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
pub struct StalledBody {
    endpoint: String,
    stop: Arc<AtomicBool>,
    headers: Arc<AtomicBool>,
    posts: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl StalledBody {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/api/v1", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let headers = Arc::new(AtomicBool::new(false));
        let posts = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(AtomicUsize::new(0));
        let stopping = stop.clone();
        let observed = headers.clone();
        let posted = posts.clone();
        let requested = requests.clone();
        let worker = std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(25);
            while !stopping.load(Ordering::SeqCst) && Instant::now() < until {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("stalled fixture accept: {error}"),
                };
                assert!(requested.fetch_add(1, Ordering::SeqCst) < 5);
                stream
                    .set_read_timeout(Some(Duration::from_millis(500)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    assert!(bytes.len() <= 64 * 1024);
                    if let Some(position) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                    {
                        let head = String::from_utf8_lossy(&bytes[..position]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .map(|value| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= position + 4 + length {
                            break;
                        }
                    }
                }
                let head = String::from_utf8_lossy(&bytes);
                let path = head.split_whitespace().nth(1).unwrap();
                if path == "/api/v1/chat/completions" {
                    assert_eq!(posted.fetch_add(1, Ordering::SeqCst), 0);
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4096\r\nX-Generation-ID: gen-supervisor\r\nConnection: close\r\n\r\n{").unwrap();
                    stream.flush().unwrap();
                    observed.store(true, Ordering::SeqCst);
                    stream
                        .set_read_timeout(Some(Duration::from_millis(10)))
                        .unwrap();
                    let mut probe = [0_u8; 1];
                    while !stopping.load(Ordering::SeqCst) && Instant::now() < until {
                        match stream.peek(&mut probe) {
                            Ok(0) => break,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) => {}
                            Err(_) => break,
                            Ok(_) => {}
                        }
                    }
                    continue;
                }
                let (status, body) = match path {
                    "/api/v1/models" => (
                        "200 OK",
                        r#"{"data":[{"id":"fixture/model","context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":[],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}"#,
                    ),
                    "/api/v1/models/fixture/model/endpoints" => (
                        "200 OK",
                        r#"{"data":{"id":"fixture/model","endpoints":[{"tag":"fixture-provider","model_id":"fixture/model","context_length":8192,"supported_parameters":["max_tokens"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}}"#,
                    ),
                    "/api/v1/generation?id=gen-supervisor" => ("404 Not Found", "{}"),
                    _ => panic!("unexpected finite fixture route"),
                };
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        Self {
            endpoint,
            stop,
            headers,
            posts,
            requests,
            worker: Some(worker),
        }
    }
    pub fn provider(&self) -> OpenRouter {
        OpenRouter::for_loopback(
            OpenRouterConfig {
                endpoint: self.endpoint.clone(),
                credential_ref: "fixture-reference".into(),
                catalog_ttl_seconds: 60,
                response_bytes: 128 * 1024,
            },
            Arc::new(Credentials),
        )
        .unwrap()
    }
    pub fn headers_received(&self) -> bool {
        self.headers.load(Ordering::SeqCst)
    }
    pub fn posts(&self) -> usize {
        self.posts.load(Ordering::SeqCst)
    }
    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}
impl Drop for StalledBody {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}
