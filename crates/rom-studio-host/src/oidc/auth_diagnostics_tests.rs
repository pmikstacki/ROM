//! Actual original-token verification and native binding retain stage attribution.
use super::credentials_tests::Fixture;
use crate::{AuthOutcome, AuthStage};
use rom::Error;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn original_expiry_crossed_during_native_binding_is_attributed_without_renewal() {
    use super::credentials_tests::{bind_barrier, resolve_across_actual_bind};
    use std::sync::Arc;

    let storage = Arc::new(bind_barrier::Controlled::new(Arc::new(
        rom_sqlite::Sqlite::open(":memory:").unwrap(),
    )));
    let fixture = Fixture::with_diagnostics(200, false, storage.clone(), None, true).await;
    let outcome = resolve_across_actual_bind(&fixture, &storage, 400).await;
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    let requests = fixture.requests.load(Ordering::SeqCst);
    let original_expiry = fixture.credentials.expiry;
    let removed = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 399)
        .is_none();
    fixture.close().await;

    assert!(matches!(outcome, Err(Error::Denied)));
    assert_eq!(original_expiry, 400);
    assert!(removed);
    assert_eq!(requests, 0);
    assert_eq!(snapshot.stage(AuthStage::RenewDue).succeeded, 0);
    assert_eq!(snapshot.stage(AuthStage::RenewAfterBind).succeeded, 0);
    assert_eq!(snapshot.stage(AuthStage::OriginalExpiry).denied, 1);
}

#[tokio::test]
async fn cached_binding_and_due_renewal_have_separate_bounded_stage_counts() {
    let fixture = Fixture::observed(200, false).await;
    assert!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie)
            .await
            .is_ok()
    );
    let first = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(first.stage(AuthStage::CachedBind).succeeded, 1);
    assert_eq!(first.stage(AuthStage::RenewDue).succeeded, 0);
    fixture.clock.0.store(130, Ordering::SeqCst);
    assert_eq!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie)
            .await
            .unwrap()
            .valid_until(),
        Some(160)
    );
    let renewed = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(renewed.stage(AuthStage::RenewDue).succeeded, 1);
    assert_eq!(
        renewed
            .stage(AuthStage::OriginalTokenVerification)
            .succeeded,
        1
    );
    assert_eq!(renewed.stage(AuthStage::FreshBind).succeeded, 1);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
    fixture.clock.0.store(400, Ordering::SeqCst);
    assert!(matches!(
        fixture.credentials.actor(&fixture.host.shared).await,
        Err(Error::Denied)
    ));
    let expired = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(expired.stage(AuthStage::OriginalExpiry).denied, 1);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
    fixture.close().await;
}

#[tokio::test]
async fn invalid_original_signature_is_attributed_without_current_binding_or_secret_payloads() {
    let fixture = Fixture::observed(200, true).await;
    fixture.clock.0.store(130, Ordering::SeqCst);
    assert!(matches!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await,
        Err(Error::Denied)
    ));
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(
        snapshot.stage(AuthStage::OriginalTokenVerification).denied,
        1
    );
    assert_eq!(snapshot.stage(AuthStage::FreshBind).succeeded, 0);
    assert!(snapshot.failures.iter().any(|record| record.stage
        == AuthStage::OriginalTokenVerification
        && record.outcome == AuthOutcome::Denied));
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains("alice"));
    assert!(!json.contains(&fixture.cookie));
    assert!(
        !fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 130)
            .is_some()
    );
    fixture.close().await;
}

#[tokio::test]
async fn key_acquisition_overload_is_distinct_from_original_token_rejection() {
    let fixture = Fixture::observed(503, false).await;
    fixture.clock.0.store(130, Ordering::SeqCst);
    assert!(matches!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await,
        Err(Error::Overloaded)
    ));
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(snapshot.stage(AuthStage::KeyAcquisition).overloaded, 1);
    assert_eq!(
        snapshot.stage(AuthStage::OriginalTokenVerification).denied,
        0
    );
    assert_eq!(snapshot.stage(AuthStage::FreshBind).succeeded, 0);
    assert!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 130)
            .is_some()
    );
    fixture.close().await;
}

#[tokio::test]
async fn current_revoked_link_is_a_binding_failure_after_successful_original_verification() {
    let fixture = Fixture::observed(200, false).await;
    fixture
        .host
        .shared
        .runtime
        .execute(
            &rom::Actor::trusted("host", "bootstrap"),
            rom::Command::replace(
                &rom_identity::link_key("fixture", rom::PrincipalKind::Human, "alice"),
                rom_identity::IdentityLink {
                    authority: "fixture".into(),
                    subject: "alice".into(),
                    principal_kind: "human".into(),
                    user_id: "alice-user".into(),
                    enabled: false,
                },
            )
            .at_revision(1)
            .idempotency("diagnostic-revoke-link"),
        )
        .await
        .unwrap();
    fixture.clock.0.store(130, Ordering::SeqCst);
    assert!(matches!(
        crate::authentication::resolve(&fixture.host.shared, &fixture.cookie).await,
        Err(Error::Denied)
    ));
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(
        snapshot
            .stage(AuthStage::OriginalTokenVerification)
            .succeeded,
        1
    );
    assert_eq!(snapshot.stage(AuthStage::FreshBind).denied, 1);
    assert_eq!(snapshot.stage(AuthStage::SessionRemoval).succeeded, 1);
    assert!(
        fixture
            .host
            .shared
            .sessions
            .lookup(&fixture.cookie, 130)
            .is_none()
    );
    fixture.close().await;
}

#[tokio::test]
async fn current_stream_check_has_its_own_binding_stage_and_admitted_lane() {
    let fixture = Fixture::observed(200, false).await;
    let actor = fixture
        .credentials
        .actor(&fixture.host.shared)
        .await
        .unwrap();
    let session = fixture
        .host
        .shared
        .sessions
        .lookup(&fixture.cookie, 100)
        .unwrap();
    assert!(matches!(
        crate::observation_gate::check(&fixture.host.shared, &session, &actor).await,
        crate::observation_gate::Gate::Ready
    ));
    let snapshot = fixture.host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(
        snapshot
            .operation(crate::AuthOperation::CurrentStream)
            .admitted,
        1
    );
    assert_eq!(snapshot.stage(AuthStage::CurrentBind).succeeded, 1);
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
    fixture.close().await;
}
