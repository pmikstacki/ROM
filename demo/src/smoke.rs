//! Finite user journey over the same generic routes used by the server.
use crate::*;
use rom::{Invocation, json};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
pub type SmokeResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
struct Client(SocketAddr);
impl Client {
    async fn stream(
        &self,
        route: &str,
        body: &Value,
        auth: bool,
        close: bool,
    ) -> SmokeResult<TcpStream> {
        let body = body.to_string();
        let mut socket = TcpStream::connect(self.0).await?;
        socket.write_all(format!("POST {route} HTTP/1.1\r\nHost: localhost\r\n{}{}Content-Length: {}\r\n\r\n{body}", if auth { "Authorization: Demo local\r\n" } else { "" }, if close { "Connection: close\r\n" } else { "" }, body.len()).as_bytes()).await?;
        Ok(socket)
    }
    async fn post(&self, route: &str, body: Value, auth: bool) -> SmokeResult<(u16, Value)> {
        let mut socket = self.stream(route, &body, auth, true).await?;
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(5), socket.read_to_string(&mut response))
            .await??;
        let code = response
            .split_whitespace()
            .nth(1)
            .ok_or("missing HTTP status")?
            .parse()?;
        let body = response
            .split_once("\r\n\r\n")
            .ok_or("missing HTTP body")?
            .1;
        Ok((code, serde_json::from_str(body)?))
    }
    async fn invoke<R: Resource>(&self, command: Command<R>) -> SmokeResult<Value> {
        let invocation: Invocation = command.into();
        let (status, value) = self
            .post("/invoke", serde_json::to_value(invocation)?, true)
            .await?;
        assert_eq!(status, 200, "invoke failed: {value}");
        Ok(value)
    }
}
async fn until(socket: &mut TcpStream, needle: &str) -> SmokeResult<String> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut text = String::new();
        let mut buffer = [0; 4096];
        while !text.contains(needle) {
            let length = socket.read(&mut buffer).await?;
            if length == 0 {
                return Err("stream closed early".into());
            }
            text.push_str(std::str::from_utf8(&buffer[..length])?);
        }
        Ok(text)
    })
    .await?
}
/// Fresh synthetic database; validates successful and rejected calls, then drains the server.
pub async fn run(redb: bool) -> SmokeResult<()> {
    let directory = std::env::var_os("ROM_DEMO_TMP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!(
        "rom-demo-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path)?)
    };
    let notices = Notices::default();
    let runtime = build(storage.clone(), notices.clone())?;
    bootstrap(&runtime).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let client = Client(listener.local_addr()?);
    let (stop, stopped) = oneshot::channel();
    let http = rom_http::Http::new(runtime.clone(), resolver(), Default::default())?;
    let server = tokio::spawn(http.serve(listener, async {
        let _ = stopped.await;
    }));
    let result = journey(&client, &runtime, &notices).await;
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(5), server).await???;
    assert_eq!(runtime.status()?.intake, rom::IntakeState::Stopped);
    assert_eq!(runtime.status()?.owned_work, 0);
    drop(runtime);
    drop(storage);
    std::fs::remove_file(path)?;
    result
}
async fn journey(client: &Client, runtime: &Runtime, notices: &Notices) -> SmokeResult<()> {
    assert_eq!(
        client
            .post("/read", json!({"kind":"settings","id":"workshop"}), false)
            .await?
            .0,
        403
    );
    assert_eq!(
        client
            .post("/read", json!({"kind":"users","id":"alice"}), true)
            .await?
            .0,
        403
    );
    let created = client
        .invoke(
            Command::create(
                "first",
                Task {
                    title: "Draft".into(),
                    done: false,
                },
            )
            .idempotency("task-create"),
        )
        .await?;
    assert_eq!(created["revision"], 1);
    let inventory = client
        .invoke(
            Command::create(
                "bin",
                InventoryItem {
                    code: StockCode::decode(json!(" bolt-7 "))?,
                    quantity: 12,
                },
            )
            .idempotency("inventory-create"),
        )
        .await?;
    assert_eq!(inventory["value"]["code"], "BOLT-7");
    let invalid = json!({"kind":"inventory","id":"invalid","expected":null,"idempotency":"invalid","operation":{"type":"create","input":{"code":"bad/code","quantity":2}}});
    assert_eq!(client.post("/invoke", invalid, true).await?.0, 400);
    for (kind, field, value) in [
        ("tasks", "done", json!(false)),
        ("inventory", "code", json!(" bolt-7 ")),
    ] {
        let (status, rows) = client
            .post(
                "/query",
                json!({"kind":kind,"query":{"filters":[{"field":field,"value":value}],"limit":10}}),
                true,
            )
            .await?;
        assert_eq!(status, 200);
        assert_eq!(rows.as_array().unwrap().len(), 1);
    }
    let mut live = client
        .stream(
            "/live",
            &json!({"kind":"tasks","field":"done","value":false}),
            true,
            false,
        )
        .await?;
    until(&mut live, "Draft").await?;
    client
        .invoke(rename_task("first", 1, "Publish workshop".into()))
        .await?;
    until(&mut live, "Publish workshop").await?;
    client
        .invoke(
            Command::action("first", COMPLETE, ())
                .at_revision(2)
                .idempotency("complete"),
        )
        .await?;
    until(&mut live, "data: []").await?;
    drop(live);
    runtime.process_work(32).await?;
    assert_eq!(
        runtime
            .read::<Dashboard>(&session_actor(), "workshop")
            .await?
            .value
            .unwrap()
            .latest,
        "Publish workshop"
    );
    assert_eq!(notices.lock().unwrap().len(), 1);
    assert_eq!(notices.lock().unwrap()[0].1, "Publish workshop");
    let (_, page) = client
        .post("/journal", json!({"kind":"tasks","after":null}), true)
        .await?;
    assert_eq!(page["events"].as_array().unwrap().len(), 3);
    let (_, resumed) = client
        .post(
            "/journal",
            json!({"kind":"tasks","after":page["cursor"]}),
            true,
        )
        .await?;
    assert!(resumed["events"].as_array().unwrap().is_empty());
    let (_, settings) = client
        .post("/read", json!({"kind":"settings","id":"workshop"}), true)
        .await?;
    assert_eq!(settings["value"]["workshop"], "Resource author workshop");
    assert!(settings.get("protected").is_none());
    // Ordinary session cannot modify the source-owned Resource or grant itself bootstrap.
    let denied = Command::replace(
        "workshop",
        Settings {
            workshop: "forged".into(),
            enabled: false,
        },
    )
    .at_revision(1)
    .idempotency("denied");
    let invocation: Invocation = denied.into();
    assert_eq!(
        client
            .post("/invoke", serde_json::to_value(invocation)?, true)
            .await?
            .0,
        403
    );
    Ok(())
}
