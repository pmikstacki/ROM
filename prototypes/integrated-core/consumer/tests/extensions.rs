use rom_probe::{
    Actor, Command, Descriptor, Error, Field, Resource, Result, Runtime, Shape, Value,
};
use rom_probe_sqlite::Sqlite;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq)]
struct Label(String);
impl Field for Label {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        Value::String(self.0.clone())
    }
    fn decode(v: Value) -> Result<Self> {
        let label = v
            .as_str()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::invalid("label", "empty"))?;
        Ok(Self(label.into()))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "custom")]
struct Custom {
    label: Label,
    enabled: bool,
}
#[derive(Clone)]
struct Manual(Custom);
impl Resource for Manual {
    const KIND: &'static str = "custom";
    fn descriptor() -> Descriptor {
        Custom::descriptor()
    }
    fn encode(&self) -> Value {
        self.0.encode()
    }
    fn decode(value: Value) -> Result<Self> {
        Custom::decode(value).map(Self)
    }
}
#[tokio::test]
async fn downstream_custom_field_and_manual_definition_use_identical_contract() {
    assert_eq!(Custom::descriptor(), Manual::descriptor());
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(Custom::definition().policy(|_, _, _| true))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("host", "service");
    rom.execute(
        &actor,
        Command::create(
            "c",
            Custom {
                label: Label("custom typed value".into()),
                enabled: true,
            },
        )
        .idempotency("create"),
    )
    .await
    .unwrap();
    let rows = rom
        .query(
            &actor,
            &Custom::label_field().equals(Label("custom typed value".into())),
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    let invalid = rom
        .execute(
            &actor,
            Command::replace(
                "c",
                Custom {
                    label: Label("".into()),
                    enabled: true,
                },
            )
            .at_revision(1)
            .idempotency("bad"),
        )
        .await;
    assert!(matches!(invalid,Err(Error::Invalid{kind,field}) if kind=="custom"&&field=="label"));
    assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    rom.shutdown().await.unwrap();
    let manual = Runtime::builder()
        .resource(Manual::definition().policy(|_, _, _| true))
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    assert_eq!(
        manual
            .read::<Manual>(&actor, "c")
            .unwrap()
            .value
            .unwrap()
            .0
            .label,
        Label("custom typed value".into())
    );
    manual.shutdown().await.unwrap();
}
