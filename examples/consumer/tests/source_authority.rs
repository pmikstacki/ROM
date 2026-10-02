use rom::{
    Actor, Command, Error, Key, PrincipalKind, Resource, RevisionCondition, Runtime, SourcePermit,
    SourceProvenance,
};
use rom_sqlite::Sqlite;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, Resource)]
#[resource(name = "source-controls")]
struct Control {
    enabled: bool,
    generation: u64,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "owned-settings")]
struct Setting {
    enabled: bool,
}
fn actor() -> Actor {
    Actor::trusted("host", "loader").with_kind(PrincipalKind::Service)
}
fn permit(revision: u64, generation: u64) -> SourcePermit {
    SourcePermit::trusted(
        Key {
            kind: Setting::KIND.into(),
            id: "settings.example[0]".into(),
        },
        SourceProvenance {
            source: "deployment".into(),
            version: format!("revision-{generation}"),
            generation,
            field_origins: BTreeMap::from([("enabled".into(), "deployment".into())]),
        },
        RevisionCondition {
            key: Key {
                kind: Control::KIND.into(),
                id: "deployment".into(),
            },
            revision,
        },
        u64::MAX,
    )
    .unwrap()
}
async fn runtime() -> Runtime {
    let r = Runtime::builder()
        .resource(
            Control::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .resource(
            Setting::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(|a| a.subject == "loader"),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &actor(),
        Command::create(
            "deployment",
            Control {
                enabled: true,
                generation: 1,
            },
        )
        .idempotency("control"),
    )
    .await
    .unwrap();
    r
}
#[tokio::test]
async fn owner_permit_and_current_revision_are_required_before_publication() {
    let r = runtime().await;
    assert!(matches!(
        r.execute(
            &actor(),
            Command::create("settings.example[0]", Setting { enabled: false })
                .idempotency("ordinary")
        )
        .await,
        Err(Error::Denied)
    ));
    r.invoke_sourced(
        &actor(),
        Command::create("settings.example[0]", Setting { enabled: false })
            .idempotency("source-1")
            .into(),
        permit(1, 1),
    )
    .await
    .unwrap();
    let meta = r
        .source_provenance(&actor(), Setting::KIND, "settings.example[0]")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(meta.generation, 1);
    assert!(matches!(
        r.source_provenance(
            &Actor::trusted("host", "viewer"),
            Setting::KIND,
            "settings.example[0]"
        )
        .await,
        Err(Error::Denied)
    ));
    r.execute(
        &actor(),
        Command::replace(
            "deployment",
            Control {
                enabled: true,
                generation: 2,
            },
        )
        .at_revision(1)
        .idempotency("request-2"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.invoke_sourced(
            &actor(),
            Command::replace("settings.example[0]", Setting { enabled: true })
                .at_revision(1)
                .idempotency("late")
                .into(),
            permit(1, 1)
        )
        .await,
        Err(Error::Conflict)
    ));
    assert!(
        !r.read::<Setting>(&actor(), "settings.example[0]")
            .await
            .unwrap()
            .value
            .unwrap()
            .enabled
    );
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn metadata_only_reload_is_a_durable_revision_and_exact_replay_is_not() {
    let r = runtime().await;
    r.invoke_sourced(
        &actor(),
        Command::create("settings.example[0]", Setting { enabled: false })
            .idempotency("source-1")
            .into(),
        permit(1, 1),
    )
    .await
    .unwrap();
    let cmd = Command::replace("settings.example[0]", Setting { enabled: false })
        .at_revision(1)
        .idempotency("source-2");
    let outcome = r
        .invoke_sourced(&actor(), cmd.into(), permit(1, 2))
        .await
        .unwrap();
    assert_eq!(outcome.revision, 2);
    let repeat = r
        .invoke_sourced(
            &actor(),
            Command::replace("settings.example[0]", Setting { enabled: false })
                .at_revision(1)
                .idempotency("source-2")
                .into(),
            permit(1, 2),
        )
        .await
        .unwrap();
    assert_eq!(repeat.revision, 2);
    let unchanged = r
        .invoke_sourced(
            &actor(),
            Command::replace("settings.example[0]", Setting { enabled: false })
                .at_revision(2)
                .idempotency("same-provenance")
                .into(),
            permit(1, 2),
        )
        .await
        .unwrap();
    assert_eq!(unchanged.revision, 2);
    assert!(
        !serde_json::to_string(&repeat)
            .unwrap()
            .contains("source_provenance")
    );
    assert_eq!(
        r.source_provenance(&actor(), Setting::KIND, "settings.example[0]")
            .await
            .unwrap()
            .unwrap()
            .generation,
        2
    );
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn wrong_owner_scope_expiry_and_target_revision_fail_closed() {
    let r = runtime().await;
    let make = |owner: &str, target: &str, expiry| {
        SourcePermit::trusted(
            Key {
                kind: Setting::KIND.into(),
                id: target.into(),
            },
            SourceProvenance {
                source: owner.into(),
                version: "v1".into(),
                generation: 1,
                field_origins: BTreeMap::from([("enabled".into(), "deployment".into())]),
            },
            RevisionCondition {
                key: Key {
                    kind: Control::KIND.into(),
                    id: "deployment".into(),
                },
                revision: 1,
            },
            expiry,
        )
        .unwrap()
    };
    for bad in [
        make("other", "settings.example[0]", u64::MAX),
        make("deployment", "another-target", u64::MAX),
        make("deployment", "settings.example[0]", 1),
    ] {
        assert!(matches!(
            r.invoke_sourced(
                &actor(),
                Command::create("settings.example[0]", Setting { enabled: false })
                    .idempotency("denied")
                    .into(),
                bad
            )
            .await,
            Err(Error::Denied)
        ));
    }
    r.invoke_sourced(
        &actor(),
        Command::create("settings.example[0]", Setting { enabled: false })
            .idempotency("source")
            .into(),
        permit(1, 1),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.invoke_sourced(
            &actor(),
            Command::replace("settings.example[0]", Setting { enabled: true })
                .at_revision(2)
                .idempotency("stale-target")
                .into(),
            permit(1, 1)
        )
        .await,
        Err(Error::Conflict)
    ));
    r.shutdown().await.unwrap();
}
