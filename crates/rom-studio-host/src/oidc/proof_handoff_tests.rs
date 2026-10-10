//! Real signed credentials, manual clock and native identity binding at handoff.
use super::credentials_tests::{Fixture, bind_barrier::Controlled};
use crate::{AuthOperation, AuthStage};
use rom::{Actor, Command, Error, PrincipalKind};
use rom_identity::{IdentityLink, link_key};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

const BOUND: Duration = Duration::from_secs(2);
async fn resolve(fixture: &Fixture) -> rom::Result<Actor> {
    tokio::time::timeout(
        BOUND,
        crate::authentication::resolve_tagged(
            &fixture.host.shared,
            &fixture.cookie,
            AuthOperation::GenericHttpResolver,
        ),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn live_proof_with_more_than_body_budget_is_reused() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(124, Ordering::SeqCst);
    let result = resolve(&fixture).await.map(|actor| actor.valid_until());
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(result, Ok(Some(130)));
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn exact_body_budget_renews_once_before_downstream_io() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(125, Ordering::SeqCst);
    let result = resolve(&fixture).await;
    let requests = fixture.requests.load(Ordering::SeqCst);
    let until = result.as_ref().ok().and_then(Actor::valid_until);
    fixture.clock.0.store(130, Ordering::SeqCst);
    let status = result
        .as_ref()
        .map(|actor| fixture.host.shared.runtime.observation_status(actor));
    let expiry = fixture.credentials.expiry;
    fixture.close().await;
    assert_eq!(until, Some(155));
    assert!(matches!(status, Ok(Ok(()))));
    assert_eq!(requests, 1);
    assert_eq!(expiry, 400);
}

#[tokio::test]
async fn cache_lock_wait_rechecks_remaining_budget() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(124, Ordering::SeqCst);
    let held = fixture.credentials.cached.lock().await;
    let mut call = Box::pin(
        fixture
            .credentials
            .actor_observed(&fixture.host.shared, AuthOperation::GenericHttpResolver),
    );
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(call.as_mut(), &mut context).is_pending());
    fixture.clock.0.store(125, Ordering::SeqCst);
    drop(held);
    let result = tokio::time::timeout(BOUND, call)
        .await
        .unwrap()
        .map(|actor| actor.valid_until());
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(result, Ok(Some(155)));
    assert_eq!(requests, 1);
}

async fn across_bind(
    start: u64,
    advanced: u64,
) -> (rom::Result<Actor>, crate::AuthSnapshot, usize, bool) {
    let storage = Arc::new(Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_diagnostics(200, false, storage.clone(), None, true).await;
    fixture.clock.0.store(start, Ordering::SeqCst);
    storage.arm();
    let shared = fixture.host.shared.clone();
    let cookie = fixture.cookie.clone();
    let task = tokio::spawn(async move {
        crate::authentication::resolve_tagged(&shared, &cookie, AuthOperation::GenericHttpResolver)
            .await
    });
    let entered = tokio::time::timeout(BOUND, storage.started.notified()).await;
    if entered.is_ok() {
        fixture.clock.0.store(advanced, Ordering::SeqCst);
    }
    storage.release();
    let result = tokio::time::timeout(BOUND, task).await.unwrap().unwrap();
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    let requests = fixture.requests.load(Ordering::SeqCst);
    let retained = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, advanced)
        .is_some();
    assert_eq!(fixture.credentials.expiry, 400);
    fixture.close().await;
    entered.expect("real User load must enter its bounded one-shot barrier");
    (result, snapshot, requests, retained)
}

#[tokio::test]
async fn successful_cached_bind_crossing_handoff_budget_renews_once() {
    let (result, snapshot, requests, retained) = across_bind(124, 125).await;
    assert_eq!(result.map(|actor| actor.valid_until()), Ok(Some(155)));
    assert_eq!(snapshot.stage(AuthStage::CachedBind).succeeded, 1);
    assert_eq!(snapshot.stage(AuthStage::RenewAfterBind).succeeded, 1);
    assert_eq!(requests, 1);
    assert!(retained);
}

#[tokio::test]
async fn fresh_bind_consuming_budget_refuses_without_second_renewal_or_session_loss() {
    // Fresh proof is valid until155, but its bind completes at151.
    let (result, snapshot, requests, retained) = across_bind(125, 151).await;
    assert!(matches!(result, Err(Error::Overloaded)));
    assert_eq!(snapshot.stage(AuthStage::FreshBind).succeeded, 1);
    assert_eq!(requests, 1);
    assert!(retained);
}

#[tokio::test]
async fn original_expiry_during_fresh_bind_remains_terminal() {
    let (result, _, requests, retained) = across_bind(125, 400).await;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(requests, 1);
    assert!(!retained);
}

#[tokio::test]
async fn short_original_lifetime_refuses_admission_without_extending_or_losing_session() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(397, Ordering::SeqCst);
    let result = resolve(&fixture).await;
    let expiry = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 397)
        .map(|s| s.expires_at());
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.clock.0.store(400, Ordering::SeqCst);
    let expired = resolve(&fixture).await;
    let final_requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert!(matches!(result, Err(Error::Overloaded)));
    assert_eq!(expiry, Some(400));
    assert_eq!(requests, 1);
    assert!(matches!(expired, Err(Error::Denied)));
    assert_eq!(final_requests, 1);
}

#[tokio::test]
async fn early_renewal_reverifies_signature_and_current_link() {
    let fixture = Fixture::observed(200, true).await;
    fixture.clock.0.store(125, Ordering::SeqCst);
    let result = resolve(&fixture).await;
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(requests, 1);

    let fixture = Fixture::observed(200, false).await;
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
            .idempotency("handoff-revoke-link"),
        )
        .await
        .unwrap();
    fixture.clock.0.store(125, Ordering::SeqCst);
    let result = resolve(&fixture).await;
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert!(matches!(result, Err(Error::Denied)));
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn early_key_outage_never_falls_back_to_the_short_cached_proof() {
    let fixture = Fixture::observed(503, false).await;
    fixture.clock.0.store(125, Ordering::SeqCst);
    let result = resolve(&fixture).await;
    let retained = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 125)
        .is_some();
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert!(matches!(result, Err(Error::Overloaded)));
    assert!(retained);
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn ordinary_handoff_policy_does_not_renew_current_streams() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(129, Ordering::SeqCst);
    let live = fixture
        .credentials
        .current(&fixture.host.shared)
        .await
        .map(|a| a.valid_until());
    fixture.clock.0.store(130, Ordering::SeqCst);
    let expired = fixture.credentials.current(&fixture.host.shared).await;
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(live, Ok(Some(130)));
    assert!(matches!(expired, Err(Error::Denied)));
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn concurrent_near_expiry_handoffs_share_one_verified_renewal() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(125, Ordering::SeqCst);
    let calls = (0..8).map(|_| resolve(&fixture));
    let results = tokio::time::timeout(BOUND, futures_util::future::join_all(calls))
        .await
        .unwrap();
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(requests, 1);
    for result in results {
        assert_eq!(result.map(|a| a.valid_until()), Ok(Some(155)));
    }
}

#[tokio::test]
async fn nonordinary_session_and_unspecified_lanes_keep_the_existing_zero_margin() {
    let fixture = Fixture::observed(200, false).await;
    fixture.clock.0.store(129, Ordering::SeqCst);
    for operation in [
        AuthOperation::Session,
        AuthOperation::Unspecified,
        AuthOperation::ProtectedMiddleware,
    ] {
        let result = fixture
            .credentials
            .actor_observed(&fixture.host.shared, operation)
            .await;
        assert_eq!(result.map(|actor| actor.valid_until()), Ok(Some(130)));
    }
    let requests = fixture.requests.load(Ordering::SeqCst);
    fixture.close().await;
    assert_eq!(requests, 0);
}

#[tokio::test]
async fn fractional_custom_minimum_is_rounded_up_without_lowering_body_budget() {
    let fixture = Fixture::observed(200, false).await;
    let host = crate::StudioHost::new(
        fixture.host.shared.runtime.clone(),
        fixture
            .host
            .shared
            .config
            .clone()
            .proof_handoff_budget(Duration::from_millis(5_001)),
    )
    .unwrap();
    fixture.clock.0.store(123, Ordering::SeqCst);
    let reused = fixture
        .credentials
        .actor_observed(&host.shared, AuthOperation::GenericHttpResolver)
        .await;
    let first_requests = fixture.requests.load(Ordering::SeqCst);
    fixture.clock.0.store(124, Ordering::SeqCst);
    let renewed = fixture
        .credentials
        .actor_observed(&host.shared, AuthOperation::GenericHttpResolver)
        .await;
    let requests = fixture.requests.load(Ordering::SeqCst);
    host.shutdown().await.unwrap();
    fixture.close().await;
    assert_eq!(reused.map(|actor| actor.valid_until()), Ok(Some(130)));
    assert_eq!(first_requests, 0);
    assert_eq!(renewed.map(|actor| actor.valid_until()), Ok(Some(154)));
    assert_eq!(requests, 1);
}

#[tokio::test]
async fn early_renewal_preserves_provider_revision_and_user_revocation() {
    for provider in [false, true] {
        let fixture = Fixture::observed(200, false).await;
        let runtime = &fixture.host.shared.runtime;
        let admin = Actor::trusted("host", "bootstrap");
        if provider {
            let mut value = runtime
                .read::<rom_identity::IdentityProvider>(&admin, "fixture")
                .await
                .unwrap()
                .value
                .unwrap();
            value.enabled = false;
            runtime
                .execute(
                    &admin,
                    Command::replace("fixture", value)
                        .at_revision(1)
                        .idempotency("handoff-disable-provider"),
                )
                .await
                .unwrap();
        } else {
            let mut value = runtime
                .read::<rom_identity::User>(&admin, "alice-user")
                .await
                .unwrap()
                .value
                .unwrap();
            value.enabled = false;
            runtime
                .execute(
                    &admin,
                    Command::replace("alice-user", value)
                        .at_revision(1)
                        .idempotency("handoff-disable-user"),
                )
                .await
                .unwrap();
        }
        fixture.clock.0.store(125, Ordering::SeqCst);
        let result = resolve(&fixture).await;
        let requests = fixture.requests.load(Ordering::SeqCst);
        fixture.close().await;
        assert!(matches!(result, Err(Error::Denied)));
        assert_eq!(requests, 1);
    }
}
