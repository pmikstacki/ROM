//! Path-addressed real HTTP fixture with bounded connection and body handling.
use super::Credentials;
use rom_openrouter::{OpenRouter, OpenRouterConfig};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

pub struct Router {
    endpoint: String,
    captured: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    peak: Arc<AtomicUsize>,
}
impl Router {
    pub fn new(routes: BTreeMap<String, String>) -> Self {
        Self::delayed(routes, Duration::ZERO)
    }
    pub fn delayed(routes: BTreeMap<String, String>, delay: Duration) -> Self {
        assert!(routes.len() <= 258);
        assert!(delay <= Duration::from_millis(250));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/api/v1", listener.local_addr().unwrap());
        let captured = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let seen = captured.clone();
        let stopping = stop.clone();
        let routes = Arc::new(routes);
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let maximum = peak.clone();
        let worker = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let count = AtomicUsize::new(0);
            let mut workers = Vec::new();
            while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        assert!(count.fetch_add(1, Ordering::SeqCst) < 512);
                        let routes = routes.clone();
                        let seen = seen.clone();
                        let active = active.clone();
                        let maximum = maximum.clone();
                        workers.push(std::thread::spawn(move || {
                            stream.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
                            let mut bytes=Vec::new(); let mut buffer=[0_u8;2048];
                            loop {
                                let size=stream.read(&mut buffer).unwrap(); if size==0 {return;} bytes.extend_from_slice(&buffer[..size]); assert!(bytes.len() <= 64*1024);
                                if let Some(position)=bytes.windows(4).position(|bytes|bytes==b"\r\n\r\n") {
                                    let headers=String::from_utf8_lossy(&bytes[..position]).to_lowercase();
                                    let length=headers.lines().find_map(|line|line.strip_prefix("content-length:").map(|value|value.trim().parse::<usize>().unwrap())).unwrap_or(0);
                                    if bytes.len() >= position+4+length {break;}
                                }
                            }
                            let line=String::from_utf8_lossy(&bytes); let path=line.split_whitespace().nth(1).unwrap();
                            let (status,body)=routes.get(path).map_or(("404 Not Found","{}"),|body|("200 OK",body.as_str()));
                            let reply=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                            let current=active.fetch_add(1,Ordering::SeqCst)+1; maximum.fetch_max(current,Ordering::SeqCst);
                            std::thread::sleep(delay);
                            seen.lock().unwrap().push(bytes); let _=stream.write_all(reply.as_bytes());
                            active.fetch_sub(1,Ordering::SeqCst);
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("router fixture accept: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            endpoint,
            captured,
            stop,
            worker: Some(worker),
            peak,
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
    pub fn requests(&self) -> Vec<Vec<u8>> {
        self.captured.lock().unwrap().clone()
    }
    pub fn peak_connections(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }
}
impl Drop for Router {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}
