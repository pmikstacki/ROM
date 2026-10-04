use rom::{Actor, Runtime};
use rom_studio_host::{HostConfig, StudioHost};
use std::sync::Arc;

#[tokio::test]
async fn host_mounts_assets_and_session_without_accepting_browser_actor_shortcuts() {
    let directory = std::env::temp_dir().join(format!("rom-host-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("index.html"), "<html>real studio</html>").unwrap();
    let runtime = Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let config = HostConfig::new(
        &origin,
        "/rom-studio/",
        &directory,
        Actor::trusted("host", "configuration"),
    )
    .allow_loopback_http(true);
    let host = StudioHost::new(runtime, config).unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(host.serve(listener, async {
        let _ = stopped.await;
    }));
    let client = reqwest::Client::new();
    let root = client
        .get(format!("{origin}/rom-studio/"))
        .send()
        .await
        .unwrap();
    assert_eq!(root.status(), 200);
    assert!(root.text().await.unwrap().contains("real studio"));
    let page = client
        .get(format!("{origin}/rom-studio/resources/tasks"))
        .send()
        .await
        .unwrap();
    assert_eq!(page.status(), 200);
    assert!(page.text().await.unwrap().contains("real studio"));
    let session = client
        .get(format!("{origin}/rom-studio/auth/session"))
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&session.bytes().await.unwrap()).unwrap();
    assert_eq!(json["authenticated"], false);
    assert!(json.get("csrf_token").is_none());
    let denied = client
        .post(format!("{origin}/rom-studio/api/discover"))
        .header("origin", &origin)
        .header("authorization", "Bearer forged")
        .body(r#"{"actor":{"authority":"host","subject":"configuration"}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 401);
    assert_eq!(denied.headers()["content-type"], "application/json");
    assert_eq!(denied.headers()["cache-control"], "no-store");
    let denied: serde_json::Value = serde_json::from_slice(&denied.bytes().await.unwrap()).unwrap();
    assert_eq!(denied, serde_json::json!({"error":"denied"}));
    let missing = client
        .get(format!("{origin}/rom-studio/assets/missing.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
