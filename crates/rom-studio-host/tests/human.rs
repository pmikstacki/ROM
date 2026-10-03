use rom::{Actor, Command, PrincipalKind, Resource, Runtime};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::{HostConfig, OidcProviderConfig, StudioHost, StudioSettings};
use std::sync::atomic::{AtomicU64, Ordering};
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
struct Provider(Child);
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
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    (Provider(child), value["issuer"].as_str().unwrap().into())
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
    journey(Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap())).await;
    let path = std::env::temp_dir().join(format!(
        "rom-human-redb-{}-{}.redb",
        std::process::id(),
        rom::Clock::now(&rom::SystemClock)
    ));
    journey(Arc::new(rom_redb::Redb::open(&path).unwrap())).await;
    std::fs::remove_file(&path).unwrap();
}
async fn journey(storage: Arc<dyn rom::Storage>) {
    let clock = Arc::new(Time(AtomicU64::new(0)));
    let admin = Actor::trusted("host", "configuration");
    let gate = IdentityGate::default()
        .allow_host("host", PrincipalKind::Embedded, "configuration")
        .unwrap();
    let runtime = Runtime::builder()
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
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (_provider, issuer) = provider(&format!("{origin}/rom-studio/auth/callback/local"));
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
    let config = HostConfig::new(
        &origin,
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        admin.clone(),
    )
    .allow_loopback_http(true)
    .clock(clock.clone())
    .settings("main")
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
    clock.0.fetch_add(31, Ordering::SeqCst);
    let lease_end = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut terminal = Vec::new();
        while let Some(chunk) = live.chunk().await.unwrap() {
            terminal.extend_from_slice(&chunk);
        }
        String::from_utf8(terminal).unwrap()
    })
    .await
    .expect("expired stream lease must close");
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
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while live.chunk().await.unwrap().is_some() {}
    })
    .await
    .expect("logout must close a session-owned stream");
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
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}

async fn authorize(browser: &mut Browser, origin: &str, issuer: &str) -> String {
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
