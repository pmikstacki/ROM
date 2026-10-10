//! Session TTL remains exclusive across queued and authoritative identity checks.
use super::credentials_tests::{Fixture, bind_barrier::Controlled};
use crate::{
    AuthOperation, AuthStage, StudioHost,
    observation_gate::{self, Gate},
    session::{Session, SessionEvidence},
};
use futures_util::FutureExt;
use rom::{Actor, Error};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

struct ExpiryFixture {
    original: Fixture,
    host: StudioHost,
    session: Arc<Session>,
    storage: Arc<Controlled>,
}
impl ExpiryFixture {
    async fn new(poll: Duration) -> Self {
        let storage = Arc::new(Controlled::new(Arc::new(
            rom_sqlite::Sqlite::open(":memory:").unwrap(),
        )));
        let original = Fixture::with_diagnostics(200, false, storage.clone(), None, true).await;
        let actor = original
            .host
            .shared
            .sessions
            .lookup(&original.cookie, 100)
            .unwrap()
            .evidence
            .actor
            .clone();
        let mut config = original.host.shared.config.clone();
        config.limits.session_seconds = 5;
        config.limits.observation_poll = poll;
        let host = StudioHost::new(original.host.shared.runtime.clone(), config).unwrap();
        let session = host
            .shared
            .sessions
            .insert(
                SessionEvidence {
                    actor,
                    user_id: "alice-user".into(),
                    token_expiry: original.credentials.expiry,
                    credentials: Some(original.credentials.clone()),
                },
                100,
            )
            .unwrap();
        assert_eq!(session.expires_at(), 105);
        assert_eq!(session.evidence.actor.valid_until(), Some(130));
        assert_eq!(original.credentials.expiry, 400);
        Self {
            original,
            host,
            session,
            storage,
        }
    }
    fn expire(&self) {
        // No SessionStore lookup: expiry alone must stop the captured session.
        self.original.clock.0.store(105, Ordering::SeqCst);
        assert!(!*self.session.cancellation().borrow());
    }
    fn actor(&self) -> &Actor {
        &self.session.evidence.actor
    }
    async fn close(self) {
        assert_eq!(self.original.credentials.expiry, 400);
        assert_eq!(self.actor().valid_until(), Some(130));
        assert_eq!(self.original.requests.load(Ordering::SeqCst), 0);
        self.host.shutdown().await.unwrap();
        self.original.close().await;
    }
}

type Held = (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<rom::Result<()>>,
);
async fn occupy_all(fixture: &ExpiryFixture) -> Vec<Held> {
    let mut held = Vec::new();
    for _ in 0..fixture.host.shared.config.limits.authentication_jobs {
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        let shared = fixture.host.shared.clone();
        let task = tokio::spawn(async move {
            shared
                .auth
                .run(async move {
                    entered.send(()).unwrap();
                    released.await.unwrap();
                    Ok(())
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(2), ready)
            .await
            .unwrap()
            .unwrap();
        held.push((release, task));
    }
    held
}
async fn release_all(held: Vec<Held>) {
    for (release, task) in held {
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn session_expiry_before_queued_request_callback_denies_without_binding() {
    let fixture = ExpiryFixture::new(Duration::from_millis(10)).await;
    let held = occupy_all(&fixture).await;
    let mut request = Box::pin(crate::authentication::resolve(
        &fixture.host.shared,
        fixture.session.cookie(),
    ));
    assert!(request.as_mut().now_or_never().is_none());
    assert_eq!(fixture.host.shared.auth.queued_counts(), (1, 0));
    fixture.expire();
    release_all(held).await;
    let result = tokio::time::timeout(Duration::from_secs(2), request)
        .await
        .unwrap();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    let removed = fixture
        .host
        .shared
        .sessions
        .lookup(fixture.session.cookie(), 104)
        .is_none();
    fixture.close().await;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(snapshot.stage(AuthStage::CachedBind).succeeded, 0);
    assert!(
        removed,
        "authoritative session expiry must remove the denied session"
    );
}

#[tokio::test]
async fn session_expiry_after_successful_request_bind_denies_and_removes_session() {
    let fixture = ExpiryFixture::new(Duration::from_secs(5)).await;
    fixture.storage.arm();
    let shared = fixture.host.shared.clone();
    let cookie = fixture.session.cookie().to_owned();
    let request =
        tokio::spawn(async move { crate::authentication::resolve(&shared, &cookie).await });
    let entered =
        tokio::time::timeout(Duration::from_secs(2), fixture.storage.started.notified()).await;
    if entered.is_ok() {
        fixture.expire();
    }
    fixture.storage.release();
    let result = tokio::time::timeout(Duration::from_secs(2), request)
        .await
        .unwrap()
        .unwrap();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    let removed = fixture
        .host
        .shared
        .sessions
        .lookup(fixture.session.cookie(), 104)
        .is_none();
    fixture.close().await;
    entered.expect("real authoritative bind must enter the User read barrier");
    assert_eq!(snapshot.stage(AuthStage::CachedBind).succeeded, 1);
    assert!(matches!(result, Err(Error::Denied)));
    assert!(removed);
}

#[tokio::test]
async fn session_expiry_while_stream_is_queued_terminates_without_accepting_work() {
    let fixture = ExpiryFixture::new(Duration::from_millis(10)).await;
    let held = occupy_all(&fixture).await;
    let mut check = Box::pin(observation_gate::check(
        &fixture.host.shared,
        &fixture.session,
        fixture.actor(),
    ));
    assert!(check.as_mut().now_or_never().is_none());
    assert_eq!(fixture.host.shared.auth.queued_counts(), (0, 1));
    fixture.expire();
    let result = tokio::time::timeout(Duration::from_secs(1), check).await;
    let pending = fixture.host.shared.auth.queued_counts();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    release_all(held).await;
    fixture.close().await;
    assert!(matches!(result, Ok(Gate::Terminal("identity_expired"))));
    assert_eq!(pending, (0, 0));
    assert_eq!(snapshot.operation(AuthOperation::CurrentStream).admitted, 0);
}

#[tokio::test]
async fn session_expiry_after_successful_stream_bind_is_identity_expired() {
    let fixture = ExpiryFixture::new(Duration::from_secs(5)).await;
    fixture.storage.arm();
    let shared = fixture.host.shared.clone();
    let session = fixture.session.clone();
    let actor = fixture.actor().clone();
    let check =
        tokio::spawn(async move { observation_gate::check(&shared, &session, &actor).await });
    let entered =
        tokio::time::timeout(Duration::from_secs(2), fixture.storage.started.notified()).await;
    if entered.is_ok() {
        fixture.expire();
    }
    fixture.storage.release();
    let result = tokio::time::timeout(Duration::from_secs(2), check)
        .await
        .unwrap()
        .unwrap();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    fixture.close().await;
    entered.expect("real current bind must enter the User read barrier");
    assert_eq!(snapshot.stage(AuthStage::CurrentBind).succeeded, 1);
    assert!(matches!(result, Gate::Terminal("identity_expired")));
}

#[tokio::test]
async fn live_short_session_returns_original_actor_and_stream_ready() {
    let fixture = ExpiryFixture::new(Duration::from_millis(10)).await;
    fixture.original.clock.0.store(104, Ordering::SeqCst);
    let result = crate::authentication::resolve(&fixture.host.shared, fixture.session.cookie())
        .await
        .map(|actor| actor.valid_until());
    let gate =
        observation_gate::check(&fixture.host.shared, &fixture.session, fixture.actor()).await;
    fixture.close().await;
    assert_eq!(result, Ok(Some(130)));
    assert!(matches!(gate, Gate::Ready));
}

#[tokio::test]
async fn logout_has_cancellation_priority_over_simultaneous_session_expiry() {
    let fixture = ExpiryFixture::new(Duration::from_millis(10)).await;
    let held = occupy_all(&fixture).await;
    let mut check = Box::pin(observation_gate::check(
        &fixture.host.shared,
        &fixture.session,
        fixture.actor(),
    ));
    assert!(check.as_mut().now_or_never().is_none());
    fixture.original.clock.0.store(105, Ordering::SeqCst);
    assert!(
        fixture
            .host
            .shared
            .sessions
            .remove(fixture.session.cookie())
    );
    let result = tokio::time::timeout(Duration::from_secs(1), check).await;
    let pending = fixture.host.shared.auth.queued_counts();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    release_all(held).await;
    fixture.close().await;
    assert!(matches!(result, Ok(Gate::Cancelled)));
    assert_eq!(pending, (0, 0));
    assert_eq!(snapshot.operation(AuthOperation::CurrentStream).admitted, 0);
}
