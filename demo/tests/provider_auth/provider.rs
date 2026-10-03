use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Semaphore, watch},
};

/// Synthetic bounded response fixture; this is not actual-provider certification.
pub struct Provider {
    pub endpoint: String,
    gate: Arc<Semaphore>,
    contacts: watch::Receiver<usize>,
    task: tokio::task::JoinHandle<()>,
}
impl Provider {
    pub async fn new(paused: bool) -> Self {
        Self::start(paused, None).await
    }
    pub async fn expecting_authorization(value: &str) -> Self {
        Self::start(false, Some(value.to_owned())).await
    }
    async fn start(paused: bool, expected: Option<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/introspect", listener.local_addr().unwrap());
        let gate = Arc::new(Semaphore::new(if paused { 0 } else { 64 }));
        let (contacts, receiver) = watch::channel(0);
        let permits = gate.clone();
        let task = tokio::spawn(async move {
            let mut count = 0;
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut request = Vec::new();
                let mut chunk = [0; 1024];
                loop {
                    let n = socket.read(&mut chunk).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..n]);
                    if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let len = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length: ")
                                    .map(str::to_owned)
                            })
                            .unwrap()
                            .parse::<usize>()
                            .unwrap();
                        if request.len() >= end + 4 + len {
                            break;
                        }
                    }
                    assert!(request.len() <= 16384);
                }
                count += 1;
                contacts.send_replace(count);
                permits.acquire().await.unwrap().forget();
                let headers = String::from_utf8_lossy(&request);
                let approved = expected.as_ref().is_none_or(|value| {
                    headers
                        .lines()
                        .any(|line| line.strip_prefix("authorization: ") == Some(value.as_str()))
                });
                let body = if approved {
                    r#"{"active":true,"sub":"service","client_id":"service","principal_kind":"service","token_type":"Bearer","iss":"https://fixture.invalid","aud":"rom-api","exp":200}"#
                } else {
                    r#"{"active":false}"#
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            }
        });
        Self {
            endpoint,
            gate,
            contacts: receiver,
            task,
        }
    }
    pub fn count(&self) -> usize {
        *self.contacts.borrow()
    }
    pub async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.count() == 0 {
                self.contacts.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    pub fn release(&self) {
        self.gate.add_permits(64);
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.task.abort();
    }
}
