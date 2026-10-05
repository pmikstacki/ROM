use rom::{
    Action, Actor, Command, Descriptor, DiscoveryTarget, Error, Field, FieldEnumLabels, Input,
    InputDescriptor, Invocation, Operation, Presence, Resource, Runtime, Shape, Value, json,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

static RENAMED: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Status(String);
impl Field for Status {
    fn shape() -> Shape {
        Shape::Enum(vec!["queued".into(), "running".into(), "paused".into()])
    }
    fn enum_labels() -> BTreeMap<String, String> {
        [
            (
                "queued".into(),
                if RENAMED.load(Ordering::SeqCst) {
                    "New waiting label"
                } else {
                    "Waiting"
                }
                .into(),
            ),
            ("running".into(), "Active".into()),
            ("paused".into(), "Active".into()),
        ]
        .into()
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> rom::Result<Self> {
        match value.as_str() {
            Some(value @ ("queued" | "running" | "paused")) => Ok(Self(value.into())),
            _ => Err(Error::invalid("input", "phase")),
        }
    }
}
#[derive(Clone, Resource)]
#[resource(name = "labelled")]
struct Labelled {
    phase: Status,
    nested: Presence<Option<Vec<BTreeMap<String, Status>>>>,
}
#[derive(Clone, Input)]
struct Choose {
    phase: Status,
}
const CHOOSE: Action<Labelled, Choose> = Action::new("choose", |row, input| {
    row.phase = input.phase;
    Ok(vec![])
});
const SCALAR: Action<Labelled, Status> = Action::new("scalar", |row, input| {
    row.phase = input;
    Ok(vec![])
});
fn runtime(store: Arc<rom_sqlite::Sqlite>) -> Runtime {
    Runtime::builder()
        .resource(
            Labelled::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(CHOOSE)
                .action(SCALAR)
                .discovery_policy(|actor, target| {
                    actor.subject != "restricted"
                        || matches!(
                            target,
                            DiscoveryTarget::Resource | DiscoveryTarget::Field("phase")
                        )
                }),
        )
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}

#[tokio::test]
async fn labels_are_shared_advisory_frozen_authorized_and_schema_independent() {
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let schema = Labelled::descriptor();
    let actor = Actor::trusted("test", "admin");
    let runtime = runtime(store.clone());
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/enum-labels-discovery-v1.json")).unwrap();
    assert_eq!(
        serde_json::to_value(runtime.discover(&actor).await.unwrap()).unwrap(),
        expected
    );
    assert_eq!(
        <Presence<Option<Vec<BTreeMap<String, Status>>>> as Field>::enum_labels(),
        Status::enum_labels()
    );
    let request = Command::create(
        "one",
        Labelled {
            phase: Status("queued".into()),
            nested: Presence::Missing,
        },
    )
    .idempotency("create");
    runtime.execute(&actor, request.clone()).await.unwrap();
    assert_eq!(
        runtime
            .read::<Labelled>(&actor, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .encode(),
        json!({"phase":"queued"})
    );
    for (action, input) in [
        ("choose", json!({"phase":"Waiting"})),
        ("scalar", json!("Active")),
    ] {
        let mut invalid: Invocation = Command::action("one", SCALAR, Status("queued".into()))
            .at_revision(1)
            .idempotency(&format!("invalid-{action}"))
            .into();
        invalid.operation = Operation::Action {
            name: action.into(),
            input,
        };
        assert!(matches!(
            runtime.invoke(&actor, invalid).await,
            Err(Error::Invalid { .. })
        ));
        assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    }
    let chosen = runtime
        .execute(
            &actor,
            Command::action(
                "one",
                CHOOSE,
                Choose {
                    phase: Status("running".into()),
                },
            )
            .at_revision(1)
            .idempotency("choose"),
        )
        .await
        .unwrap();
    assert_eq!(chosen.revision, 2);
    let scalar = runtime
        .execute(
            &actor,
            Command::action("one", SCALAR, Status("paused".into()))
                .at_revision(2)
                .idempotency("scalar"),
        )
        .await
        .unwrap();
    assert_eq!(scalar.revision, 3);
    assert_eq!(
        runtime
            .read::<Labelled>(&actor, "one")
            .await
            .unwrap()
            .value
            .unwrap()
            .encode(),
        json!({"phase":"paused"})
    );
    let restricted = serde_json::to_value(
        runtime
            .discover(&Actor::trusted("test", "restricted"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        restricted["resources"][0]["fields"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        restricted["resources"][0]["action_inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    RENAMED.store(true, Ordering::SeqCst);
    assert_eq!(Labelled::descriptor(), schema);
    assert_eq!(
        serde_json::to_value(runtime.discover(&actor).await.unwrap()).unwrap(),
        expected
    );
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let reopened = self::runtime(store);
    let fresh = serde_json::to_value(reopened.discover(&actor).await.unwrap()).unwrap();
    assert_eq!(
        fresh["resources"][0]["fields"][1]["enum_labels"]["queued"],
        "New waiting label"
    );
    assert_eq!(reopened.execute(&actor, request).await.unwrap().revision, 1);
    assert_eq!(Labelled::descriptor(), schema);
    reopened.shutdown().await.unwrap();
    RENAMED.store(false, Ordering::SeqCst);
}

#[derive(Clone)]
struct InvalidInput(Value);
impl Input for InvalidInput {
    fn descriptor() -> Option<InputDescriptor> {
        Some(serde_json::from_value(json!({"type":"scalar","value":{"shape":{"type":"string"},"enum_labels":{"queued":"Waiting"}}})).unwrap())
    }
    fn encode(&self) -> Value {
        self.0.clone()
    }
    fn decode(value: Value) -> rom::Result<Self> {
        Ok(Self(value))
    }
}
#[derive(Clone)]
struct InvalidLabels;
impl Resource for InvalidLabels {
    const KIND: &'static str = "invalid-labels";
    fn descriptor() -> Descriptor {
        Descriptor {
            kind: Self::KIND.into(),
            version: 1,
            fields: vec![rom::FieldDescriptor {
                name: "phase".into(),
                shape: Shape::String,
            }],
        }
    }
    fn field_enum_labels() -> Vec<FieldEnumLabels> {
        vec![FieldEnumLabels {
            name: "phase".into(),
            labels: [("queued".into(), "Waiting".into())].into(),
        }]
    }
    fn encode(&self) -> Value {
        json!({"phase":"queued"})
    }
    fn decode(_: Value) -> rom::Result<Self> {
        Ok(Self)
    }
    fn normalize_field(_: &str, value: Value) -> rom::Result<Value> {
        Ok(value)
    }
}
#[test]
fn invalid_labels_fail_registration_without_changing_value_contracts() {
    let build = |definition| {
        Runtime::builder().resource(definition).build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
    };
    assert!(matches!(
        build(InvalidLabels::definition()),
        Err(Error::Invalid { .. })
    ));
    let invalid: Action<Labelled, InvalidInput> = Action::new("invalid", |_, _| Ok(vec![]));
    assert!(
        Runtime::builder()
            .resource(Labelled::definition().action(invalid))
            .build(
                Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
                Runtime::shared_cpu_pool(1).unwrap()
            )
            .is_err()
    );
    assert!(<Status as Field>::decode(json!("Waiting")).is_err());
    assert_eq!(
        Field::encode(&<Status as Field>::decode(json!("queued")).unwrap()),
        json!("queued")
    );
}
