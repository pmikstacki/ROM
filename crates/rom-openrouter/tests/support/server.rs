//! Finite real TCP fixtures; no live provider requests.
use super::Credentials;
use rom_openrouter::{OpenRouter, OpenRouterConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub struct Server {
    endpoint: String,
    request: Arc<Mutex<Vec<Vec<u8>>>>,
    worker: Option<thread::JoinHandle<()>>,
    response_release: Option<std::sync::mpsc::Sender<()>>,
}
impl Server {
    #[allow(dead_code, reason = "shared fixture method used by other test targets")]
    pub fn once(status: &str, headers: &str, body: &str, delay: Duration) -> Self {
        Self::sequence(vec![(status.into(), headers.into(), body.into(), delay)])
    }
    pub fn sequence(responses: Vec<(String, String, String, Duration)>) -> Self {
        Self::sequence_held(responses, None, None)
    }
    #[allow(dead_code, reason = "shared fixture method used by other test targets")]
    pub fn once_held(
        status: &str,
        headers: &str,
        body: &str,
    ) -> (Self, tokio::sync::oneshot::Receiver<()>) {
        let (posted, receipt) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        (
            Self::sequence_held(
                vec![(status.into(), headers.into(), body.into(), Duration::ZERO)],
                Some((posted, held)),
                Some(release),
            ),
            receipt,
        )
    }
    fn sequence_held(
        responses: Vec<(String, String, String, Duration)>,
        mut held: Option<(
            tokio::sync::oneshot::Sender<()>,
            std::sync::mpsc::Receiver<()>,
        )>,
        response_release: Option<std::sync::mpsc::Sender<()>>,
    ) -> Self {
        assert!(!responses.is_empty() && responses.len() <= 8);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/api/v1", listener.local_addr().unwrap());
        let request = Arc::new(Mutex::new(Vec::new()));
        let captured = request.clone();
        let worker = thread::spawn(move || {
            for (status, headers, body, delay) in responses {
                let content_type = if headers.to_ascii_lowercase().contains("content-type:") {
                    ""
                } else {
                    "Content-Type: application/json\r\n"
                };
                let content_length = if headers.to_ascii_lowercase().contains("content-length:") {
                    String::new()
                } else {
                    format!("Content-Length: {}\r\n", body.len())
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\n{content_type}{content_length}Connection: close\r\n{headers}\r\n{body}"
                );
                let until = std::time::Instant::now() + Duration::from_secs(2);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if std::time::Instant::now() >= until {
                                return;
                            }
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("fixture accept: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_millis(500)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut complete = false;
                let mut buffer = [0_u8; 2048];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                    assert!(bytes.len() <= 64 * 1024);
                    if let Some(position) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..position]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if held.is_some() {
                            assert!(length > 0, "held fixture requires a POST body");
                        }
                        if bytes.len() >= position + 4 + length {
                            complete = true;
                            break;
                        }
                    }
                }
                if held.is_some() {
                    assert!(complete, "held fixture requires a full HTTP request");
                    assert!(bytes.starts_with(b"POST /api/v1/chat/completions HTTP/1.1\r\n"));
                }
                captured.lock().unwrap().push(bytes);
                if let Some((posted, release)) = held.take() {
                    let _ = posted.send(());
                    if release.recv_timeout(Duration::from_secs(2)).is_err() {
                        return;
                    }
                }
                thread::sleep(delay);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            endpoint,
            request,
            worker: Some(worker),
            response_release,
        }
    }
    #[allow(dead_code, reason = "shared fixture method used by other test targets")]
    pub fn release_response(&mut self) {
        self.response_release.take().unwrap().send(()).unwrap();
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
    #[allow(dead_code, reason = "shared fixture method used by other test targets")]
    pub fn captured(&self) -> Vec<u8> {
        self.request
            .lock()
            .unwrap()
            .last()
            .cloned()
            .unwrap_or_default()
    }
    #[allow(dead_code, reason = "shared fixture method used by other test targets")]
    pub fn requests(&self) -> Vec<Vec<u8>> {
        self.request.lock().unwrap().clone()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        drop(self.response_release.take());
        self.worker.take().unwrap().join().unwrap();
    }
}
