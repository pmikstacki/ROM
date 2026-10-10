use crate::{
    control::{self, Request},
    protected::ProtectedDocument,
    provisioning,
};
use rom::{Actor, PrincipalKind};
use rom_identity::{IdentityLink, IdentityProvider, User, link_key};

fn configuration(adapter: &str) -> provisioning::Configuration {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = format!(
        "/var/tmp/rom-010-authentik-20261007/run/volume/private/control-tests-{}-{nonce}-{adapter}",
        std::process::id()
    );
    std::fs::create_dir(&directory).unwrap();
    provisioning::Configuration {
        adapter: adapter.into(),
        assets_directory: "/root/ROM/tests/identity/provider/host/assets".into(),
        database: format!("{directory}/database"),
        stop_file: format!("{directory}/stop"),
        control_directory: None,
        public_origin: None,
        private_token_endpoint: None,
        private_jwks_endpoint: None,
        issuer: "http://127.0.0.1:44390/application/o/rom-synthetic-identity/".into(),
        client_id: "rom-synthetic-identity".into(),
        client_secret: "synthetic-not-used".into(),
        authorization_endpoint: "http://127.0.0.1:44390/application/o/authorize/".into(),
        token_endpoint: "http://127.0.0.1:44390/application/o/token/".into(),
        jwks_endpoint: "http://127.0.0.1:44390/application/o/rom-synthetic-identity/jwks/".into(),
        verified_synthetic_subject: "synthetic-test-subject".into(),
    }
}
fn request(sequence: u8, action: &str) -> Request {
    Request::parse(format!("{{\"sequence\":{sequence},\"action\":\"{action}\"}}").as_bytes())
        .unwrap()
}

#[test]
fn requests_are_bounded_closed_actions_without_actor_or_resource_inputs() {
    for action in [
        "disable-user",
        "enable-user",
        "disable-link",
        "enable-link",
        "disable-provider",
        "enable-provider",
    ] {
        let _ = request(1, action);
    }
    for bytes in [
        br#"{"sequence":0,"action":"enable-user"}"#.as_slice(),
        br#"{"sequence":17,"action":"enable-user"}"#,
        br#"{"sequence":1,"action":"unknown"}"#,
        br#"{"sequence":1,"action":"enable-user","actor":{"authority":"fixture-host"}}"#,
        br#"{"sequence":1,"action":"enable-user","subject":"other"}"#,
        br#"{"sequence":1,"action":"enable-user","resource":"other"}"#,
        br#"{"sequence":1,"sequence":2,"action":"enable-user"}"#,
        br#"{"sequence":"1","action":"enable-user"}"#,
    ] {
        assert!(Request::parse(bytes).is_err());
    }
    assert!(Request::parse(&vec![b' '; 4097]).is_err());
}

#[tokio::test]
async fn controls_replace_current_identity_resources_on_both_real_adapters() {
    for adapter in ["sqlite", "redb"] {
        let configuration = configuration(adapter);
        let (runtime, actor) = provisioning::runtime(&configuration).unwrap();
        provisioning::seed(&runtime, &actor, &configuration)
            .await
            .unwrap();
        let subject = &configuration.verified_synthetic_subject;
        for (sequence, action) in [
            (1, "disable-user"),
            (2, "enable-user"),
            (3, "disable-link"),
            (4, "enable-link"),
            (5, "disable-provider"),
            (6, "enable-provider"),
        ] {
            let revision = control::apply(&runtime, &actor, subject, &request(sequence, action))
                .await
                .unwrap();
            assert_eq!(revision, if sequence % 2 == 1 { 2 } else { 3 });
            let enabled = match action {
                "disable-user" | "enable-user" => {
                    runtime
                        .read::<User>(&actor, "fixture-user")
                        .await
                        .unwrap()
                        .value
                        .unwrap()
                        .enabled
                }
                "disable-link" | "enable-link" => {
                    runtime
                        .read::<IdentityLink>(
                            &actor,
                            &link_key("authentik", PrincipalKind::Human, subject),
                        )
                        .await
                        .unwrap()
                        .value
                        .unwrap()
                        .enabled
                }
                _ => {
                    runtime
                        .read::<IdentityProvider>(&actor, "authentik")
                        .await
                        .unwrap()
                        .value
                        .unwrap()
                        .enabled
                }
            };
            assert_eq!(enabled, sequence % 2 == 0);
        }
        drop(runtime);
    }
}

#[tokio::test]
async fn unconfigured_embedded_human_and_service_actors_cannot_control_or_read_private_fixture_state()
 {
    for adapter in ["sqlite", "redb"] {
        let configuration = configuration(adapter);
        let (runtime, actor) = provisioning::runtime(&configuration).unwrap();
        provisioning::seed(&runtime, &actor, &configuration)
            .await
            .unwrap();
        for untrusted in [
            Actor::trusted("other", "configuration"),
            Actor::trusted("fixture-host", "other"),
            Actor::trusted("fixture-host", "configuration").with_kind(PrincipalKind::Human),
            Actor::trusted("fixture-host", "configuration").with_kind(PrincipalKind::Service),
            Actor::trusted("authentik", "synthetic-test-subject").with_kind(PrincipalKind::Human),
        ] {
            assert!(
                control::apply(
                    &runtime,
                    &untrusted,
                    &configuration.verified_synthetic_subject,
                    &request(1, "disable-user")
                )
                .await
                .is_err()
            );
            assert!(
                runtime
                    .read::<ProtectedDocument>(&untrusted, "private")
                    .await
                    .is_err()
            );
        }
        let snapshot = runtime.read::<User>(&actor, "fixture-user").await.unwrap();
        assert_eq!(snapshot.revision, 1);
        assert!(snapshot.value.unwrap().enabled);
        assert!(
            runtime
                .read::<ProtectedDocument>(&actor, "private")
                .await
                .is_ok()
        );
        drop(runtime);
    }
}

#[tokio::test]
async fn private_control_mailbox_rejects_out_of_order_symlink_and_oversize_requests_without_mutation()
 {
    for adapter in ["sqlite", "redb"] {
        let configuration = configuration(adapter);
        let (runtime, actor) = provisioning::runtime(&configuration).unwrap();
        provisioning::seed(&runtime, &actor, &configuration)
            .await
            .unwrap();
        let parent = std::path::Path::new(&configuration.database)
            .parent()
            .unwrap();
        for case in ["order", "symlink", "oversize", "sequence"] {
            let directory = parent.join(case);
            std::fs::create_dir(&directory).unwrap();
            let path = directory.join("request-01.json");
            match case {
                "order" => std::fs::write(
                    directory.join("request-02.json"),
                    br#"{"sequence":2,"action":"disable-user"}"#,
                )
                .unwrap(),
                "symlink" => std::os::unix::fs::symlink("/dev/null", &path).unwrap(),
                "oversize" => std::fs::write(&path, vec![b' '; 4097]).unwrap(),
                _ => std::fs::write(&path, br#"{"sequence":2,"action":"disable-user"}"#).unwrap(),
            }
            let mut mailbox = control::Mailbox::new(directory.to_str().unwrap()).unwrap();
            assert!(
                mailbox
                    .poll(&runtime, &actor, &configuration.verified_synthetic_subject)
                    .await
                    .is_err()
            );
            assert!(!directory.join("response-01.json").exists());
            let snapshot = runtime.read::<User>(&actor, "fixture-user").await.unwrap();
            assert_eq!(snapshot.revision, 1);
            assert!(snapshot.value.unwrap().enabled);
        }
    }
}

#[tokio::test]
async fn private_control_mailbox_acknowledges_once_after_real_commit() {
    for adapter in ["sqlite", "redb"] {
        let configuration = configuration(adapter);
        let (runtime, actor) = provisioning::runtime(&configuration).unwrap();
        provisioning::seed(&runtime, &actor, &configuration)
            .await
            .unwrap();
        let directory = std::path::Path::new(&configuration.database)
            .parent()
            .unwrap()
            .join("mailbox");
        std::fs::create_dir(&directory).unwrap();
        let mut mailbox = control::Mailbox::new(directory.to_str().unwrap()).unwrap();
        assert!(
            !mailbox
                .poll(&runtime, &actor, &configuration.verified_synthetic_subject)
                .await
                .unwrap()
        );
        std::fs::write(
            directory.join("request-01.json"),
            br#"{"sequence":1,"action":"disable-user"}"#,
        )
        .unwrap();
        assert!(
            mailbox
                .poll(&runtime, &actor, &configuration.verified_synthetic_subject)
                .await
                .unwrap()
        );
        let response: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("response-01.json")).unwrap())
                .unwrap();
        assert_eq!(
            response,
            serde_json::json!({"sequence":1,"revision":2,"status":"committed"})
        );
        assert!(
            !mailbox
                .poll(&runtime, &actor, &configuration.verified_synthetic_subject)
                .await
                .unwrap()
        );
        assert_eq!(
            runtime
                .read::<User>(&actor, "fixture-user")
                .await
                .unwrap()
                .revision,
            2
        );
    }
}

#[test]
fn fixture_https_origin_and_private_routes_are_explicit_and_closed() {
    let mut configuration = configuration("sqlite");
    assert_eq!(
        provisioning::approved_origin(&configuration).unwrap(),
        "http://127.0.0.1:44391"
    );
    configuration.public_origin = Some("https://127.0.0.1:44389".into());
    configuration.issuer = "https://127.0.0.1:44392/application/o/rom-synthetic-identity/".into();
    configuration.authorization_endpoint =
        "https://127.0.0.1:44392/application/o/authorize/".into();
    configuration.token_endpoint = "https://127.0.0.1:44392/application/o/token/".into();
    configuration.jwks_endpoint =
        "https://127.0.0.1:44392/application/o/rom-synthetic-identity/jwks/".into();
    configuration.private_token_endpoint =
        Some("http://127.0.0.1:44393/application/o/token/".into());
    configuration.private_jwks_endpoint =
        Some("http://127.0.0.1:44393/application/o/rom-synthetic-identity/jwks/".into());
    assert_eq!(
        provisioning::approved_origin(&configuration).unwrap(),
        "https://127.0.0.1:44389"
    );
    for origin in [
        "http://127.0.0.1:44389",
        "https://example.invalid",
        "https://127.0.0.1:44389/",
    ] {
        configuration.public_origin = Some(origin.into());
        assert!(provisioning::approved_origin(&configuration).is_err());
    }
    configuration.public_origin = Some("https://127.0.0.1:44389".into());
    configuration.private_token_endpoint =
        Some("http://example.invalid/application/o/token/".into());
    assert!(provisioning::approved_origin(&configuration).is_err());
    configuration.private_token_endpoint = None;
    assert!(provisioning::approved_origin(&configuration).is_err());
}
