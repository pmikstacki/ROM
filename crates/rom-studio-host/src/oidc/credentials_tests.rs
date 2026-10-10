//! Original credentials remain usable across a queued proof-renewal boundary.
use super::{Cached, Credentials, PinnedKeys, parse_jwks};
use crate::{HostConfig, OidcProviderConfig, StudioHost, session::SessionEvidence};
use rom::{Actor, Clock, Command, Error, PrincipalKind, Resource, Runtime};
use rom_auth::{OidcIdTokenAdapter, oidc::OidcTokenBindings};
use rom_identity::{
    IdentityGate, IdentityLink, IdentityProvider, ProviderActivation, ProviderProfile, User,
    link_key,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "credentials_tests/bind_barrier.rs"]
pub(super) mod bind_barrier;

pub(super) struct Time(pub(super) AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
pub(super) struct Fixture {
    pub(super) host: StudioHost,
    pub(super) clock: Arc<Time>,
    pub(super) credentials: Arc<Credentials>,
    pub(super) cookie: String,
    pub(super) requests: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn new(jwks_status: u16) -> Self {
        Self::with_token(jwks_status, false).await
    }
    async fn with_token(jwks_status: u16, corrupt_signature: bool) -> Self {
        Self::with_storage(
            jwks_status,
            corrupt_signature,
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        )
        .await
    }
    async fn with_storage(
        jwks_status: u16,
        corrupt_signature: bool,
        storage: Arc<dyn rom::Storage>,
    ) -> Self {
        Self::with_storage_and_clocks(jwks_status, corrupt_signature, storage, None).await
    }
    async fn with_storage_and_clocks(
        jwks_status: u16,
        corrupt_signature: bool,
        storage: Arc<dyn rom::Storage>,
        clocks: Option<(Arc<Time>, Arc<dyn Clock>)>,
    ) -> Self {
        Self::with_diagnostics(jwks_status, corrupt_signature, storage, clocks, false).await
    }
    pub(super) async fn observed(jwks_status: u16, corrupt_signature: bool) -> Self {
        Self::with_diagnostics(
            jwks_status,
            corrupt_signature,
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            None,
            true,
        )
        .await
    }
    pub(super) async fn with_diagnostics(
        jwks_status: u16,
        corrupt_signature: bool,
        storage: Arc<dyn rom::Storage>,
        clocks: Option<(Arc<Time>, Arc<dyn Clock>)>,
        diagnostics: bool,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let issuer = format!("http://{address}");
        // Same Node RSA signing backend as the existing maintained human-provider fixture.
        let output = std::process::Command::new("node").args(["--input-type=module", "-e", r#"
import {generateKeyPairSync,sign} from 'node:crypto';
const {privateKey,publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
const header=Buffer.from(JSON.stringify({alg:'RS256',kid:'fixture',typ:'JWT'})).toString('base64url');
const claims=Buffer.from(JSON.stringify({iss:process.argv[1],aud:'studio',sub:'alice',iat:100,exp:400,nonce:'fixture-nonce'})).toString('base64url');
const input=header+'.'+claims;
process.stdout.write(JSON.stringify({token:input+'.'+sign('RSA-SHA256',Buffer.from(input),privateKey).toString('base64url'),jwks:{keys:[{...publicKey.export({format:'jwk'}),kid:'fixture',alg:'RS256',use:'sig'}]}}));
"#]).arg(&issuer).stderr(std::process::Stdio::null()).output().unwrap();
        assert!(output.status.success());
        assert!(output.stdout.len() <= 16384);
        let signed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let token = signed["token"].as_str().unwrap().to_owned();
        let jwks = serde_json::to_vec(&signed["jwks"]).unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let observed = requests.clone();
        let body = jwks.clone();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = [0_u8; 4096];
                assert!(socket.read(&mut bytes).await.unwrap() > 0);
                observed.fetch_add(1, Ordering::SeqCst);
                socket.write_all(format!("HTTP/1.1 {jwks_status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
        });
        let (clock, host_clock): (Arc<Time>, Arc<dyn Clock>) = clocks.unwrap_or_else(|| {
            let clock = Arc::new(Time(AtomicU64::new(100)));
            (clock.clone(), clock)
        });
        let admin = Actor::trusted("host", "bootstrap");
        let runtime = Runtime::builder()
            .clock(clock.clone())
            .actor_gate(Arc::new(
                IdentityGate::default()
                    .allow_host("host", PrincipalKind::Embedded, "bootstrap")
                    .unwrap()
                    .allow_host("rom-blob-host", PrincipalKind::Service, "attachments")
                    .unwrap(),
            ))
            .resource(rom_blob::definition())
            .resource(
                super::proof_handoff_downstream_tests::Document::definition()
                    .policy(|actor, _, _| actor.authority == "fixture")
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
            .build(storage, Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
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
                .idempotency("fixture-user"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &admin,
                Command::create(
                    "fixture",
                    IdentityProvider {
                        enabled: true,
                        profile: ProviderProfile::OidcRs256Human,
                        issuer: issuer.clone(),
                        audience: "studio".into(),
                        endpoint: None,
                        credential_ref: None,
                    },
                )
                .idempotency("fixture-provider"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &admin,
                Command::create(
                    &link_key("fixture", PrincipalKind::Human, "alice"),
                    IdentityLink {
                        authority: "fixture".into(),
                        subject: "alice".into(),
                        principal_kind: "human".into(),
                        user_id: "alice-user".into(),
                        enabled: true,
                    },
                )
                .idempotency("fixture-link"),
            )
            .await
            .unwrap();
        let activation = ProviderActivation::read(&runtime, &admin, "fixture")
            .await
            .unwrap();
        let proof = activation
            .verify(|authority, provider| {
                OidcIdTokenAdapter::configured(
                    authority,
                    &provider.issuer,
                    &provider.audience,
                    PinnedKeys(parse_jwks(&jwks).unwrap()),
                )?
                .authenticate(
                    &token,
                    "fixture-nonce",
                    OidcTokenBindings {
                        access_token: Some("fixture-access"),
                        authorization_code: Some("fixture-code"),
                    },
                    100,
                )
            })
            .unwrap();
        let actor = proof.bind(&runtime).await.unwrap();
        assert_eq!(actor.valid_until(), Some(130));
        // Corrupt only the retained signature after issuing the genuine initial proof.
        let token = if corrupt_signature {
            let (body, signature) = token.rsplit_once('.').unwrap();
            let mut changed = signature.as_bytes().to_vec();
            changed[0] = if changed[0] == b'A' { b'B' } else { b'A' };
            format!("{body}.{}", String::from_utf8(changed).unwrap())
        } else {
            token
        };
        let credentials = Arc::new(Credentials {
            activation,
            token,
            nonce: "fixture-nonce".into(),
            access_token: "fixture-access".into(),
            code: "fixture-code".into(),
            expiry: 400,
            cached: tokio::sync::Mutex::new(Cached { proof, until: 130 }),
        });
        let config = HostConfig::new(
            "http://127.0.0.1:43210",
            "/rom-studio/",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
            admin,
        )
        .authentication_diagnostics(diagnostics)
        .clock(host_clock)
        .allow_loopback_http(true)
        .provider(OidcProviderConfig {
            authority: "fixture".into(),
            label: "Fixture".into(),
            issuer: issuer.clone(),
            client_id: "studio".into(),
            authorization_endpoint: format!("http://{address}/authorize"),
            token_endpoint: format!("http://{address}/token"),
            jwks_endpoint: format!("http://{address}/jwks"),
            client_secret: None,
        });
        let host = StudioHost::new(runtime, config).unwrap();
        let session = host
            .shared
            .sessions
            .insert(
                SessionEvidence {
                    actor,
                    user_id: "alice-user".into(),
                    token_expiry: 400,
                    credentials: Some(credentials.clone()),
                },
                100,
            )
            .unwrap();
        let cookie = session.cookie().to_owned();
        Self {
            host,
            clock,
            credentials,
            cookie,
            requests,
            server,
        }
    }
    pub(super) async fn close(self) {
        self.server.abort();
        assert!(self.server.await.unwrap_err().is_cancelled());
        self.host.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn queued_original_credential_refreshes_at_exclusive_proof_expiry_without_losing_session() {
    let fixture = Fixture::new(200).await;
    fixture.clock.0.store(129, Ordering::SeqCst);
    let held = fixture.credentials.cached.lock().await;
    let mut request = Box::pin(fixture.credentials.actor(&fixture.host.shared));
    // This actual future reaches the held-cache wait at 129, without sleeps.
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(request.as_mut(), &mut context).is_pending());
    fixture.clock.0.store(130, Ordering::SeqCst);
    drop(held);
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), request)
        .await
        .unwrap();
    if let Err(error) = &outcome {
        fixture.host.shared.sessions.failed(&fixture.cookie, error);
    }
    let session_survived = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 130)
        .is_some();
    let refreshes = fixture.requests.load(Ordering::SeqCst);
    let result = outcome.map(|actor| actor.valid_until());
    let expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert_eq!(
        result,
        Ok(Some(160)),
        "queued valid original token must be reverified after mutex wait"
    );
    assert!(
        session_survived,
        "a temporary cached-proof boundary must not revoke the session"
    );
    assert_eq!(refreshes, 1);
    assert_eq!(expiry, 400);
}

#[tokio::test]
async fn original_expiry_and_revoked_link_still_deny_and_remove_session() {
    let fixture = Fixture::new(200).await;
    fixture.clock.0.store(400, Ordering::SeqCst);
    assert!(matches!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await,
        Err(Error::Denied)
    ));
    assert!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 399)
            .is_none()
    );
    fixture.close().await;
    let fixture = Fixture::new(200).await;
    let admin = Actor::trusted("host", "bootstrap");
    fixture
        .host
        .shared
        .runtime
        .execute(
            &admin,
            Command::replace(
                &link_key("fixture", PrincipalKind::Human, "alice"),
                IdentityLink {
                    authority: "fixture".into(),
                    subject: "alice".into(),
                    principal_kind: "human".into(),
                    user_id: "alice-user".into(),
                    enabled: false,
                },
            )
            .at_revision(1)
            .idempotency("fixture-revoke-link"),
        )
        .await
        .unwrap();
    assert!(matches!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await,
        Err(Error::Denied)
    ));
    assert!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 100)
            .is_none()
    );
    fixture.close().await;
}

#[tokio::test]
async fn temporary_refresh_overload_retains_session_and_original_expiry() {
    let fixture = Fixture::new(503).await;
    fixture.clock.0.store(130, Ordering::SeqCst);
    let result = crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await;
    assert!(matches!(result, Err(Error::Overloaded)));
    assert_eq!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 130)
            .unwrap()
            .expires_at(),
        400
    );
    assert_eq!(fixture.credentials.expiry, 400);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
    fixture.close().await;
}

#[tokio::test]
async fn queued_original_token_expiry_refuses_without_key_fetch_or_session_extension() {
    let fixture = Fixture::new(200).await;
    fixture.clock.0.store(129, Ordering::SeqCst);
    let held = fixture.credentials.cached.lock().await;
    let mut request = Box::pin(fixture.credentials.actor(&fixture.host.shared));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(request.as_mut(), &mut context).is_pending());
    fixture.clock.0.store(400, Ordering::SeqCst);
    drop(held);
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), request)
        .await
        .unwrap();
    let error = outcome.unwrap_err();
    fixture.host.shared.sessions.failed(&fixture.cookie, &error);
    let refused = matches!(error, Error::Denied);
    let session_removed = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 399)
        .is_none();
    let key_requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert!(refused);
    assert!(session_removed);
    assert_eq!(
        key_requests, 0,
        "expired original credentials must be rejected before JWKS acquisition"
    );
    assert_eq!(original_expiry, 400);
}

#[tokio::test]
async fn corrupted_original_signature_renewal_denies_and_removes_the_original_session() {
    let fixture = Fixture::with_token(200, true).await;
    assert!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 129)
            .is_some()
    );
    fixture.clock.0.store(130, Ordering::SeqCst);
    let outcome = crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await;
    let refused = matches!(outcome, Err(Error::Denied));
    let session_removed = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 130)
        .is_none();
    let key_requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert!(refused);
    assert!(session_removed);
    assert_eq!(
        key_requests, 1,
        "the actual renewal must verify the corrupted signature against acquired keys"
    );
    assert_eq!(original_expiry, 400);
}

#[tokio::test]
async fn proof_expiring_during_actual_bind_refreshes_original_credentials_without_losing_session() {
    let storage = Arc::new(bind_barrier::Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_storage(200, false, storage.clone()).await;
    let outcome = resolve_across_actual_bind(&fixture, &storage, 130).await;
    let actor_until = outcome.map(|actor| actor.valid_until());
    let surviving_expiry = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 130)
        .map(|session| session.expires_at());
    let key_requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert_eq!(
        actor_until,
        Ok(Some(160)),
        "a short cached proof expiring during bind must not invalidate the valid original token"
    );
    assert_eq!(surviving_expiry, Some(400));
    assert_eq!(original_expiry, 400);
    assert_eq!(
        key_requests, 1,
        "renewal must actually reverify signed original credentials"
    );
}

#[tokio::test]
async fn revoked_link_before_actual_bind_remains_denied_without_session_resurrection() {
    let storage = Arc::new(bind_barrier::Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_storage(200, false, storage).await;
    fixture
        .host
        .shared
        .runtime
        .execute(
            &Actor::trusted("host", "bootstrap"),
            Command::replace(
                &link_key("fixture", PrincipalKind::Human, "alice"),
                IdentityLink {
                    authority: "fixture".into(),
                    subject: "alice".into(),
                    principal_kind: "human".into(),
                    user_id: "alice-user".into(),
                    enabled: false,
                },
            )
            .at_revision(1)
            .idempotency("bind-control-revoke-link"),
        )
        .await
        .unwrap();
    fixture.clock.0.store(129, Ordering::SeqCst);
    let outcome = crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await;
    let refused = matches!(outcome, Err(Error::Denied));
    let removed = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 129)
        .is_none();
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert!(refused);
    assert!(removed);
    assert_eq!(original_expiry, 400);
}

pub(super) async fn resolve_across_actual_bind(
    fixture: &Fixture,
    storage: &bind_barrier::Controlled,
    after_read_entry: u64,
) -> rom::Result<Actor> {
    fixture.clock.0.store(129, Ordering::SeqCst);
    storage.arm();
    let shared = fixture.host.shared.clone();
    let cookie = fixture.cookie.clone();
    let request =
        tokio::spawn(async move { crate::authentication::resolve(&shared, &cookie).await });
    let entered = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        storage.started.notified(),
    )
    .await;
    if entered.is_err() {
        storage.release();
    }
    entered.expect("actual Runtime bind must enter the one-shot native user-read barrier");
    // The real cached-lock/clock check is complete. Actual native storage IO alone is held.
    fixture.clock.0.store(after_read_entry, Ordering::SeqCst);
    storage.release();
    tokio::time::timeout(std::time::Duration::from_secs(2), request)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn unchanged_clock_actual_bind_returns_original_live_proof_without_refresh() {
    let storage = Arc::new(bind_barrier::Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_storage(200, false, storage.clone()).await;
    let outcome = resolve_across_actual_bind(&fixture, &storage, 129).await;
    let until = outcome.map(|actor| actor.valid_until());
    let expiry = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 129)
        .map(|session| session.expires_at());
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(until, Ok(Some(130)));
    assert_eq!(expiry, Some(400));
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn original_token_expiring_during_actual_bind_denies_without_refresh_or_extension() {
    let storage = Arc::new(bind_barrier::Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_storage(200, false, storage.clone()).await;
    let outcome = resolve_across_actual_bind(&fixture, &storage, 400).await;
    let refused = matches!(outcome, Err(Error::Denied));
    let removed = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 399)
        .is_none();
    let requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert!(refused);
    assert!(removed);
    assert_eq!(requests, 0);
    assert_eq!(original_expiry, 400);
}

/// The same underlying clock advances only at the second Host sample. Runtime
/// authoritative bind has completed at129 before the Host observes130.
struct PostBindClock {
    inner: Arc<Time>,
    armed: std::sync::atomic::AtomicBool,
    samples: AtomicUsize,
}
impl Clock for PostBindClock {
    fn now(&self) -> u64 {
        if self.armed.load(Ordering::SeqCst) && self.samples.fetch_add(1, Ordering::SeqCst) == 1 {
            self.inner.0.store(130, Ordering::SeqCst);
        }
        self.inner.now()
    }
}
#[tokio::test]
async fn successful_bind_followed_by_exclusive_expiry_clock_sample_refreshes_before_return() {
    let clock = Arc::new(Time(AtomicU64::new(100)));
    let host_clock = Arc::new(PostBindClock {
        inner: clock.clone(),
        armed: std::sync::atomic::AtomicBool::new(false),
        samples: AtomicUsize::new(0),
    });
    let fixture = Fixture::with_storage_and_clocks(
        200,
        false,
        Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
        Some((clock.clone(), host_clock.clone())),
    )
    .await;
    clock.0.store(129, Ordering::SeqCst);
    host_clock.armed.store(true, Ordering::SeqCst);
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.credentials.actor(&fixture.host.shared),
    )
    .await
    .unwrap();
    if let Err(error) = &outcome {
        fixture.host.shared.sessions.failed(&fixture.cookie, error);
    }
    let observed_time = clock.now();
    let until = outcome.map(|actor| actor.valid_until());
    let retained_expiry = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, observed_time)
        .map(|session| session.expires_at());
    let requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert_eq!(observed_time, 130);
    assert_eq!(
        until,
        Ok(Some(160)),
        "successful old bind cannot return an Actor expired at the final Host sample"
    );
    assert_eq!(retained_expiry, Some(400));
    assert_eq!(original_expiry, 400);
    assert_eq!(requests, 1);
}
