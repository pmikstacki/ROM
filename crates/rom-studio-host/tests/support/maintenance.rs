//! Actual provider, HTTP session, native backup and offline schema maintenance journey.
use super::*;
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::path::{Path, PathBuf};

#[derive(Clone, Resource)]
#[resource(name = "maintenance-documents")]
struct Before {
    done: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "maintenance-documents", version = 2)]
struct After {
    completed: bool,
}
fn convert(before: Before) -> rom::Result<After> {
    Ok(After {
        completed: before.done,
    })
}

enum Database {
    Sqlite(Arc<rom_sqlite::Sqlite>),
    Redb(Arc<rom_redb::Redb>),
}
impl Database {
    fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(Arc::new(rom_redb::Redb::open(path).unwrap()))
        } else {
            Self::Sqlite(Arc::new(rom_sqlite::Sqlite::open(path).unwrap()))
        }
    }
    fn storage(&self) -> Arc<dyn rom::Storage> {
        match self {
            Self::Sqlite(db) => db.clone(),
            Self::Redb(db) => db.clone(),
        }
    }
    fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(db) => db.counts().unwrap(),
            Self::Redb(db) => db.counts().unwrap(),
        }
    }
    fn backup(&self, path: &Path) -> rom_backup::Snapshot {
        let backend = match self {
            Self::Sqlite(db) => {
                db.backup_to(path, BackupLimits::default()).unwrap();
                rom_backup::Backend::Sqlite
            }
            Self::Redb(db) => {
                db.backup_to(path, BackupLimits::default()).unwrap();
                rom_backup::Backend::Redb
            }
        };
        rom_backup::read(path, backend, BackupLimits::default())
            .unwrap()
            .1
    }
    fn restore(redb: bool, archive: &Path, destination: &Path) -> Self {
        if redb {
            Self::Redb(Arc::new(
                rom_redb::Redb::restore_from(archive, destination, BackupLimits::default())
                    .unwrap(),
            ))
        } else {
            Self::Sqlite(Arc::new(
                rom_sqlite::Sqlite::restore_from(archive, destination, BackupLimits::default())
                    .unwrap(),
            ))
        }
    }
    fn migrate(
        redb: bool,
        source: &Path,
        destination: &Path,
        plan: &MigrationPlan,
    ) -> rom::Result<Self> {
        if redb {
            rom_redb::Redb::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|db| Self::Redb(Arc::new(db)))
        } else {
            rom_sqlite::Sqlite::migrate_from(source, destination, plan, BackupLimits::default())
                .map(|db| Self::Sqlite(Arc::new(db)))
        }
    }
}
struct Scratch(PathBuf);
impl Scratch {
    fn new(redb: bool) -> Self {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rom-host-maintenance-{}-{redb}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if std::env::var_os("ROM_MAINTENANCE_KEEP_SCRATCH").is_some() {
            eprintln!("maintenance scratch retained: {}", self.0.display());
        } else {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
fn build_runtime(database: &Database, upgraded: bool) -> Runtime {
    let gate = IdentityGate::default()
        .allow_host("host", PrincipalKind::Embedded, "configuration")
        .unwrap();
    let builder = Runtime::builder()
        .actor_gate(Arc::new(gate))
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
        );
    let builder = if upgraded {
        builder.resource(
            After::definition()
                .policy(|a, _, _| a.authority == "local")
                .allow_all_fields()
                .replay_from::<Before>(),
        )
    } else {
        builder.resource(
            Before::definition()
                .policy(|a, _, _| a.authority == "local")
                .allow_all_fields(),
        )
    };
    builder
        .build(database.storage(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
async fn provision(runtime: &Runtime, issuer: &str) {
    let admin = Actor::trusted("host", "configuration");
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
                    issuer: issuer.into(),
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
}
async fn start(
    runtime: &Runtime,
    listener: tokio::net::TcpListener,
    origin: &str,
    issuer: &str,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<rom::Result<()>>,
) {
    let config = HostConfig::new(
        origin,
        "/rom-studio/",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        Actor::trusted("host", "configuration"),
    )
    .allow_loopback_http(true)
    .provider(OidcProviderConfig {
        authority: "local".into(),
        label: "Real test provider".into(),
        issuer: issuer.into(),
        client_id: "studio".into(),
        authorization_endpoint: format!("{issuer}/auth"),
        token_endpoint: format!("{issuer}/token"),
        jwks_endpoint: format!("{issuer}/jwks"),
        client_secret: Some("controlled-host-test-secret".into()),
    });
    let host = StudioHost::new(runtime.clone(), config).unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    (
        stop,
        tokio::spawn(host.serve(listener, async {
            let _ = stopped.await;
        })),
    )
}
async fn stop(
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<rom::Result<()>>,
) {
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
async fn session(browser: &mut Browser, origin: &str) -> serde_json::Value {
    let response = browser
        .send(
            reqwest::Method::GET,
            &format!("{origin}/rom-studio/auth/session"),
            None,
        )
        .await;
    assert_eq!(response.status(), 200);
    serde_json::from_slice(&response.bytes().await.unwrap()).unwrap()
}
async fn invoke(
    browser: &Browser,
    origin: &str,
    csrf: &str,
    invocation: &rom::Invocation,
) -> reqwest::Response {
    browser
        .client
        .post(format!("{origin}/rom-studio/api/invoke"))
        .header("cookie", browser.cookie(origin))
        .header("origin", origin)
        .header("x-rom-csrf", csrf)
        .body(serde_json::to_vec(invocation).unwrap())
        .send()
        .await
        .unwrap()
}
#[tokio::test]
async fn real_provider_backup_migration_reopen_requires_fresh_session_and_current_authority() {
    for redb in [false, true] {
        tokio::time::timeout(std::time::Duration::from_secs(30), journey(redb))
            .await
            .unwrap();
    }
}
async fn journey(redb: bool) {
    let scratch = Scratch::new(redb);
    let source = scratch.path("source");
    let restored_path = scratch.path("restored");
    let destination = scratch.path("migrated");
    let archive = scratch.path("backup");
    let database = Database::open(redb, &source);
    let runtime = build_runtime(&database, false);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let origin = format!("http://{address}");
    let (_provider, issuer) = provider(&format!("{origin}/rom-studio/auth/callback/local"));
    provision(&runtime, &issuer).await;
    let (shutdown, serving) = start(&runtime, listener, &origin, &issuer).await;
    let mut old = Browser::new();
    authorize(&mut old, &origin, &issuer).await;
    let initial_session = session(&mut old, &origin).await;
    assert_eq!(initial_session["authenticated"], true);
    let invocation: rom::Invocation = Command::create("one", Before { done: false })
        .idempotency("stable-record")
        .into();
    let created = invoke(
        &old,
        &origin,
        initial_session["csrf_token"].as_str().unwrap(),
        &invocation,
    )
    .await;
    assert_eq!(created.status(), 200);
    let created: serde_json::Value =
        serde_json::from_slice(&created.bytes().await.unwrap()).unwrap();
    assert_eq!(created["revision"], 1);
    assert_eq!(created["value"]["done"], false);
    let before = database.counts();
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(convert).unwrap(),
    ])
    .unwrap();
    // Actual negative control: maintenance cannot bypass the live native owner.
    assert!(Database::migrate(redb, &source, &destination, &plan).is_err());
    assert!(!destination.exists());
    assert_eq!(database.counts(), before);
    stop(shutdown, serving).await;
    drop(runtime);
    let snapshot = database.backup(&archive);
    assert_eq!(
        snapshot
            .events
            .iter()
            .filter(|(_, row)| row.key.kind == Before::KIND)
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .receipts
            .iter()
            .filter(|receipt| receipt.row.key.kind == Before::KIND)
            .count(),
        1
    );
    drop(database);
    let original_bytes = std::fs::read(&source).unwrap();
    let restored = Database::restore(redb, &archive, &restored_path);
    assert_eq!(restored.counts(), before);
    drop(restored);
    let migrated = Database::migrate(redb, &restored_path, &destination, &plan).unwrap();
    assert_eq!(migrated.counts(), before);
    assert_eq!(std::fs::read(&source).unwrap(), original_bytes);
    drop(migrated);
    // Reopen from disk rather than keep the migration return handle alive.
    let database = Database::open(redb, &destination);
    let reopened_snapshot = database.backup(&scratch.path("migrated-backup"));
    let migrated_events: Vec<_> = reopened_snapshot
        .events
        .iter()
        .filter(|(_, row)| row.key.kind == After::KIND)
        .collect();
    assert_eq!(migrated_events.len(), 1);
    assert_eq!(
        migrated_events[0].1.value,
        Some(rom::json!({"completed": false}))
    );
    let current = build_runtime(&database, true);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let (shutdown, serving) = start(&current, listener, &origin, &issuer).await;
    assert_eq!(session(&mut old, &origin).await["authenticated"], false);
    // Old browser cookie and CSRF must not become authority after reopen.
    let stale = invoke(
        &old,
        &origin,
        initial_session["csrf_token"].as_str().unwrap(),
        &invocation,
    )
    .await;
    assert_eq!(stale.status(), 401);
    assert_eq!(database.counts(), before);
    let mut fresh = Browser::new();
    authorize(&mut fresh, &origin, &issuer).await;
    let fresh_session = session(&mut fresh, &origin).await;
    assert_eq!(fresh_session["authenticated"], true);
    assert_eq!(fresh_session["user_id"], "alice-user");
    assert_ne!(fresh_session["generation"], initial_session["generation"]);
    let replay = invoke(
        &fresh,
        &origin,
        fresh_session["csrf_token"].as_str().unwrap(),
        &invocation,
    )
    .await;
    assert_eq!(replay.status(), 200);
    let replay: serde_json::Value = serde_json::from_slice(&replay.bytes().await.unwrap()).unwrap();
    assert_eq!(replay["revision"], 1);
    assert_eq!(replay["value"], serde_json::json!({"completed": false}));
    assert_eq!(database.counts(), before);
    current
        .execute(
            &Actor::trusted("host", "configuration"),
            Command::replace(
                &link_key("local", PrincipalKind::Human, "alice"),
                IdentityLink {
                    authority: "local".into(),
                    subject: "alice".into(),
                    principal_kind: "human".into(),
                    user_id: "alice-user".into(),
                    enabled: false,
                },
            )
            .at_revision(1)
            .idempotency("revoke-current-link"),
        )
        .await
        .unwrap();
    let revoked_counts = database.counts();
    let denied = invoke(
        &fresh,
        &origin,
        fresh_session["csrf_token"].as_str().unwrap(),
        &invocation,
    )
    .await;
    assert_eq!(denied.status(), 401);
    let denied: serde_json::Value = serde_json::from_slice(&denied.bytes().await.unwrap()).unwrap();
    assert_eq!(denied, serde_json::json!({"error": "denied"}));
    assert_eq!(database.counts(), revoked_counts);
    stop(shutdown, serving).await;
}
