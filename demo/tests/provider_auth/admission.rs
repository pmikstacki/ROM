use super::support::*;
use rom_demo::provider_profile::AuthLimits;
use std::time::Duration;

#[tokio::test]
async fn caller_cancellation_retains_capacity_and_cancelled_drain_can_be_reawaited() {
    let scratch = Scratch::new();
    let mut provider = Provider::new(true).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(
        scratch.file("v1", b"secret", 0o600),
        AuthLimits {
            jobs: 1,
            response_timeout: Duration::from_secs(2),
        },
    );
    let resolver = auth.resolver();
    let caller = tokio::spawn(resolver(headers("Bearer fixture-token")));
    provider.entered().await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(matches!(
        auth.resolver()(headers("Bearer next")).await,
        Err(rom::Error::Overloaded)
    ));
    assert_eq!(provider.count(), 1);
    auth.close();
    assert!(matches!(
        auth.resolver()(headers("Bearer next")).await,
        Err(rom::Error::Closed)
    ));
    // timeout cancels a drain future that has actually been polled against active work.
    assert!(
        tokio::time::timeout(Duration::from_millis(20), auth.drain())
            .await
            .is_err()
    );
    let owned = auth.clone();
    let waiter = tokio::spawn(async move { owned.drain().await });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());
    provider.release();
    tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    auth.drain().await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn caller_deadline_retains_capacity_until_verifier_completion() {
    let scratch = Scratch::new();
    let mut provider = Provider::new(true).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(
        scratch.file("v1", b"secret", 0o600),
        AuthLimits {
            jobs: 1,
            response_timeout: Duration::from_millis(30),
        },
    );
    let caller = tokio::spawn(auth.resolver()(headers("Bearer fixture-token")));
    provider.entered().await;
    assert!(matches!(caller.await.unwrap(), Err(rom::Error::Overloaded)));
    assert!(matches!(
        auth.resolver()(headers("Bearer next")).await,
        Err(rom::Error::Overloaded)
    ));
    assert_eq!(provider.count(), 1);
    provider.release();
    tokio::time::timeout(Duration::from_secs(1), auth.drain())
        .await
        .expect("the original admitted verifier should drain after release")
        .unwrap();

    // Prove the same job tracker admits work again after the detached verifier
    // releases its slot. The short caller deadline remains active here, so a
    // loaded host may time out the response even though the verifier was admitted.
    let retry = tokio::spawn(auth.resolver()(headers("Bearer after-completion")));
    tokio::time::timeout(Duration::from_secs(1), async {
        while provider.count() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the recovered job tracker should admit the second verifier");
    tokio::time::timeout(Duration::from_secs(1), auth.drain())
        .await
        .expect("the admitted verifier should drain")
        .unwrap();
    match retry.await.unwrap() {
        Ok(actor) => {
            assert_eq!(actor.subject, "service");
            assert_eq!(actor.valid_until(), Some(NOW + 5));
        }
        // `response_timeout` is independent of admission: under host load the
        // caller can time out while the admitted job still completes and drains.
        Err(rom::Error::Overloaded) => {}
        other => panic!("unexpected result after successful re-admission: {other:?}"),
    }
    assert_eq!(provider.count(), 2);
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn provider_revision_change_while_verifying_denies_stale_binding_without_holding_core_gate() {
    let scratch = Scratch::new();
    let mut provider = Provider::new(true).await;
    let mut fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(scratch.file("v1", b"secret", 0o600), AuthLimits::default());
    let caller = tokio::spawn(auth.resolver()(headers("Bearer fixture-token")));
    provider.entered().await;
    // Rotation advances the activation revision while the old verifier is paused.
    fixture.provider.credential_ref = Some("secret-v2".into());
    tokio::time::timeout(Duration::from_millis(200), fixture.change())
        .await
        .unwrap();
    provider.release();
    assert!(matches!(caller.await.unwrap(), Err(rom::Error::Denied)));
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}
