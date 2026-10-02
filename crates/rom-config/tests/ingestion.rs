use rom::{Access, Actor, Clock, Command, Error, PrincipalKind, Resource, Runtime, Storage};
use rom_config::{Format, REQUEST_RELOAD, ReloadError, ReloadTicket, SourceActivation};
use rom_identity::{IdentityProvider, User};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Debug, Resource)]
#[resource(name = "application-settings")]
struct Settings {
    name: String,
    enabled: bool,
    count: u64,
    note: Option<String>,
    tags: Vec<String>,
}
const FIRST: &str = r#"{"name":"first","enabled":false,"count":0,"note":null,"tags":["one"]}"#;
const SECOND: &str = r#"{"name":"second","enabled":true,"count":1,"note":"set","tags":["two"]}"#;
struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
fn admin() -> Actor {
    Actor::trusted("host", "admin")
}
fn worker(subject: &str) -> Actor {
    Actor::trusted("host", subject).with_kind(PrincipalKind::Service)
}
fn is_admin(a: &Actor) -> bool {
    a.authority == "host" && a.subject == "admin" && a.principal_kind() == PrincipalKind::Embedded
}
fn permitted(a: &Actor, subject: &str) -> bool {
    is_admin(a)
        || (a.authority == "host"
            && a.subject == subject
            && a.principal_kind() == PrincipalKind::Service)
}
fn assigned(source: &SourceActivation) -> &str {
    match source.target_kind.as_str() {
        "users" => "user-loader",
        "identity-providers" => "trust-loader",
        _ => "settings-loader",
    }
}
fn source_policy(a: &Actor, _: Access, s: &SourceActivation) -> bool {
    permitted(a, assigned(s))
}
fn source_fields(a: &Actor, access: Access, field: &str, s: &SourceActivation) -> bool {
    is_admin(a)
        || (permitted(a, assigned(s))
            && (matches!(access, Access::Read)
                || matches!(
                    field,
                    "requested_generation"
                        | "source_version"
                        | "target_revision"
                        | "target_present"
                )))
}
fn build(storage: Arc<dyn Storage>, clock: Arc<Time>) -> Runtime {
    Runtime::builder()
        .clock(clock)
        .resource(
            SourceActivation::definition()
                .policy(source_policy)
                .field_policy(source_fields)
                .query_policy(|_, _| false)
                .action(REQUEST_RELOAD),
        )
        .resource(
            Settings::definition()
                .policy(|a, _, _| permitted(a, "settings-loader"))
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(|a| permitted(a, "settings-loader")),
        )
        .resource(
            User::definition()
                .policy(|a, _, _| permitted(a, "user-loader"))
                .allow_all_fields()
                .source_owner("user-seed")
                .source_metadata_policy(|a| permitted(a, "user-loader")),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| permitted(a, "trust-loader"))
                .allow_all_fields()
                .source_owner("trust-seed")
                .source_metadata_policy(|a| permitted(a, "trust-loader")),
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
fn source(kind: &str, id: &str) -> SourceActivation {
    SourceActivation {
        worker_authority: "host".into(),
        worker_subject: match kind {
            "users" => "user-loader",
            "identity-providers" => "trust-loader",
            _ => "settings-loader",
        }
        .into(),
        enabled: true,
        target_kind: kind.into(),
        target_id: id.into(),
        requested_generation: 0,
        source_version: "unloaded".into(),
        target_revision: None,
        target_present: false,
        valid_until: 200,
    }
}
async fn initialize(r: &Runtime) {
    for (id, kind, target) in [
        ("deployment", Settings::KIND, "settings.example[0]"),
        ("user-seed", User::KIND, "user.example[0]"),
        ("trust-seed", IdentityProvider::KIND, "idp"),
    ] {
        r.execute(
            &admin(),
            Command::create(id, source(kind, target)).idempotency(id),
        )
        .await
        .unwrap();
    }
}
fn clock() -> Arc<Time> {
    Arc::new(Time(AtomicU64::new(100)))
}

#[derive(Clone)]
struct BadDiagnostic(String);
impl rom::Field for BadDiagnostic {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> rom::Value {
        rom::json!(self.0)
    }
    fn decode(value: rom::Value) -> rom::Result<Self> {
        Err(Error::Unsupported(
            value.as_str().unwrap_or("invalid").into(),
        ))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "custom-diagnostics")]
struct Custom {
    value: BadDiagnostic,
}
#[tokio::test]
async fn target_codec_diagnostics_are_redacted_at_ingestion_boundary() {
    let r = Runtime::builder()
        .clock(clock())
        .resource(
            SourceActivation::definition()
                .policy(source_policy)
                .field_policy(source_fields)
                .action(REQUEST_RELOAD),
        )
        .resource(
            Custom::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(|_| true),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &admin(),
        Command::create("deployment", source(Custom::KIND, "custom")).idempotency("source"),
    )
    .await
    .unwrap();
    let ticket = ReloadTicket::request(&r, &worker("settings-loader"), "deployment", "v1")
        .await
        .unwrap();
    let error = ticket
        .load(
            &r,
            Format::Json,
            r#"{"value":"secret-that-custom-codec-leaks"}"#,
            "deployment",
        )
        .await
        .unwrap_err();
    assert!(!format!("{error:?} {error}").contains("secret-that-custom-codec-leaks"));
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_new_request_preserves_last_valid_and_supersedes_old_completion() {
    let r = build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        clock(),
    );
    initialize(&r).await;
    let loader = worker("settings-loader");
    let first = ReloadTicket::request(&r, &loader, "deployment", "v1")
        .await
        .unwrap();
    first
        .load(&r, Format::Json, FIRST, "deployment")
        .await
        .unwrap();
    let stale = ReloadTicket::request(&r, &loader, "deployment", "v2")
        .await
        .unwrap();
    let failed = ReloadTicket::request(&r, &loader, "deployment", "v3")
        .await
        .unwrap();
    assert!(matches!(
        failed
            .load(
                &r,
                Format::Json,
                r#"{"name":"sensitive-value","enabled":"wrong","count":0,"note":null,"tags":[]}"#,
                "deployment"
            )
            .await,
        Err(ReloadError::Runtime(Error::Invalid { .. }))
    ));
    assert!(matches!(
        stale.load(&r, Format::Json, SECOND, "deployment").await,
        Err(ReloadError::Runtime(Error::Conflict))
    ));
    let row = r
        .read::<Settings>(&loader, "settings.example[0]")
        .await
        .unwrap();
    assert_eq!(row.revision, 1);
    assert_eq!(row.value.unwrap().name, "first");
    let state = r
        .source_state(&loader, Settings::KIND, "settings.example[0]")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.provenance.unwrap().generation, 1);
    assert_eq!(
        r.read::<SourceActivation>(&loader, "deployment")
            .await
            .unwrap()
            .value
            .unwrap()
            .requested_generation,
        3
    );
    assert!(matches!(
        r.execute(
            &loader,
            Command::replace("deployment", source(IdentityProvider::KIND, "idp"))
                .at_revision(4)
                .idempotency("self-grant")
        )
        .await,
        Err(Error::Denied)
    ));
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_users_and_trust_configuration_use_the_same_acceptance_pipeline() {
    let r = build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        clock(),
    );
    initialize(&r).await;
    assert!(matches!(
        ReloadTicket::request(&r, &worker("settings-loader"), "trust-seed", "v1").await,
        Err(Error::Denied)
    ));
    let user = ReloadTicket::request(&r, &worker("user-loader"), "user-seed", "v1")
        .await
        .unwrap();
    user.load(
        &r,
        Format::Toml,
        "enabled=true\ndisplay_name='Seeded profile'\n",
        "user-seed",
    )
    .await
    .unwrap();
    assert_eq!(
        r.read::<User>(&admin(), "user.example[0]")
            .await
            .unwrap()
            .value
            .unwrap()
            .display_name,
        "Seeded profile"
    );
    let trust = ReloadTicket::request(&r, &worker("trust-loader"), "trust-seed", "v1")
        .await
        .unwrap();
    let config = r#"{"enabled":true,"profile":"jwt-rs256-human","issuer":"https://issuer.example","audience":"rom","endpoint":null,"credential_ref":"vault-entry"}"#;
    trust
        .load(&r, Format::Json, config, "trust-seed")
        .await
        .unwrap();
    let invalid = ReloadTicket::request(&r, &worker("trust-loader"), "trust-seed", "v2")
        .await
        .unwrap();
    assert!(
        invalid
            .load(
                &r,
                Format::Json,
                &config.replace("jwt-rs256-human", "anything"),
                "trust-seed"
            )
            .await
            .is_err()
    );
    assert_eq!(
        r.read::<IdentityProvider>(&admin(), "idp")
            .await
            .unwrap()
            .revision,
        1
    );
    let bad = ReloadTicket::request(&r, &worker("user-loader"), "user-seed", "v2")
        .await
        .unwrap();
    assert!(
        bad.load(
            &r,
            Format::Json,
            r#"{"enabled":true,"display_name":"x","administrator":true}"#,
            "user-seed"
        )
        .await
        .is_err()
    );
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn disabled_then_reenabled_source_does_not_restore_old_ticket_authority() {
    let r = build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        clock(),
    );
    initialize(&r).await;
    let loader = worker("settings-loader");
    let ticket = ReloadTicket::request(&r, &loader, "deployment", "v1")
        .await
        .unwrap();
    let current = r
        .read::<SourceActivation>(&admin(), "deployment")
        .await
        .unwrap();
    let mut disabled = current.value.unwrap();
    disabled.enabled = false;
    r.execute(
        &admin(),
        Command::replace("deployment", disabled.clone())
            .at_revision(current.revision)
            .idempotency("disable"),
    )
    .await
    .unwrap();
    assert!(matches!(
        ticket.load(&r, Format::Json, FIRST, "deployment").await,
        Err(ReloadError::Runtime(Error::Conflict))
    ));
    assert!(matches!(
        ReloadTicket::request(&r, &loader, "deployment", "v2").await,
        Err(Error::Denied)
    ));
    disabled.enabled = true;
    r.execute(
        &admin(),
        Command::replace("deployment", disabled)
            .at_revision(current.revision + 1)
            .idempotency("enable"),
    )
    .await
    .unwrap();
    assert!(matches!(
        ticket.load(&r, Format::Json, FIRST, "deployment").await,
        Err(ReloadError::Runtime(Error::Conflict))
    ));
    ReloadTicket::request(&r, &loader, "deployment", "v2")
        .await
        .unwrap()
        .load(&r, Format::Json, FIRST, "deployment")
        .await
        .unwrap();
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn reading_activation_does_not_grant_its_worker_identity() {
    let r = Runtime::builder()
        .clock(clock())
        .resource(
            SourceActivation::definition()
                .policy(|a, access, s| {
                    matches!(access, Access::Read) || source_policy(a, access, s)
                })
                .field_policy(|a, access, field, s| {
                    matches!(access, Access::Read) || source_fields(a, access, field, s)
                })
                .action(REQUEST_RELOAD),
        )
        .resource(
            Settings::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(|_| true),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &admin(),
        Command::create("deployment", source(Settings::KIND, "settings")).idempotency("create"),
    )
    .await
    .unwrap();
    ReloadTicket::request(&r, &worker("settings-loader"), "deployment", "v1")
        .await
        .unwrap();
    assert!(matches!(
        ReloadTicket::resume(&r, &worker("someone-else"), "deployment").await,
        Err(Error::Denied)
    ));
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn metadata_only_reload_expiry_and_explicit_delete_have_distinct_semantics() {
    let time = clock();
    let r = build(
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        time.clone(),
    );
    initialize(&r).await;
    let loader = worker("settings-loader");
    ReloadTicket::request(&r, &loader, "deployment", "v1")
        .await
        .unwrap()
        .load(&r, Format::Json, FIRST, "deployment")
        .await
        .unwrap();
    let second = ReloadTicket::request(&r, &loader, "deployment", "v2")
        .await
        .unwrap();
    let outcome = second
        .load(&r, Format::Json, FIRST, "deployment")
        .await
        .unwrap();
    assert_eq!(outcome.revision, 2);
    assert_eq!(
        second
            .load(&r, Format::Json, FIRST, "deployment")
            .await
            .unwrap()
            .revision,
        2
    );
    let expired = ReloadTicket::request(&r, &loader, "deployment", "v3")
        .await
        .unwrap();
    time.0.store(200, Ordering::SeqCst);
    assert!(matches!(
        expired.load(&r, Format::Json, SECOND, "deployment").await,
        Err(ReloadError::Runtime(Error::Denied))
    ));
    // Accepted passive Resource state remains available; source permit TTL is not data TTL.
    assert_eq!(
        r.read::<Settings>(&loader, "settings.example[0]")
            .await
            .unwrap()
            .revision,
        2
    );
    time.0.store(100, Ordering::SeqCst);
    let deletion = ReloadTicket::request(&r, &loader, "deployment", "v4")
        .await
        .unwrap();
    assert!(deletion.delete(&r).await.unwrap().value.is_none());
    let recreate = ReloadTicket::request(&r, &loader, "deployment", "v5")
        .await
        .unwrap();
    assert_eq!(
        recreate
            .load(&r, Format::Json, FIRST, "deployment")
            .await
            .unwrap()
            .revision,
        4
    );
    r.shutdown().await.unwrap();
}

struct Opened {
    storage: Arc<dyn Storage>,
    arm: Box<dyn Fn(Option<usize>)>,
}
fn open(kind: &str, path: &Path) -> Opened {
    if kind == "sqlite" {
        let s = Arc::new(rom_sqlite::Sqlite::open(path).unwrap());
        let control = s.clone();
        Opened {
            storage: s,
            arm: Box::new(move |at| {
                control.on_commit(at.map(|point| {
                    Arc::new(move |actual| {
                        if actual == point {
                            Err(if point == usize::MAX {
                                Error::Unknown
                            } else {
                                Error::Storage
                            })
                        } else {
                            Ok(())
                        }
                    }) as _
                }))
            }),
        }
    } else {
        let s = Arc::new(rom_redb::Redb::open(path).unwrap());
        let control = s.clone();
        Opened {
            storage: s,
            arm: Box::new(move |at| {
                control.on_commit(at.map(|point| {
                    Arc::new(move |actual| {
                        if actual == point {
                            Err(if point == usize::MAX {
                                Error::Unknown
                            } else {
                                Error::Storage
                            })
                        } else {
                            Ok(())
                        }
                    }) as _
                }))
            }),
        }
    }
}
fn path(kind: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "rom-config-{}-{}-{kind}.db",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ))
}
#[tokio::test]
async fn both_adapters_recover_atomic_provenance_and_original_identity_after_lost_ack() {
    for kind in ["sqlite", "redb"] {
        let path = path(kind);
        let opened = open(kind, &path);
        let r = build(opened.storage.clone(), clock());
        initialize(&r).await;
        let loader = worker("settings-loader");
        ReloadTicket::request(&r, &loader, "deployment", "v1")
            .await
            .unwrap()
            .load(&r, Format::Json, FIRST, "deployment")
            .await
            .unwrap();
        let ticket = ReloadTicket::request(&r, &loader, "deployment", "v2")
            .await
            .unwrap();
        (opened.arm)(Some(1));
        assert!(
            ticket
                .load(&r, Format::Json, SECOND, "deployment")
                .await
                .is_err()
        );
        (opened.arm)(None);
        let before = r
            .source_state(&loader, Settings::KIND, "settings.example[0]")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(before.revision, 1);
        assert_eq!(before.provenance.unwrap().generation, 1);
        (opened.arm)(Some(usize::MAX));
        assert!(matches!(
            ticket.load(&r, Format::Json, SECOND, "deployment").await,
            Err(ReloadError::Runtime(Error::Unknown))
        ));
        (opened.arm)(None);
        r.shutdown().await.unwrap();
        drop(r);
        drop(opened);
        let reopened = open(kind, &path);
        let r = build(reopened.storage.clone(), clock());
        let resumed = ReloadTicket::resume(&r, &loader, "deployment")
            .await
            .unwrap();
        assert_eq!(
            resumed
                .load(&r, Format::Json, SECOND, "deployment")
                .await
                .unwrap()
                .revision,
            2
        );
        let state = r
            .source_state(&loader, Settings::KIND, "settings.example[0]")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.provenance.unwrap().generation, 2);
        assert_eq!(
            r.read::<SourceActivation>(&loader, "deployment")
                .await
                .unwrap()
                .value
                .unwrap()
                .requested_generation,
            2
        );
        let events = reopened
            .storage
            .journal(Settings::KIND, None, 100, 1_000_000)
            .unwrap();
        assert_eq!(events.events.len(), 2);
        r.shutdown().await.unwrap();
        drop(r);
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
}
