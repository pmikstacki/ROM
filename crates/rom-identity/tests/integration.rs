use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, encode};
#[path = "support/identity_gate_query_profile.rs"]
mod identity_gate_query_profile;
#[path = "support/oidc.rs"]
mod oidc;
use rom::{Actor, Clock, Command, Error, PrincipalKind, Resource, Runtime};
use rom_auth::{
    AuthError,
    jwt::{JwtAdapter, TrustedKeys},
};
use rom_identity::{
    ActivatedIdentity, IdentityGate, IdentityLink, IdentityProvider, ProviderActivation,
    ProviderProfile, User, link_key,
};
use rom_sqlite::Sqlite;
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command as Process, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
const NOW: u64 = 1_800_000_000;
struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
struct Keys {
    private: Vec<u8>,
    public: Vec<u8>,
}
fn keys() -> &'static Keys {
    static KEYS: OnceLock<Keys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let output = Process::new("openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
            ])
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
        let private = output.stdout;
        let mut child = Process::new("openssl")
            .args(["pkey", "-pubout"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&private).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        Keys {
            private,
            public: output.stdout,
        }
    })
}
struct Source;
impl TrustedKeys for Source {
    fn fetch(&mut self) -> Result<BTreeMap<String, DecodingKey>, AuthError> {
        Ok(BTreeMap::from([(
            "fixture".into(),
            DecodingKey::from_rsa_pem(&keys().public).unwrap(),
        )]))
    }
}
fn proof(a: &ProviderActivation, subject: &str) -> ActivatedIdentity {
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("fixture".into());
    h.typ = Some("at+jwt".into());
    let c = rom::json!({"iss":a.config().issuer,"aud":a.config().audience,"sub":subject,"iat":NOW,"exp":NOW+600,"jti":"fixture","client_id":"client","principal_kind":"human","email":"ignored@example.invalid"});
    let token = encode(&h, &c, &EncodingKey::from_rsa_pem(&keys().private).unwrap()).unwrap();
    a.verify(|authority, config| {
        JwtAdapter::configured(authority, &config.issuer, &config.audience, Source)?
            .authenticate(&token, NOW)
    })
    .unwrap()
}
fn admin() -> Actor {
    Actor::trusted("host", "bootstrap")
}
fn admin_policy(a: &Actor, _: rom::Access, _: &User) -> bool {
    a == &admin()
}
fn provider() -> IdentityProvider {
    IdentityProvider {
        enabled: true,
        profile: ProviderProfile::JwtRs256Human,
        issuer: "https://issuer.example".into(),
        audience: "rom".into(),
        endpoint: Some("host-key-source".into()),
        credential_ref: None,
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "documents")]
struct Document {
    body: String,
}
type Pause = (
    tokio::sync::oneshot::Sender<()>,
    std::sync::mpsc::Receiver<()>,
);
static PAUSE: std::sync::Mutex<Option<Pause>> = std::sync::Mutex::new(None);
fn slow(document: &mut Document, _: ()) -> rom::Result<Vec<rom::Intent>> {
    let (entered, release) = PAUSE.lock().unwrap().take().unwrap();
    let _ = entered.send(());
    release.recv().unwrap();
    document.body = "changed".into();
    Ok(vec![])
}
const SLOW: rom::Action<Document, ()> = rom::Action::new("slow", slow);
async fn setup() -> (Runtime, Arc<Time>, ProviderActivation) {
    let clock = Arc::new(Time(AtomicU64::new(NOW)));
    let gate = IdentityGate::default()
        .allow_host("host", PrincipalKind::Embedded, "bootstrap")
        .unwrap();
    let r = Runtime::builder()
        .clock(clock.clone())
        .actor_gate(Arc::new(gate))
        .resource(User::definition().policy(admin_policy).allow_all_fields())
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| a == &admin())
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| a == &admin())
                .allow_all_fields(),
        )
        .resource(
            Document::definition()
                .policy(|a, _, _| a == &admin() || a.subject == "subject")
                .allow_all_fields()
                .action(SLOW),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &admin(),
        Command::create(
            "user",
            User {
                enabled: true,
                display_name: "Profile".into(),
            },
        )
        .idempotency("user"),
    )
    .await
    .unwrap();
    r.execute(
        &admin(),
        Command::create("provider", provider()).idempotency("provider"),
    )
    .await
    .unwrap();
    r.execute(
        &admin(),
        Command::create(
            &link_key("provider", PrincipalKind::Human, "subject"),
            IdentityLink {
                authority: "provider".into(),
                principal_kind: "human".into(),
                subject: "subject".into(),
                user_id: "user".into(),
                enabled: true,
            },
        )
        .idempotency("link"),
    )
    .await
    .unwrap();
    let activation = ProviderActivation::read(&r, &admin(), "provider")
        .await
        .unwrap();
    (r, clock, activation)
}
#[tokio::test]
async fn real_signed_proof_links_through_native_resources_and_expiry_is_preserved() {
    let (r, clock, a) = setup().await;
    let p = proof(&a, "subject");
    let actor = p.bind(&r).await.unwrap();
    assert_eq!(actor.principal_kind(), PrincipalKind::Human);
    assert_eq!(actor.valid_until(), Some(NOW + 30));
    assert_eq!(
        rom_identity::linked_user_id(&actor).as_deref(),
        Some("user")
    );
    r.execute(
        &actor,
        Command::create(
            "d",
            Document {
                body: "protected".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    assert!(matches!(
        proof(&a, "unlinked").bind(&r).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        r.read::<Document>(
            &Actor::trusted("provider", "subject").with_kind(PrincipalKind::Human),
            "d"
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        r.read::<Document>(
            &Actor::trusted("provider", "subject").with_kind(PrincipalKind::Service),
            "d"
        )
        .await,
        Err(Error::Denied)
    ));
    clock.0.store(NOW + 30, Ordering::SeqCst);
    assert!(matches!(
        r.read::<Document>(&actor, "d").await,
        Err(Error::Denied)
    ));
    assert!(matches!(p.bind(&r).await, Err(Error::Denied)));
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn user_disable_during_action_proposal_prevents_commit() {
    let (r, _, a) = setup().await;
    let actor = proof(&a, "subject").bind(&r).await.unwrap();
    r.execute(
        &actor,
        Command::create(
            "d",
            Document {
                body: "protected".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    *PAUSE.lock().unwrap() = Some((entered, wait));
    let runtime = r.clone();
    let work = tokio::spawn(async move {
        runtime
            .execute(
                &actor,
                Command::action("d", SLOW, ())
                    .at_revision(1)
                    .idempotency("slow"),
            )
            .await
    });
    waiting.await.unwrap();
    r.execute(
        &admin(),
        Command::replace(
            "user",
            User {
                enabled: false,
                display_name: "Profile".into(),
            },
        )
        .at_revision(1)
        .idempotency("disable"),
    )
    .await
    .unwrap();
    release.send(()).unwrap();
    assert!(matches!(work.await.unwrap(), Err(Error::Denied)));
    let row = r.read::<Document>(&admin(), "d").await.unwrap();
    assert_eq!(row.revision, 1);
    assert_eq!(row.value.unwrap().body, "protected");
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn authority_links_and_provider_config_are_validated_and_bootstrap_is_explicit() {
    let (r, _, a) = setup().await;
    assert!(
        IdentityGate::default()
            .allow_host("host", PrincipalKind::Human, "admin")
            .is_err()
    );
    assert!(matches!(
        r.read::<User>(&Actor::trusted("host", "first-caller"), "user")
            .await,
        Err(Error::Denied)
    ));
    assert_ne!(
        link_key("provider", PrincipalKind::Human, "subject"),
        link_key("provider", PrincipalKind::Service, "subject")
    );
    r.execute(
        &admin(),
        Command::create("other-provider", provider()).idempotency("other"),
    )
    .await
    .unwrap();
    let other = ProviderActivation::read(&r, &admin(), "other-provider")
        .await
        .unwrap();
    assert!(matches!(
        proof(&other, "subject").bind(&r).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        a.verify(|_, _| Err(AuthError::Unavailable)),
        Err(Error::Denied)
    ));
    let mut invalid = provider().encode();
    invalid["profile"] = rom::json!("unverified-anything");
    let invocation = rom::Invocation {
        retry_epoch: 0,
        kind: IdentityProvider::KIND.into(),
        id: "invalid".into(),
        expected: None,
        idempotency: "invalid".into(),
        operation: rom::Operation::Create(invalid),
    };
    assert!(matches!(
        r.invoke(&admin(), invocation).await,
        Err(Error::Invalid { .. })
    ));
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn disabled_user_denies_cached_retry_and_live_without_repeating_commit() {
    let (r, _, a) = setup().await;
    let actor = proof(&a, "subject").bind(&r).await.unwrap();
    r.execute(
        &actor,
        Command::create(
            "d",
            Document {
                body: "protected".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    let mut live = r
        .live_projected(&actor, "documents", "body", rom::json!("protected"))
        .await
        .unwrap();
    r.execute(
        &admin(),
        Command::replace(
            "user",
            User {
                enabled: false,
                display_name: "Profile".into(),
            },
        )
        .at_revision(1)
        .idempotency("disable"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.execute(
            &actor,
            Command::create(
                "d",
                Document {
                    body: "protected".into()
                }
            )
            .idempotency("create")
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(live.changed().await, Err(Error::Denied)));
    assert_eq!(r.read::<Document>(&admin(), "d").await.unwrap().revision, 1);
    r.execute(
        &admin(),
        Command::replace(
            "user",
            User {
                enabled: true,
                display_name: "Profile".into(),
            },
        )
        .at_revision(2)
        .idempotency("enable"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.read::<Document>(&actor, "d").await,
        Err(Error::Denied)
    ));
    assert!(proof(&a, "subject").bind(&r).await.is_ok());
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn provider_revision_and_unlink_relink_cannot_revive_old_bindings() {
    let (r, _, a) = setup().await;
    let p = proof(&a, "subject");
    let actor = p.bind(&r).await.unwrap();
    let mut disabled = provider();
    disabled.enabled = false;
    r.execute(
        &admin(),
        Command::replace("provider", disabled)
            .at_revision(1)
            .idempotency("disable"),
    )
    .await
    .unwrap();
    assert!(matches!(p.bind(&r).await, Err(Error::Denied)));
    r.execute(
        &admin(),
        Command::replace("provider", provider())
            .at_revision(2)
            .idempotency("enable"),
    )
    .await
    .unwrap();
    assert!(matches!(p.bind(&r).await, Err(Error::Denied)));
    assert!(matches!(
        r.query::<Document>(&actor, &Document::body_field().equals("x".into()))
            .await,
        Err(Error::Denied)
    ));
    let current = ProviderActivation::read(&r, &admin(), "provider")
        .await
        .unwrap();
    let fresh = proof(&current, "subject").bind(&r).await.unwrap();
    let id = link_key("provider", PrincipalKind::Human, "subject");
    let old = r
        .read::<IdentityLink>(&admin(), &id)
        .await
        .unwrap()
        .value
        .unwrap();
    let mut off = old.clone();
    off.enabled = false;
    r.execute(
        &admin(),
        Command::replace(&id, off)
            .at_revision(1)
            .idempotency("unlink"),
    )
    .await
    .unwrap();
    r.execute(
        &admin(),
        Command::replace(&id, old)
            .at_revision(2)
            .idempotency("relink"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.query::<Document>(&fresh, &Document::body_field().equals("x".into()))
            .await,
        Err(Error::Denied)
    ));
    assert!(proof(&current, "subject").bind(&r).await.is_ok());
    r.shutdown().await.unwrap();
}
