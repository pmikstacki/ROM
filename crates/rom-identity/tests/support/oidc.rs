use super::*;
use rom_auth::{IdentityProfile, OidcIdTokenAdapter, VerifiedIdentity, oidc::OidcTokenBindings};

const NONCE: &str = "host-owned-login-nonce";
fn id_proof(activation: &ProviderActivation, subject: &str) -> VerifiedIdentity {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("fixture".into());
    let claims = rom::json!({"iss":activation.config().issuer,"aud":activation.config().audience,"sub":subject,"iat":NOW,"exp":NOW+600,"nonce":NONCE,"email":"ignored@example.invalid"});
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(&keys().private).unwrap(),
    )
    .unwrap();
    OidcIdTokenAdapter::configured(
        activation.authority(),
        &activation.config().issuer,
        &activation.config().audience,
        Source,
    )
    .unwrap()
    .authenticate(&token, NONCE, OidcTokenBindings::default(), NOW)
    .unwrap()
}
fn activated(activation: &ProviderActivation, subject: &str) -> ActivatedIdentity {
    activation
        .verify(|_, _| Ok(id_proof(activation, subject)))
        .unwrap()
}
async fn setup_oidc() -> (Runtime, Arc<Time>, ProviderActivation) {
    let (runtime, clock, _) = setup().await;
    let mut config = provider();
    config.profile = ProviderProfile::OidcRs256Human;
    runtime
        .execute(
            &admin(),
            Command::replace("provider", config)
                .at_revision(1)
                .idempotency("oidc-profile"),
        )
        .await
        .unwrap();
    let activation = ProviderActivation::read(&runtime, &admin(), "provider")
        .await
        .unwrap();
    (runtime, clock, activation)
}

#[tokio::test]
async fn oidc_proof_uses_explicit_current_link_and_finite_gate() {
    let (runtime, clock, activation) = setup_oidc().await;
    let proof = id_proof(&activation, "subject");
    assert_eq!(proof.profile(), IdentityProfile::OidcRs256Human);
    let evidence = activated(&activation, "subject");
    let actor = evidence.bind(&runtime).await.unwrap();
    assert_eq!(actor.principal_kind(), PrincipalKind::Human);
    assert_eq!(actor.valid_until(), Some(NOW + 30));
    assert_eq!(
        rom_identity::linked_user_id(&actor).as_deref(),
        Some("user")
    );
    runtime
        .execute(
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
        activated(&activation, "unlinked").bind(&runtime).await,
        Err(Error::Denied)
    ));
    clock.0.store(NOW + 30, Ordering::SeqCst);
    assert!(matches!(
        runtime.read::<Document>(&actor, "d").await,
        Err(Error::Denied)
    ));
    assert!(matches!(evidence.bind(&runtime).await, Err(Error::Denied)));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn access_and_id_proofs_cannot_cross_provider_profiles_or_stamp_profiles() {
    let (runtime, _, access_activation) = setup().await;
    let id = id_proof(&access_activation, "subject");
    assert!(matches!(
        access_activation.verify(|_, _| Ok(id)),
        Err(Error::Denied)
    ));
    let mut access_header = Header::new(Algorithm::RS256);
    access_header.typ = Some("at+jwt".into());
    access_header.kid = Some("fixture".into());
    let claims = rom::json!({"iss":access_activation.config().issuer,"aud":access_activation.config().audience,"sub":"subject","iat":NOW,"exp":NOW+600,"jti":"id","client_id":"client","principal_kind":"human"});
    let token = encode(
        &access_header,
        &claims,
        &EncodingKey::from_rsa_pem(&keys().private).unwrap(),
    )
    .unwrap();
    let access = JwtAdapter::configured(
        "provider",
        &access_activation.config().issuer,
        &access_activation.config().audience,
        Source,
    )
    .unwrap()
    .authenticate(&token, NOW)
    .unwrap();
    let mut config = provider();
    config.profile = ProviderProfile::OidcRs256Human;
    runtime
        .execute(
            &admin(),
            Command::replace("provider", config)
                .at_revision(1)
                .idempotency("oidc"),
        )
        .await
        .unwrap();
    let current = ProviderActivation::read(&runtime, &admin(), "provider")
        .await
        .unwrap();
    assert!(matches!(
        current.verify(|_, _| Ok(access)),
        Err(Error::Denied)
    ));
    let actor = activated(&current, "subject").bind(&runtime).await.unwrap();
    let mut stamp: rom::Value = serde_json::from_str(actor.host_stamp().unwrap()).unwrap();
    stamp["profile"] = rom::json!("jwt-rs256-human");
    let changed = actor.with_host_stamp(&stamp.to_string());
    assert!(matches!(
        runtime
            .query::<Document>(&changed, &Document::body_field().equals("x".into()))
            .await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn oidc_current_provider_user_and_link_revisions_do_not_revive_evidence() {
    for changed in ["provider", "user", "link"] {
        let (runtime, _, activation) = setup_oidc().await;
        let evidence = activated(&activation, "subject");
        let actor = evidence.bind(&runtime).await.unwrap();
        match changed {
            "provider" => {
                let mut config = activation.config().clone();
                config.enabled = false;
                runtime
                    .execute(
                        &admin(),
                        Command::replace("provider", config)
                            .at_revision(2)
                            .idempotency("disable"),
                    )
                    .await
                    .unwrap();
                assert!(
                    ProviderActivation::read(&runtime, &admin(), "provider")
                        .await
                        .is_err()
                );
                runtime
                    .execute(
                        &admin(),
                        Command::replace("provider", activation.config().clone())
                            .at_revision(3)
                            .idempotency("enable"),
                    )
                    .await
                    .unwrap();
            }
            "user" => {
                for (revision, enabled) in [(1, false), (2, true)] {
                    runtime
                        .execute(
                            &admin(),
                            Command::replace(
                                "user",
                                User {
                                    enabled,
                                    display_name: "Profile".into(),
                                },
                            )
                            .at_revision(revision)
                            .idempotency(&format!("user-{revision}")),
                        )
                        .await
                        .unwrap();
                    if !enabled {
                        assert!(matches!(evidence.bind(&runtime).await, Err(Error::Denied)));
                    }
                }
            }
            "link" => {
                let id = link_key("provider", PrincipalKind::Human, "subject");
                let mut link = runtime
                    .read::<IdentityLink>(&admin(), &id)
                    .await
                    .unwrap()
                    .value
                    .unwrap();
                for (revision, enabled) in [(1, false), (2, true)] {
                    link.enabled = enabled;
                    runtime
                        .execute(
                            &admin(),
                            Command::replace(&id, link.clone())
                                .at_revision(revision)
                                .idempotency(&format!("link-{revision}")),
                        )
                        .await
                        .unwrap();
                    if !enabled {
                        assert!(matches!(evidence.bind(&runtime).await, Err(Error::Denied)));
                    }
                }
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            runtime
                .query::<Document>(&actor, &Document::body_field().equals("x".into()))
                .await,
            Err(Error::Denied)
        ));
        let current = ProviderActivation::read(&runtime, &admin(), "provider")
            .await
            .unwrap();
        if changed == "provider" {
            // Verification after an activation becomes stale still cannot bind its captured revision.
            assert!(matches!(
                activated(&activation, "subject").bind(&runtime).await,
                Err(Error::Denied)
            ));
        }
        assert!(activated(&current, "subject").bind(&runtime).await.is_ok());
        runtime.shutdown().await.unwrap();
    }
}
