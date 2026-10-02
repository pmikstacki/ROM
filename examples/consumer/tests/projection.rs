use rom::{Access, Action, Actor, Command, Error, Intent, Resource, Result, Runtime};
use rom_sqlite::Sqlite;
use std::sync::Arc;
#[derive(Clone, Debug, Resource)]
#[resource(name = "vaults")]
struct Vault {
    public: String,
    secret: String,
}
fn fields(a: &Actor, access: Access, field: &str, _: &Vault) -> bool {
    a.subject == "admin"
        || match access {
            Access::Read => field == "public",
            Access::Write => a.subject == "writer",
        }
}
fn change(v: &mut Vault, input: String) -> Result<Vec<Intent>> {
    v.secret = input;
    Ok(vec![])
}
const CHANGE: Action<Vault, String> = Action::new("change", change);
fn runtime() -> Runtime {
    Runtime::builder()
        .resource(
            Vault::definition()
                .policy(|_, _, _| true)
                .field_policy(fields)
                .query_policy(|a, f| a.subject == "admin" || f == "public")
                .action(CHANGE),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap()
}
fn actor(s: &str) -> Actor {
    Actor::trusted("local", s)
}
#[tokio::test]
async fn projected_outcomes_never_decode_as_complete_resources() {
    let r = runtime();
    r.execute(
        &actor("admin"),
        Command::create(
            "v",
            Vault {
                public: "visible".into(),
                secret: "private".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    let v = r
        .read_projected(&actor("reader"), "vaults", "v")
        .await
        .unwrap();
    assert_eq!(
        v.value.unwrap(),
        rom::json!({"public":"visible"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert!(matches!(
        r.read::<Vault>(&actor("reader"), "v").await,
        Err(Error::Denied)
    ));
    let cmd = Command::action("v", CHANGE, "changed".into())
        .at_revision(1)
        .idempotency("change");
    let outcome = r
        .invoke_projected(&actor("writer"), cmd.into())
        .await
        .unwrap();
    assert_eq!(outcome.revision, 2);
    assert!(!outcome.value.unwrap().contains_key("secret"));
    let replay = r
        .invoke_projected(
            &actor("writer"),
            Command::action("v", CHANGE, "changed".into())
                .at_revision(1)
                .idempotency("change")
                .into(),
        )
        .await
        .unwrap();
    assert!(!replay.value.unwrap().contains_key("secret"));
    assert_eq!(
        r.read::<Vault>(&actor("admin"), "v")
            .await
            .unwrap()
            .value
            .unwrap()
            .secret,
        "changed"
    );
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn hidden_filter_is_denied_even_when_database_is_empty() {
    let r = runtime();
    assert!(matches!(
        r.query_projected(&actor("reader"), "vaults", "secret", rom::json!("guess"))
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        r.query::<Vault>(
            &actor("reader"),
            &Vault::secret_field().equals("guess".into())
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(
        r.query_projected(&actor("reader"), "vaults", "public", rom::json!("visible"))
            .await
            .unwrap()
            .is_empty()
    );
    r.shutdown().await.unwrap();
}
#[tokio::test]
async fn forbidden_derived_write_does_not_commit() {
    let r = runtime();
    r.execute(
        &actor("admin"),
        Command::create(
            "v",
            Vault {
                public: "visible".into(),
                secret: "private".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.invoke_projected(
            &actor("reader"),
            Command::action("v", CHANGE, "changed".into())
                .at_revision(1)
                .idempotency("denied")
                .into()
        )
        .await,
        Err(Error::Denied)
    ));
    assert_eq!(
        r.read::<Vault>(&actor("admin"), "v")
            .await
            .unwrap()
            .revision,
        1
    );
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn field_permissions_default_to_deny_and_mixed_replace_is_atomic() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let r = Runtime::builder()
        .resource(Vault::definition().policy(|_, _, _| true))
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    assert!(matches!(
        r.execute(
            &actor("admin"),
            Command::create(
                "v",
                Vault {
                    public: "x".into(),
                    secret: "y".into()
                }
            )
            .idempotency("create")
        )
        .await,
        Err(Error::Denied)
    ));
    r.shutdown().await.unwrap();
    let r = Runtime::builder()
        .resource(
            Vault::definition()
                .policy(|_, _, _| true)
                .field_policy(|a, access, field, _| {
                    a.subject == "admin" || matches!(access, Access::Read) || field == "public"
                })
                .query_policy(|_, _| true),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    r.execute(
        &actor("admin"),
        Command::create(
            "v",
            Vault {
                public: "x".into(),
                secret: "y".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    assert!(matches!(
        r.execute(
            &actor("reader"),
            Command::replace(
                "v",
                Vault {
                    public: "edited".into(),
                    secret: "changed".into()
                }
            )
            .at_revision(1)
            .idempotency("replace")
        )
        .await,
        Err(Error::Denied)
    ));
    let current = r.read::<Vault>(&actor("admin"), "v").await.unwrap();
    assert_eq!(current.revision, 1);
    assert_eq!(current.value.unwrap().public, "x");
    r.shutdown().await.unwrap();
}

#[tokio::test]
async fn projected_live_rechecks_revocation_before_buffered_delivery() {
    let r = runtime();
    r.execute(
        &actor("admin"),
        Command::create(
            "v",
            Vault {
                public: "x".into(),
                secret: "y".into(),
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    let reader = actor("reader");
    let mut live = r
        .live_projected(&reader, "vaults", "public", rom::json!("x"))
        .await
        .unwrap();
    let first = live.changed().await.unwrap();
    assert_eq!(first.len(), 1);
    assert!(!first[0].value.as_ref().unwrap().contains_key("secret"));
    r.revoke(&reader);
    assert!(matches!(live.changed().await, Err(Error::Denied)));
    r.shutdown().await.unwrap();
}
