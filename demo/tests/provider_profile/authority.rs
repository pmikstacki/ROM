use super::support::*;
use rom::{Actor, Command, Error, PrincipalKind};
use rom_demo::{
    Task,
    provider_profile::{LocalMode, build, configuration_reader, maintain, provision},
};
use std::sync::Arc;
#[tokio::test]
async fn serving_excludes_provisioner_and_local_roles_cannot_mutate_business_resource() {
    let scratch = Scratch::new();
    let path = scratch.0.join("db");
    let config = settings("http://127.0.0.1:1/introspect");
    let runtime = build(
        storage(false, &path),
        Arc::new(Time),
        &config,
        LocalMode::Provisioning,
    )
    .unwrap();
    provision(&runtime, &config).await.unwrap();
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let runtime = build(
        storage(false, &path),
        Arc::new(Time),
        &config,
        LocalMode::Serving,
    )
    .unwrap();
    assert!(matches!(
        provision(&runtime, &config).await,
        Err(Error::Denied)
    ));
    for actor in [
        configuration_reader(),
        Actor::trusted("provider-profile-host", "maintainer"),
        Actor::trusted("provider", "service").with_kind(PrincipalKind::Service),
    ] {
        assert!(matches!(
            runtime
                .execute(
                    &actor,
                    Command::create(
                        "x",
                        Task {
                            title: "x".into(),
                            done: false
                        }
                    )
                    .idempotency("x")
                )
                .await,
            Err(Error::Denied)
        ));
    }
    let invocation = rom::Invocation {
        kind: "users".into(),
        id: "other".into(),
        expected: Some(1),
        idempotency: "x".into(),
        retry_epoch: 0,
        operation: rom::Operation::Delete,
    };
    assert!(matches!(
        maintain(&runtime, &config, invocation).await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn linked_service_authority_revocation_and_structured_archive_redaction() {
    use rom::{FieldUpdate, Invocation, Operation, Resource};
    use rom_demo::provider_profile::AuthLimits;
    use rom_identity::{IdentityLink, IdentityProvider, User, link_key};
    use std::collections::BTreeMap;
    let provider = Provider::new(false).await;
    for redb in [false, true] {
        let scratch = Scratch::new();
        let config = settings(&provider.endpoint);
        let db = Db::open(redb, &scratch.0.join("db"));
        let runtime = build(
            db.storage(),
            Arc::new(Time),
            &config,
            LocalMode::Provisioning,
        )
        .unwrap();
        provision(&runtime, &config).await.unwrap();
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let runtime = build(db.storage(), Arc::new(Time), &config, LocalMode::Serving).unwrap();
        let secret_marker = "unique-private-credential-profile-marker";
        let token_marker = "unique-bearer-profile-marker";
        let auth = authentication(
            &runtime,
            &config,
            &scratch,
            AuthLimits::default(),
            secret_marker.as_bytes(),
        );
        let actor = auth.resolver()(headers(&format!("Bearer {token_marker}")))
            .await
            .unwrap();
        let command = Command::create(
            "task",
            Task {
                title: "Allowed".into(),
                done: false,
            },
        )
        .idempotency("task");
        runtime.execute(&actor, command.clone()).await.unwrap();
        runtime
            .execute(
                &actor,
                Command::action("task", rom_demo::COMPLETE, ())
                    .at_revision(1)
                    .idempotency("complete"),
            )
            .await
            .unwrap();
        let capabilities = runtime.operator_capabilities(&actor).await.unwrap();
        assert!(!capabilities.inspect && !capabilities.retry && !capabilities.reconcile);
        assert!(matches!(
            runtime
                .work_list(
                    &actor,
                    rom::operator::WorkQuery {
                        state: None,
                        category: None,
                        definition: None,
                        limit: 10,
                        cursor: None
                    }
                )
                .await,
            Err(Error::Denied)
        ));
        for target in [
            (User::KIND, "user".to_owned()),
            (IdentityProvider::KIND, "provider".to_owned()),
            (
                IdentityLink::KIND,
                link_key("provider", PrincipalKind::Service, "service"),
            ),
        ] {
            let current = auth.resolver()(headers(&format!("Bearer {token_marker}")))
                .await
                .unwrap();
            runtime.execute(&current, command.clone()).await.unwrap();
            let disable = Invocation {
                kind: target.0.into(),
                id: target.1.clone(),
                expected: Some(1),
                idempotency: format!("disable-{}", target.0),
                retry_epoch: 0,
                operation: Operation::Patch(BTreeMap::from([(
                    "enabled".into(),
                    FieldUpdate::Set(rom::json!(false)),
                )])),
            };
            maintain(&runtime, &config, disable).await.unwrap();
            assert!(matches!(
                runtime.execute(&current, command.clone()).await,
                Err(Error::Denied)
            ));
            let enable = Invocation {
                kind: target.0.into(),
                id: target.1,
                expected: Some(2),
                idempotency: format!("enable-{}", target.0),
                retry_epoch: 0,
                operation: Operation::Patch(BTreeMap::from([(
                    "enabled".into(),
                    FieldUpdate::Set(rom::json!(true)),
                )])),
            };
            maintain(&runtime, &config, enable).await.unwrap();
            assert!(matches!(
                runtime.execute(&current, command.clone()).await,
                Err(Error::Denied)
            ));
        }
        auth.close();
        auth.drain().await.unwrap();
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let snapshot = db.archive(&scratch.0.join("archive"));
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        for marker in [secret_marker, token_marker] {
            assert!(!bytes.windows(marker.len()).any(|w| w == marker.as_bytes()));
        }
        assert_eq!(snapshot.state.operator_receipt_count(), 0);
        assert!(
            snapshot
                .state
                .work_snapshot(2048, 8 * 1024 * 1024)
                .unwrap()
                .records
                .is_empty()
        );
    }
}
#[tokio::test]
async fn linked_service_discovers_only_declared_task_metadata() {
    use rom_demo::provider_profile::AuthLimits;
    let provider = Provider::new(false).await;
    let scratch = Scratch::new();
    let config = settings(&provider.endpoint);
    let runtime = build(
        storage(false, &scratch.0.join("db")),
        Arc::new(Time),
        &config,
        LocalMode::Provisioning,
    )
    .unwrap();
    provision(&runtime, &config).await.unwrap();
    let auth = authentication(
        &runtime,
        &config,
        &scratch,
        AuthLimits::default(),
        b"private-credential",
    );
    let actor = auth.resolver()(headers("Bearer private-token"))
        .await
        .unwrap();
    let definitions = runtime.discover(&actor).await.unwrap();
    assert_eq!(definitions.resources.len(), 1);
    assert_eq!(definitions.resources[0].kind, "tasks");
    assert_eq!(definitions.resources[0].actions.len(), 1);
    auth.close();
    auth.drain().await.unwrap();
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn local_maintenance_validates_key_revision_identity_and_operation_before_mutation() {
    use rom::{Invocation, Operation};
    let scratch = Scratch::new();
    let db = Db::open(false, &scratch.0.join("db"));
    let config = settings("http://127.0.0.1:1/introspect");
    let runtime = build(
        db.storage(),
        Arc::new(Time),
        &config,
        LocalMode::Provisioning,
    )
    .unwrap();
    provision(&runtime, &config).await.unwrap();
    let base = Invocation {
        kind: "users".into(),
        id: "user".into(),
        expected: Some(1),
        idempotency: "valid".into(),
        retry_epoch: 0,
        operation: Operation::Delete,
    };
    let mut cases = Vec::new();
    let mut v = base.clone();
    v.expected = None;
    cases.push(v);
    let mut v = base.clone();
    v.expected = Some(0);
    cases.push(v);
    let mut v = base.clone();
    v.idempotency = String::new();
    cases.push(v);
    let mut v = base.clone();
    v.id = "other".into();
    cases.push(v);
    let mut v = base.clone();
    v.operation = Operation::Create(rom::json!({"enabled":true,"display_name":"Other"}));
    cases.push(v);
    let mut v = base.clone();
    v.operation = Operation::Action {
        name: "anything".into(),
        input: rom::json!(null),
    };
    cases.push(v);
    for invocation in cases {
        assert!(matches!(
            maintain(&runtime, &config, invocation).await,
            Err(Error::Denied)
        ));
        assert_eq!(db.counts(), [3, 3, 3, 0]);
    }
    for actor in [
        Actor::trusted("provider", "service")
            .with_kind(PrincipalKind::Service)
            .with_host_stamp("{}")
            .expires_at(200),
        Actor::trusted("provider", "unlinked")
            .with_kind(PrincipalKind::Service)
            .expires_at(200),
    ] {
        assert!(matches!(
            runtime
                .execute(
                    &actor,
                    Command::create(
                        "forged",
                        Task {
                            title: "x".into(),
                            done: false
                        }
                    )
                    .idempotency("forged")
                )
                .await,
            Err(Error::Denied)
        ));
    }
    assert!(matches!(
        runtime
            .read::<rom_identity::User>(&configuration_reader(), "user")
            .await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
