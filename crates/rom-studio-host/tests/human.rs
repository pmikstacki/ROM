use rom::{Actor, Command, PrincipalKind, Resource, Runtime};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::{HostConfig, OidcProviderConfig, StudioHost, StudioSettings};
use std::sync::atomic::{AtomicU64, Ordering};
#[path = "support/blob.rs"]
mod blob;
#[path = "support/current_bind.rs"]
mod current_bind;
struct Time(AtomicU64);
impl rom::Clock for Time {
    fn now(&self) -> u64 {
        rom::Clock::now(&rom::SystemClock).saturating_add(self.0.load(Ordering::SeqCst))
    }
}
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    process::{Child, Command as Process, Stdio},
    sync::Arc,
};

#[derive(Clone, Resource)]
#[resource(name = "documents")]
struct Document {
    done: bool,
}
struct Provider(
    Child,
    Arc<std::sync::Mutex<BufReader<std::process::ChildStdout>>>,
);
impl Provider {
    fn command(&mut self, command: &str) {
        use std::io::Write;
        writeln!(
            self.0.stdin.as_mut().unwrap(),
            "{}",
            serde_json::to_string(command).unwrap()
        )
        .unwrap();
    }
    async fn event(&mut self, expected: &str) {
        let reader = self.1.clone();
        let line = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::task::spawn_blocking(move || {
                let mut reader = reader.lock().unwrap();
                loop {
                    let mut line = String::new();
                    assert_ne!(
                        reader.read_line(&mut line).unwrap(),
                        0,
                        "private provider event pipe closed"
                    );
                    if !line.trim().is_empty() {
                        break line;
                    }
                }
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            !line.is_empty(),
            "provider EOF while awaiting {expected}, status {:?}",
            self.0.try_wait().unwrap()
        );
        let value: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("private event {line:?}: {error}"));
        assert_eq!(value["event"], expected);
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn provider(callback: &str) -> (Provider, String) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/provider-server.mjs");
    let mut child = Process::new("node")
        .arg(path)
        .arg(callback)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    reader.read_line(&mut line).unwrap();
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    (
        Provider(child, Arc::new(std::sync::Mutex::new(reader))),
        value["issuer"].as_str().unwrap().into(),
    )
}
struct Browser {
    client: reqwest::Client,
    cookies: BTreeMap<String, BTreeMap<String, String>>,
}
impl Browser {
    fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            cookies: BTreeMap::new(),
        }
    }
    async fn send(
        &mut self,
        method: reqwest::Method,
        url: &str,
        form: Option<&[(&str, &str)]>,
    ) -> reqwest::Response {
        let origin = url::Url::parse(url).unwrap().origin().ascii_serialization();
        let cookies = self
            .cookies
            .entry(origin.clone())
            .or_default()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        let mut request = self.client.request(method, url).header("cookie", cookies);
        if let Some(form) = form {
            request = request.header("origin", &origin).form(form);
        }
        let response = request.send().await.unwrap();
        for header in response.headers().get_all("set-cookie") {
            let pair = header.to_str().unwrap().split(';').next().unwrap();
            let (name, value) = pair.split_once('=').unwrap();
            self.cookies
                .entry(origin.clone())
                .or_default()
                .insert(name.into(), value.into());
        }
        response
    }
    fn cookie(&self, origin: &str) -> String {
        self.cookies
            .get(origin)
            .unwrap()
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}
#[tokio::test]
async fn actual_provider_login_binds_current_resources_and_protects_generic_api() {
    journey(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        ExpiryProbe::Stalled,
    )
    .await;
    let path = std::env::temp_dir().join(format!(
        "rom-human-redb-{}-{}.redb",
        std::process::id(),
        rom::Clock::now(&rom::SystemClock)
    ));
    journey(
        Arc::new(rom_redb::Redb::open(&path).unwrap()),
        ExpiryProbe::Stalled,
    )
    .await;
    std::fs::remove_file(&path).unwrap();
}
#[tokio::test]
async fn logout_closes_stalled_current_identity_bind() {
    journey(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        ExpiryProbe::Ordinary,
    )
    .await;
}
#[derive(Clone, Copy)]
enum ExpiryProbe {
    Ordinary,
    Stalled,
    FailedAfterExpiry,
}
#[tokio::test]
async fn expiry_rechecks_original_lease_after_current_bind_fails() {
    journey(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        ExpiryProbe::FailedAfterExpiry,
    )
    .await;
}
async fn journey(storage: Arc<dyn rom::Storage>, expiry_probe: ExpiryProbe) {
    let current_bind = Arc::new(current_bind::Controlled::new(storage));
    let blob_metadata = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let visible_metadata = blob_metadata.clone();
    let blob_names = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let clock = Arc::new(Time(AtomicU64::new(0)));
    let admin = Actor::trusted("host", "configuration");
    let gate = IdentityGate::default()
        .allow_host("host", PrincipalKind::Embedded, "configuration")
        .unwrap()
        .allow_host("rom-blob-host", PrincipalKind::Service, "attachments")
        .unwrap();
    let runtime = Runtime::builder()
        .resource(
            rom_blob::definition().discovery_policy(move |actor, target| {
                actor.authority == "local"
                    && actor.principal_kind() == PrincipalKind::Human
                    && !matches!(
                        (visible_metadata.load(Ordering::SeqCst), target),
                        (1, rom::DiscoveryTarget::Field("store"))
                            | (2, rom::DiscoveryTarget::Resource)
                    )
            }),
        )
        .clock(clock.clone())
        .actor_gate(Arc::new(gate))
        .resource(
            StudioSettings::definition()
                .policy(|a, _, _| a.authority == "host")
                .allow_all_fields(),
        )
        .resource(
            User::definition()
                .policy(|a, _, _| a.authority == "host")
                .allow_all_fields(),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| a.authority == "host")
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| a.authority == "host")
                .allow_all_fields(),
        )
        .resource(
            Document::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .discovery_policy(|_, _| true),
        )
        .build(current_bind.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (mut provider_process, issuer) =
        provider(&format!("{origin}/rom-studio/auth/callback/local"));
    runtime
        .execute(
            &admin,
            Command::create(
                "alice-user",
                User {
                    enabled: true,
                    display_name: "Alice".into(),
                },
            )
            .idempotency("seed-user"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &admin,
            Command::create(
                "local",
                IdentityProvider {
                    enabled: true,
                    profile: ProviderProfile::OidcRs256Human,
                    issuer: issuer.clone(),
                    audience: "studio".into(),
                    endpoint: None,
                    credential_ref: None,
                },
            )
            .idempotency("seed-provider"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &admin,
            Command::create(
                &link_key("local", PrincipalKind::Human, "alice"),
                IdentityLink {
                    authority: "local".into(),
                    subject: "alice".into(),
                    principal_kind: "human".into(),
                    user_id: "alice-user".into(),
                    enabled: true,
                },
            )
            .idempotency("seed-link"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &admin,
            Command::create(
                "main",
                StudioSettings {
                    primary_provider: None,
                },
            )
            .idempotency("studio-settings"),
        )
        .await
        .unwrap();
    let blob_store = Arc::new(blob::Memory::default());
    let blobs = rom_blob::BlobService::builder(runtime.clone())
        .store("attachments", blob_store.clone())
        .limits(rom_blob::Limits {
            chunk_bytes: 2,
            ..Default::default()
        })
        .build()
        .unwrap();
    let visible_names = blob_names.clone();
    let config = HostConfig::new(
        &origin,
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        admin.clone(),
    )
    .allow_loopback_http(true)
    .clock(clock.clone())
    .settings("main")
    .blobs(blobs)
    .blob_store_discovery(move |actor, name| {
        visible_names.load(Ordering::SeqCst)
            && actor.authority == "local"
            && actor.principal_kind() == PrincipalKind::Human
            && name == "attachments"
    })
    .provider(OidcProviderConfig {
        authority: "local".into(),
        label: "Local".into(),
        issuer: issuer.clone(),
        client_id: "studio".into(),
        authorization_endpoint: format!("{issuer}/auth"),
        token_endpoint: format!("{issuer}/token"),
        jwks_endpoint: format!("{issuer}/jwks"),
        client_secret: Some("controlled-host-test-secret".into()),
    });
    let host = StudioHost::new(runtime.clone(), config).unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(host.serve(listener, async {
        let _ = stopped.await;
    }));
    let mut browser = Browser::new();
    let listed = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/providers"),
            None,
        )
        .await;
    let listed: serde_json::Value = serde_json::from_slice(&listed.bytes().await.unwrap()).unwrap();
    assert!(listed["primary"].is_null());
    runtime
        .execute(
            &admin,
            Command::replace(
                "main",
                StudioSettings {
                    primary_provider: Some("local".into()),
                },
            )
            .at_revision(1)
            .idempotency("select-primary"),
        )
        .await
        .unwrap();
    let listed = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/providers"),
            None,
        )
        .await;
    let listed: serde_json::Value = serde_json::from_slice(&listed.bytes().await.unwrap()).unwrap();
    assert_eq!(listed["primary"], "local");
    let callback = authorize(&mut browser, &origin, &issuer).await;
    let session = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    let session: serde_json::Value =
        serde_json::from_slice(&session.bytes().await.unwrap()).unwrap();
    assert_eq!(session["authenticated"], true);
    assert_eq!(session["user_id"], "alice-user");
    capabilities_journey(&browser, &origin, &blob_metadata, &blob_names).await;
    blob_journey(
        &browser,
        &origin,
        session["csrf_token"].as_str().unwrap(),
        &blob_store,
    )
    .await;
    let rejected = browser
        .client
        .post(format!("{origin}/rom-studio/api/invoke"))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);
    assert_eq!(rejected.headers()["content-type"], "application/json");
    let rejected: serde_json::Value =
        serde_json::from_slice(&rejected.bytes().await.unwrap()).unwrap();
    assert_eq!(rejected, serde_json::json!({"error":"denied"}));
    let first_generation = session["generation"].clone();
    clock.0.fetch_add(31, Ordering::SeqCst);
    let renewed = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    let renewed: serde_json::Value =
        serde_json::from_slice(&renewed.bytes().await.unwrap()).unwrap();
    assert_eq!(renewed["authenticated"], true);
    assert_eq!(renewed["generation"], first_generation);
    let csrf = session["csrf_token"].as_str().unwrap();
    let response = browser
        .client
        .post(format!("{origin}/rom-studio/api/discover"))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .header("x-rom-csrf", csrf)
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let replay = browser.send(reqwest::Method::GET, &callback, None).await;
    assert_eq!(replay.status(), 401);
    let mut live = browser
        .client
        .post(format!("{origin}/rom-studio/api/live"))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .header("x-rom-csrf", csrf)
        .body(r#"{"kind":"documents","field":"done","value":false}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(live.status(), 200);
    assert!(live.chunk().await.unwrap().is_some());
    if !matches!(expiry_probe, ExpiryProbe::Ordinary) {
        current_bind.arm();
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            current_bind.started.notified(),
        )
        .await
        .unwrap();
    }
    clock.0.fetch_add(31, Ordering::SeqCst);
    if matches!(expiry_probe, ExpiryProbe::FailedAfterExpiry) {
        current_bind.release();
    }
    let lease_end = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut terminal = Vec::new();
        while let Some(chunk) = live.chunk().await.unwrap() {
            terminal.extend_from_slice(&chunk);
        }
        String::from_utf8(terminal).unwrap()
    })
    .await;
    if !matches!(expiry_probe, ExpiryProbe::Ordinary) {
        current_bind.release();
    }
    let lease_end =
        lease_end.expect("expiry must close the stream while current bind remains stalled");
    assert!(lease_end.contains("identity_expired"), "{lease_end}");
    let renewed = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    let renewed: serde_json::Value =
        serde_json::from_slice(&renewed.bytes().await.unwrap()).unwrap();
    assert_eq!(renewed["authenticated"], true);
    live = browser
        .client
        .post(format!("{origin}/rom-studio/api/live"))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .header("x-rom-csrf", csrf)
        .body(r#"{"kind":"documents","field":"done","value":false}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(live.status(), 200);
    assert!(live.chunk().await.unwrap().is_some());
    current_bind.arm();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        current_bind.started.notified(),
    )
    .await
    .unwrap();
    let logout = browser
        .client
        .post(format!("{origin}/rom-studio/auth/logout"))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .header("x-rom-csrf", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status(), 204);
    let logout_end = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while live.chunk().await.unwrap().is_some() {}
    })
    .await;
    if logout_end.is_err() {
        current_bind.release();
    }
    logout_end
        .expect("logout must close a session-owned stream while current bind remains stalled");
    assert!(
        runtime.status().unwrap().owned_work > 0,
        "the stopped observer must not abandon accepted storage work"
    );
    if matches!(expiry_probe, ExpiryProbe::Ordinary) {
        stop.send(()).unwrap();
        let mut task = task;
        let pending = tokio::time::timeout(std::time::Duration::from_millis(30), &mut task).await;
        current_bind.release();
        assert!(
            pending.is_err(),
            "shutdown must still drain accepted current bind"
        );
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        return;
    }
    current_bind.release();
    authorize(&mut browser, &origin, &issuer).await;
    runtime
        .execute(
            &admin,
            Command::replace(
                "alice-user",
                User {
                    enabled: false,
                    display_name: "Alice".into(),
                },
            )
            .at_revision(1)
            .idempotency("disable-user"),
        )
        .await
        .unwrap();
    let session = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    let session: serde_json::Value =
        serde_json::from_slice(&session.bytes().await.unwrap()).unwrap();
    assert_eq!(session["authenticated"], false);
    let hidden_capabilities = browser
        .client
        .get(format!("{origin}/rom-studio/blobs/capabilities"))
        .header("cookie", browser.cookie(&origin))
        .send()
        .await
        .unwrap();
    assert_eq!(
        hidden_capabilities.status(),
        401,
        "disabled current User must not disclose cached capabilities"
    );
    runtime
        .execute(
            &admin,
            Command::replace(
                "alice-user",
                User {
                    enabled: true,
                    display_name: "Alice".into(),
                },
            )
            .at_revision(2)
            .idempotency("reenable-user"),
        )
        .await
        .unwrap();
    authorize(&mut browser, &origin, &issuer).await;
    let session = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    let session: serde_json::Value =
        serde_json::from_slice(&session.bytes().await.unwrap()).unwrap();
    let csrf = session["csrf_token"].as_str().unwrap();
    let reserved = browser.client.post(format!("{origin}/rom-studio/blobs/reserve"))
        .header("cookie",browser.cookie(&origin)).header("origin",&origin).header("x-rom-csrf",csrf)
        .body(serde_json::json!({"id":"shutdown-avatar","store":"attachments","digest":rom_blob::Digest::of(b"bye").as_str(),"bytes":3,"idempotency":"shutdown-reserve"}).to_string()).send().await.unwrap();
    assert_eq!(reserved.status(), 200);
    let mut signing = Browser::new();
    let callback = authorization(&mut signing, &origin, &issuer, false).await;
    provider_process.command("pause-token");
    provider_process.event("armed").await;
    let request = signing
        .client
        .get(callback)
        .header("cookie", signing.cookie(&origin));
    let auth_caller = tokio::spawn(async move { request.send().await });
    provider_process.event("token-started").await;
    auth_caller.abort();
    let _ = auth_caller.await;
    blob_store.pause_create.store(true, Ordering::SeqCst);
    let request = browser
        .client
        .post(format!(
            "{origin}/rom-studio/blobs/upload?id=shutdown-avatar"
        ))
        .header("cookie", browser.cookie(&origin))
        .header("origin", &origin)
        .header("x-rom-csrf", csrf)
        .body("bye");
    let caller = tokio::spawn(async move { request.send().await });
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        blob_store.started.notified(),
    )
    .await
    .unwrap();
    caller.abort();
    let _ = caller.await;
    stop.send(()).unwrap();
    let mut task = task;
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut task)
            .await
            .is_err()
    );
    blob_store.release.notify_one();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut task)
            .await
            .is_err()
    );
    provider_process.command("release");
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

async fn authorize(browser: &mut Browser, origin: &str, issuer: &str) -> String {
    authorization(browser, origin, issuer, true).await
}
async fn authorization(
    browser: &mut Browser,
    origin: &str,
    issuer: &str,
    complete: bool,
) -> String {
    let mut current = format!("{origin}/rom-studio/auth/login/local");
    let mut callback = None;
    for _ in 0..16 {
        let response = browser.send(reqwest::Method::GET, &current, None).await;
        if current.starts_with(&format!("{origin}/rom-studio/auth/")) {
            assert!(
                response.status().is_redirection(),
                "missing or rejected host auth route: {}",
                response.status()
            );
        }
        if response.status().is_redirection() {
            let target = url::Url::parse(&current)
                .unwrap()
                .join(response.headers()["location"].to_str().unwrap())
                .unwrap()
                .to_string();
            if target.starts_with(&format!("{origin}/rom-studio/auth/callback/local")) {
                if !complete {
                    return target;
                }
                callback = Some(target.clone());
            }
            current = target;
            if current == format!("{origin}/rom-studio/") {
                break;
            }
        } else {
            assert_eq!(response.status(), 200, "{current}");
            let html = response.text().await.unwrap();
            let action = html
                .split("<form method=\"post\" action=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            let action = url::Url::parse(issuer)
                .unwrap()
                .join(action)
                .unwrap()
                .to_string();
            let fields = if action.ends_with("/login") {
                vec![("account", "alice")]
            } else {
                vec![("consent", "accept")]
            };
            let response = browser
                .send(reqwest::Method::POST, &action, Some(&fields))
                .await;
            assert!(response.status().is_redirection());
            current = url::Url::parse(&action)
                .unwrap()
                .join(response.headers()["location"].to_str().unwrap())
                .unwrap()
                .to_string();
        }
    }
    callback.expect("provider must return a callback")
}

async fn blob_journey(browser: &Browser, origin: &str, csrf: &str, store: &blob::Memory) {
    let request = |path: &str| {
        browser
            .client
            .post(format!("{origin}/rom-studio/blobs/{path}"))
            .header("cookie", browser.cookie(origin))
            .header("origin", origin)
            .header("x-rom-csrf", csrf)
    };
    let reserved = request("reserve").body(serde_json::json!({"id":"avatar","store":"attachments","digest":rom_blob::Digest::of(b"hello").as_str(),"bytes":5,"idempotency":"avatar-reserve"}).to_string()).send().await.unwrap();
    assert_eq!(reserved.status(), 200);
    let uploaded = request("upload?id=avatar")
        .body("hello")
        .send()
        .await
        .unwrap();
    assert_eq!(uploaded.status(), 200);
    let uploaded: serde_json::Value =
        serde_json::from_slice(&uploaded.bytes().await.unwrap()).unwrap();
    assert_eq!(uploaded["status"], "attached");
    let read = browser
        .client
        .get(format!("{origin}/rom-studio/blobs/attachment/avatar"))
        .header("cookie", browser.cookie(origin))
        .send()
        .await
        .unwrap();
    assert_eq!(read.status(), 200);
    assert_eq!(read.bytes().await.unwrap().as_ref(), b"hello");
    let rejected = browser
        .client
        .post(format!("{origin}/rom-studio/blobs/detach"))
        .header("cookie", browser.cookie(origin))
        .header("origin", origin)
        .body(r#"{"id":"avatar"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 403);
    let detached = request("detach")
        .body(r#"{"id":"avatar"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(detached.status(), 200);
    let read = browser
        .client
        .get(format!("{origin}/rom-studio/blobs/attachment/avatar"))
        .header("cookie", browser.cookie(origin))
        .send()
        .await
        .unwrap();
    assert_eq!(read.status(), 404);
    let reserved = request("reserve").body(serde_json::json!({"id":"uncertain","store":"attachments","digest":rom_blob::Digest::of(b"retry").as_str(),"bytes":5,"idempotency":"uncertain-reserve"}).to_string()).send().await.unwrap();
    assert_eq!(reserved.status(), 200);
    store.unknown_create.store(true, Ordering::SeqCst);
    let unknown = request("upload?id=uncertain")
        .body("retry")
        .send()
        .await
        .unwrap();
    assert_eq!(unknown.status(), 503);
    let unknown: serde_json::Value =
        serde_json::from_slice(&unknown.bytes().await.unwrap()).unwrap();
    assert_eq!(unknown, serde_json::json!({"error":"outcome_unknown"}));
    let retry = request("upload?id=uncertain")
        .body("retry")
        .send()
        .await
        .unwrap();
    assert_eq!(retry.status(), 200);
    let invalid = request("reserve")
        .body(r#"{"id":"first","id":"second"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), 400);
    let missing_dot = browser
        .client
        .get(format!(
            "{origin}/rom-studio/blobs/attachment?{}",
            query_id(".")
        ))
        .header("cookie", browser.cookie(origin))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_dot.status(), 404);
    let missing_dot: serde_json::Value =
        serde_json::from_slice(&missing_dot.bytes().await.unwrap()).unwrap();
    assert_eq!(missing_dot, serde_json::json!({"error":"missing"}));
    for id in [".", "..", "folder/name"] {
        let reserved=request("reserve").body(serde_json::json!({"id":id,"store":"attachments","digest":rom_blob::Digest::of(id.as_bytes()).as_str(),"bytes":id.len(),"idempotency":format!("exact-{id}")}).to_string()).send().await.unwrap();
        assert_eq!(reserved.status(), 200);
        let uploaded = request(&format!("upload?{}", query_id(id)))
            .body(id.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(uploaded.status(), 200);
        let download = browser
            .client
            .get(format!(
                "{origin}/rom-studio/blobs/attachment?{}",
                query_id(id)
            ))
            .header("cookie", browser.cookie(origin))
            .send()
            .await
            .unwrap();
        assert_eq!(download.status(), 200);
        assert_eq!(download.bytes().await.unwrap().as_ref(), id.as_bytes());
    }
    let invalid_query = request("upload?id=avatar&id=uncertain")
        .body("x")
        .send()
        .await
        .unwrap();
    assert_eq!(invalid_query.status(), 400);
    let invalid_query: serde_json::Value =
        serde_json::from_slice(&invalid_query.bytes().await.unwrap()).unwrap();
    assert_eq!(invalid_query, serde_json::json!({"error":"invalid"}));
}

async fn capabilities_journey(
    browser: &Browser,
    origin: &str,
    metadata: &std::sync::atomic::AtomicU8,
    names: &std::sync::atomic::AtomicBool,
) {
    let request = || {
        browser
            .client
            .get(format!("{origin}/rom-studio/blobs/capabilities"))
            .header("cookie", browser.cookie(origin))
    };
    let initial = request().send().await.unwrap();
    assert_eq!(initial.status(), 200);
    let initial: serde_json::Value =
        serde_json::from_slice(&initial.bytes().await.unwrap()).unwrap();
    assert_eq!(
        initial["stores"],
        serde_json::json!([]),
        "descriptor visibility is not store-name disclosure authority"
    );
    names.store(true, Ordering::SeqCst);
    let allowed = request().send().await.unwrap();
    assert_eq!(allowed.status(), 200);
    let allowed: serde_json::Value =
        serde_json::from_slice(&allowed.bytes().await.unwrap()).unwrap();
    assert_eq!(
        allowed,
        serde_json::json!({"version":1,"resource_kind":"blobs","stores":["attachments"],"limits":{"blob_bytes":1048576,"chunk_bytes":2,"chunks":1024},"operations":["reserve","upload","download","detach"]})
    );
    for hidden in [1, 2] {
        metadata.store(hidden, Ordering::SeqCst);
        let denied = request().send().await.unwrap();
        assert_eq!(denied.status(), 403);
        let denied: serde_json::Value =
            serde_json::from_slice(&denied.bytes().await.unwrap()).unwrap();
        assert_eq!(denied, serde_json::json!({"error":"denied"}));
    }
    metadata.store(0, Ordering::SeqCst);
}
fn query_id(id: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("id", id)
        .finish()
}
