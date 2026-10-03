use super::{
    BOUND, NOW,
    model::{Ledger, Record, RecordV2},
};
use rom::*;
use rom_identity::{
    IdentityGate, IdentityLink, IdentityProvider, ProviderActivation, ProviderProfile, User,
    link_key,
};
use std::sync::Arc;
pub fn host() -> Actor {
    Actor::trusted("studio-resilience-host", "admin")
}
pub struct Time;
impl Clock for Time {
    fn now(&self) -> u64 {
        NOW
    }
}
pub fn base() -> Builder {
    Runtime::builder()
        .clock(Arc::new(Time))
        .limits(Limits {
            actions: 2,
            ..Default::default()
        })
        .actor_gate(Arc::new(
            IdentityGate::default()
                .allow_host("studio-resilience-host", PrincipalKind::Embedded, "admin")
                .unwrap(),
        ))
        .resource(
            User::definition()
                .policy(|a, _, _| a == &host())
                .allow_all_fields(),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| a == &host())
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| a == &host())
                .allow_all_fields(),
        )
        .resource(
            Ledger::definition()
                .policy(|a, _, value| a == &host() || a.subject == value.owner)
                .allow_all_fields()
                .discovery_policy(|_, _| true),
        )
}
pub fn record_definition() -> Definition<Record> {
    Record::definition()
        .policy(|a, _, value| a == &host() || a.subject == value.owner)
        .allow_all_fields()
        .discovery_policy(|_, _| true)
}
pub fn after() -> Builder {
    base().resource(
        RecordV2::definition()
            .replay_from::<Record>()
            .policy(|a, _, value| a == &host() || a.subject == value.owner)
            .allow_all_fields()
            .discovery_policy(|_, _| true),
    )
}
pub async fn provision(runtime: &Runtime) -> ProviderActivation {
    runtime
        .execute(
            &host(),
            Command::create(
                "user",
                User {
                    enabled: true,
                    display_name: "Explicit profile".into(),
                },
            )
            .idempotency("user"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &host(),
            Command::create(
                "provider",
                IdentityProvider {
                    enabled: true,
                    profile: ProviderProfile::OidcRs256Human,
                    issuer: "https://issuer.example".into(),
                    audience: "studio-client".into(),
                    endpoint: Some("host-pinned-key-source".into()),
                    credential_ref: None,
                },
            )
            .idempotency("provider"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &host(),
            Command::create(
                &link_id(),
                IdentityLink {
                    authority: "provider".into(),
                    subject: "reader".into(),
                    principal_kind: "human".into(),
                    user_id: "user".into(),
                    enabled: true,
                },
            )
            .idempotency("link"),
        )
        .await
        .unwrap();
    activation(runtime).await
}
pub fn link_id() -> String {
    link_key("provider", PrincipalKind::Human, "reader")
}
pub async fn activation(runtime: &Runtime) -> ProviderActivation {
    ProviderActivation::read(runtime, &host(), "provider")
        .await
        .unwrap()
}
pub async fn human(runtime: &Runtime, activation: &ProviderActivation) -> Actor {
    use jsonwebtoken::{Algorithm, Header};
    use rom_auth::{
        AuthError, OidcIdTokenAdapter,
        jwt::{DecodingKey, TrustedKeys},
        oidc::OidcTokenBindings,
    };
    use std::collections::BTreeMap;
    struct Keys;
    impl TrustedKeys for Keys {
        fn fetch(&mut self) -> std::result::Result<BTreeMap<String, DecodingKey>, AuthError> {
            Ok(BTreeMap::from([(
                "fixture".into(),
                crate::signing::public(0),
            )]))
        }
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("fixture".into());
    let token = crate::signing::token(
        &json!({"iss":activation.config().issuer,"aud":activation.config().audience,"sub":"reader","iat":NOW,"exp":NOW+600,"nonce":"retained-host-nonce"}),
        header,
        0,
    );
    activation
        .verify(|authority, config| {
            OidcIdTokenAdapter::configured(authority, &config.issuer, &config.audience, Keys)?
                .authenticate(
                    &token,
                    "retained-host-nonce",
                    OidcTokenBindings::default(),
                    NOW,
                )
        })
        .unwrap()
        .bind(runtime)
        .await
        .unwrap()
}
pub async fn shutdown(runtime: &Runtime) {
    tokio::time::timeout(BOUND, runtime.shutdown())
        .await
        .unwrap()
        .unwrap();
}
pub async fn enabled(runtime: &Runtime, key: &Key, revision: u64, enabled: bool) {
    runtime
        .invoke(
            &host(),
            Invocation {
                kind: key.kind.clone(),
                id: key.id.clone(),
                expected: Some(revision),
                retry_epoch: 0,
                idempotency: format!("{}-{}-{revision}-{enabled}", key.kind, key.id),
                operation: Operation::Patch(std::collections::BTreeMap::from([(
                    "enabled".into(),
                    FieldUpdate::Set(json!(enabled)),
                )])),
            },
        )
        .await
        .unwrap();
}
pub fn identity_targets() -> [Key; 3] {
    [
        Key {
            kind: User::KIND.into(),
            id: "user".into(),
        },
        Key {
            kind: IdentityProvider::KIND.into(),
            id: "provider".into(),
        },
        Key {
            kind: IdentityLink::KIND.into(),
            id: link_id(),
        },
    ]
}
