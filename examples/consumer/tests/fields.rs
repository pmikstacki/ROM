use rom::{Actor, Command, Error, Field, FiniteF64, Resource, ResourceRef, Runtime, Shape, json};
use rom_sqlite::Sqlite;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Resource)]
#[resource(name = "people")]
struct Person {
    name: String,
}
#[derive(Clone, Resource)]
#[resource(name = "measurements")]
struct Measurement {
    signed: i64,
    value: FiniteF64,
    tags: Vec<Option<String>>,
    labels: BTreeMap<String, Vec<bool>>,
    person: ResourceRef<Person>,
}
#[tokio::test]
async fn standard_fields_roundtrip_through_the_same_resource_pipeline() {
    let runtime = Runtime::builder()
        .resource(
            Person::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .resource(
            Measurement::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let value = Measurement {
        signed: -4,
        value: FiniteF64::new(0.25).unwrap(),
        tags: vec![None, Some("".into())],
        labels: BTreeMap::from([("b".into(), vec![]), ("a".into(), vec![false, true])]),
        person: ResourceRef::new("p1").unwrap(),
    };
    let encoded = value.encode();
    let result = runtime
        .execute(
            &Actor::trusted("local", "alice"),
            Command::create("m1", value).idempotency("new"),
        )
        .await
        .unwrap();
    assert_eq!(result.value.unwrap().encode(), encoded);
    // References identify a target kind/id. This profile does not promise target existence.
    assert_eq!(Measurement::decode(encoded).unwrap().person.id(), "p1");
    runtime.shutdown().await.unwrap();
}
#[test]
fn nested_field_values_reject_wrong_shapes_and_nonfinite_numbers() {
    assert!(<Vec<Option<String>> as Field>::decode(json!([false])).is_err());
    assert!(<BTreeMap<String, Vec<bool>> as Field>::decode(json!({"a":[0]})).is_err());
    assert!(<i64 as Field>::decode(json!(u64::MAX)).is_err());
    assert!(<FiniteF64 as Field>::decode(json!(null)).is_err());
    assert!(<ResourceRef<Person> as Field>::decode(json!("")).is_err());
    assert!(ResourceRef::<Person>::new("").is_err());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteF64::new(bad).is_err());
    }
}
#[derive(Clone, Resource)]
#[resource(name = "ambiguous")]
struct Ambiguous {
    values: Vec<Option<Option<bool>>>,
}
#[test]
fn nullable_ambiguity_is_rejected_inside_collections() {
    let result = Runtime::builder().resource(Ambiguous::definition()).build(
        Arc::new(Sqlite::open(":memory:").unwrap()),
        Runtime::shared_cpu_pool(1).unwrap(),
    );
    assert!(matches!(result, Err(Error::Unsupported(_))));
}
#[test]
fn typed_reference_requires_registered_target_kind() {
    let result = Runtime::builder()
        .resource(Measurement::definition())
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        );
    assert!(matches!(result, Err(Error::Unsupported(_))));
}
#[derive(Clone)]
struct EmptyEnum;
impl Field for EmptyEnum {
    fn shape() -> Shape {
        Shape::Enum(vec![])
    }
    fn encode(&self) -> rom::Value {
        json!("")
    }
    fn decode(_: rom::Value) -> rom::Result<Self> {
        Ok(Self)
    }
}
#[derive(Clone, Resource)]
#[resource(name = "invalid_enum")]
struct InvalidEnum {
    status: EmptyEnum,
}
#[test]
fn invalid_enum_definition_is_rejected_at_registration() {
    let result = Runtime::builder()
        .resource(InvalidEnum::definition())
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        );
    assert!(matches!(result, Err(Error::Unsupported(_))));
}
#[derive(Clone, Resource)]
#[resource(name = "optional_float")]
struct OptionalFloat {
    value: Option<FiniteF64>,
}
#[test]
fn invalid_float_cannot_silently_become_null() {
    assert!(FiniteF64::new(f64::NAN).is_err());
    let valid = OptionalFloat {
        value: Some(FiniteF64::new(-0.0).unwrap()),
    };
    assert!(
        OptionalFloat::decode(valid.encode())
            .unwrap()
            .value
            .is_some()
    );
}
#[derive(Clone)]
struct Status(String);
impl Field for Status {
    fn shape() -> Shape {
        Shape::Enum(vec!["pending".into(), "ready".into()])
    }
    fn encode(&self) -> rom::Value {
        json!(self.0)
    }
    fn decode(value: rom::Value) -> rom::Result<Self> {
        value.as_str().map(|v| Self(v.into())).ok_or(Error::Denied)
    }
}
#[derive(Clone, Resource)]
#[resource(name = "states")]
struct State {
    status: Status,
}
#[tokio::test]
async fn enum_metadata_enforces_the_custom_codec_contract() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            State::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("test", "owner");
    assert!(
        runtime
            .execute(
                &actor,
                Command::create(
                    "s",
                    State {
                        status: Status("other".into())
                    }
                )
                .idempotency("bad")
            )
            .await
            .is_err()
    );
    assert_eq!(store.counts().unwrap(), [0, 0, 0, 0]);
    runtime
        .execute(
            &actor,
            Command::create(
                "s",
                State {
                    status: Status("pending".into()),
                },
            )
            .idempotency("good"),
        )
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
}
#[derive(Clone, Resource)]
#[resource(name = "left")]
struct Left {
    right: Option<ResourceRef<Right>>,
}
#[derive(Clone, Resource)]
#[resource(name = "right")]
struct Right {
    left: Option<ResourceRef<Left>>,
}
#[test]
fn mutually_referencing_kinds_register_without_graph_recursion() {
    assert!(
        Runtime::builder()
            .resource(Left::definition())
            .resource(Right::definition())
            .build(
                Arc::new(Sqlite::open(":memory:").unwrap()),
                Runtime::shared_cpu_pool(1).unwrap()
            )
            .is_ok()
    );
}
