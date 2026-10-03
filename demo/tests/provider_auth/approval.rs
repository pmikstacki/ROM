use super::support::*;
use rom_auth::introspection::EndpointPolicy;
use rom_demo::provider_profile::{AuthLimits, HostAuth, SecretFiles};
use std::{collections::BTreeMap, time::Duration};

#[tokio::test]
async fn malformed_duplicate_and_oversized_bearers_never_contact_provider() {
    let scratch = Scratch::new();
    let provider = Provider::new(false).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let auth = fixture.auth(scratch.file("v1", b"secret", 0o600), AuthLimits::default());
    let mut cases = vec![rom_http::HeaderMap::new()];
    for value in [
        "Basic value",
        "Bearer ",
        "Bearer two words",
        "Bearer\tvalue",
        "bearer value",
    ] {
        cases.push(headers(value));
    }
    cases.push(headers(&format!("Bearer {}", "x".repeat(4097))));
    let mut duplicate = headers("Bearer one");
    duplicate.append("authorization", "Bearer two".parse().unwrap());
    cases.push(duplicate);
    for request in cases {
        assert!(matches!(
            auth.resolver()(request).await,
            Err(rom::Error::Denied)
        ));
    }
    assert_eq!(provider.count(), 0);
    auth.close();
    auth.drain().await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn current_unapproved_configuration_denies_before_external_acquisition() {
    let scratch = Scratch::new();
    let provider = Provider::new(false).await;
    for field in [
        "issuer",
        "audience",
        "endpoint",
        "profile",
        "reference",
        "disabled",
    ] {
        let mut fixture = Fixture::new(&provider.endpoint).await;
        let auth = fixture.auth(scratch.file(field, b"secret", 0o600), AuthLimits::default());
        match field {
            "issuer" => fixture.provider.issuer = "https://unapproved.invalid".into(),
            "audience" => fixture.provider.audience = "other".into(),
            "endpoint" => fixture.provider.endpoint = Some("http://127.0.0.1:9/arbitrary".into()),
            "profile" => fixture.provider.profile = rom_identity::ProviderProfile::JwtRs256Human,
            "reference" => fixture.provider.credential_ref = Some("unapproved".into()),
            _ => fixture.provider.enabled = false,
        }
        fixture.change().await;
        denied(&auth).await;
        auth.close();
        auth.drain().await.unwrap();
        fixture.finish().await;
    }
    assert_eq!(provider.count(), 0);
}

#[tokio::test]
async fn unsafe_endpoint_and_missing_provider_are_static_denials() {
    let scratch = Scratch::new();
    let provider = Provider::new(false).await;
    let fixture = Fixture::new(&provider.endpoint).await;
    let file = scratch.file("v1", b"secret", 0o600);
    let make = |approved| {
        HostAuth::new(
            fixture.runtime.clone(),
            host(),
            fixture.clock.clone(),
            approved,
            SecretFiles::new(BTreeMap::from([("secret-v1".into(), file.clone())])).unwrap(),
            EndpointPolicy::HttpsOnly,
            AuthLimits::default(),
        )
        .unwrap()
    };
    let auth = make(fixture.approved());
    denied(&auth).await;
    auth.close();
    auth.drain().await.unwrap();
    let mut approved = fixture.approved();
    approved.authority = "absent".into();
    let auth = make(approved);
    denied(&auth).await;
    auth.close();
    auth.drain().await.unwrap();
    assert_eq!(provider.count(), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn invalid_host_capacity_and_unrepresentable_timeout_reject_at_construction() {
    let scratch = Scratch::new();
    let fixture = Fixture::new("http://127.0.0.1:9/introspect").await;
    let file = scratch.file("v1", b"secret", 0o600);
    for limits in [
        AuthLimits {
            jobs: 0,
            response_timeout: Duration::from_secs(2),
        },
        AuthLimits {
            jobs: usize::MAX,
            response_timeout: Duration::from_secs(2),
        },
        AuthLimits {
            jobs: 1,
            response_timeout: Duration::ZERO,
        },
        AuthLimits {
            jobs: 1,
            response_timeout: Duration::MAX,
        },
    ] {
        assert!(
            HostAuth::new(
                fixture.runtime.clone(),
                host(),
                fixture.clock.clone(),
                fixture.approved(),
                SecretFiles::new(BTreeMap::from([("secret-v1".into(), file.clone())])).unwrap(),
                EndpointPolicy::LoopbackTestOnly,
                limits
            )
            .is_err()
        );
    }
    fixture.finish().await;
}
