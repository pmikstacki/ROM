use super::support::*;
use rom_demo::provider_profile::AuthLimits;
use std::time::Duration;

/// Synthetic provider outage exercises host composition, not provider certification.
#[tokio::test]
async fn transport_timeout_denies_statically_then_drains_and_recovers_capacity() {
    let scratch = Scratch::new();
    let mut provider = Provider::new(true).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(
        scratch.file("v1", b"outage-secret-marker", 0o600),
        AuthLimits {
            jobs: 1,
            response_timeout: Duration::from_secs(2),
        },
    );
    let caller = tokio::spawn(auth.resolver()(headers("Bearer outage-token-marker")));
    provider.entered().await;
    // No release: the admitted verifier contacts the approved endpoint and reaches
    // its finite HTTP transport timeout, before the longer caller deadline.
    let error = tokio::time::timeout(Duration::from_secs(3), caller)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(error, rom::Error::Denied));
    let diagnostic = format!("{error:?} {error}");
    for marker in [
        "outage-secret-marker",
        "outage-token-marker",
        scratch.0.to_str().unwrap(),
    ] {
        assert!(!diagnostic.contains(marker));
    }
    assert_eq!(
        provider.count(),
        1,
        "failed authentication must not automatically retry"
    );
    tokio::time::timeout(Duration::from_secs(1), auth.drain())
        .await
        .unwrap()
        .unwrap();
    provider.release();
    let actor = tokio::time::timeout(
        Duration::from_secs(2),
        auth.resolver()(headers("Bearer after-outage")),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(actor.subject, "service");
    assert_eq!(actor.valid_until(), Some(NOW + 5));
    assert_eq!(
        provider.count(),
        2,
        "capacity must be reusable after failed worker completion"
    );
    auth.close();
    tokio::time::timeout(Duration::from_secs(1), auth.drain())
        .await
        .unwrap()
        .unwrap();
    fixture.finish().await;
}
