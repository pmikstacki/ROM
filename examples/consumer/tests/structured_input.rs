use rom::{Action, Actor, Command, Input, Presence, Resource, Runtime, json};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Input)]
struct ReserveInput {
    #[input(rename = "reservation-token")]
    token: String,
    quantity: u64,
    note: Presence<Option<String>>,
}

#[derive(Clone, Debug, Resource)]
#[resource(name = "inventory-input-test")]
struct Inventory {
    available: u64,
}
const RESERVE: Action<Inventory, ReserveInput> = Action::new("reserve", |stock, input| {
    stock.available = stock
        .available
        .checked_sub(input.quantity)
        .ok_or_else(|| rom::Error::invalid("inventory-input-test", "quantity"))?;
    Ok(vec![])
});

#[test]
fn object_members_preserve_omission_null_and_aliases() {
    assert_eq!(
        ReserveInput::field_names(),
        &["reservation-token", "quantity", "note"]
    );
    for note in [
        Presence::Missing,
        Presence::Value(None),
        Presence::Value(Some("note".into())),
    ] {
        let input = ReserveInput {
            token: "a".into(),
            quantity: 2,
            note,
        };
        let encoded = input.encode();
        assert_eq!(encoded["reservation-token"], "a");
        assert!(encoded.get("token").is_none());
        assert_eq!(
            encoded.get("note").is_some(),
            input.note != Presence::Missing
        );
        assert_eq!(ReserveInput::decode(encoded).unwrap(), input);
    }
    assert_eq!(
        ReserveInput::decode(json!({"reservation-token":false,"quantity":1})).unwrap_err(),
        rom::Error::invalid("input", "reservation-token")
    );
    for invalid in [
        json!(null),
        json!({"reservation-token":"a"}),
        json!({"token":"a","quantity":2}),
        json!({"reservation-token":"a","quantity":2,"extra":true}),
        json!({"reservation-token":"a","quantity":null}),
        json!({"reservation-token":"a","quantity":-1}),
        json!({"reservation-token":"a","quantity":2,"note":false}),
    ] {
        assert!(ReserveInput::decode(invalid).is_err());
    }
}

#[tokio::test]
async fn structured_input_executes_through_public_action_pipeline() {
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Inventory::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(RESERVE),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("test", "owner");
    runtime
        .execute(
            &actor,
            Command::create("stock", Inventory { available: 5 }).idempotency("create"),
        )
        .await
        .unwrap();
    let command = || {
        Command::action(
            "stock",
            RESERVE,
            ReserveInput {
                token: "a".into(),
                quantity: 2,
                note: Presence::Missing,
            },
        )
        .at_revision(1)
        .idempotency("reserve")
    };
    runtime.execute(&actor, command()).await.unwrap();
    let counts = store.counts().unwrap();
    runtime.execute(&actor, command()).await.unwrap();
    assert_eq!(store.counts().unwrap(), counts);
    assert_eq!(
        runtime
            .query(&actor, &Inventory::available_field().equals(3))
            .await
            .unwrap()
            .len(),
        1
    );
    let invalid = runtime
        .invoke(
            &actor,
            rom::Invocation {
                kind: Inventory::KIND.into(),
                id: "stock".into(),
                expected: Some(2),
                idempotency: "invalid".into(),
                operation: rom::Operation::Action {
                    name: "reserve".into(),
                    input: json!({"reservation-token":false,"quantity":1}),
                },
            },
        )
        .await;
    assert_eq!(
        invalid.unwrap_err(),
        rom::Error::invalid(Inventory::KIND, "reserve.reservation-token")
    );
    assert_eq!(store.counts().unwrap(), counts);
    runtime.shutdown().await.unwrap();
}

#[derive(Clone, Debug, PartialEq)]
struct Trimmed(String);
impl rom::Field for Trimmed {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> rom::Value {
        self.0.clone().into()
    }
    fn decode(value: rom::Value) -> rom::Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| rom::Error::invalid("input", "trimmed"))?
            .trim();
        if value.is_empty() {
            return Err(rom::Error::invalid("input", "trimmed"));
        }
        Ok(Self(value.into()))
    }
}
#[derive(Clone, Input)]
struct CustomInput {
    r#type: Trimmed,
    nullable: Option<String>,
}
#[test]
fn custom_fields_use_the_same_codec_and_required_nullable_member() {
    let payload = CustomInput::decode(json!({"type":"  resource  ","nullable":null})).unwrap();
    assert_eq!(payload.encode(), json!({"type":"resource","nullable":null}));
    assert!(CustomInput::decode(json!({"type":"resource"})).is_err());
    assert!(CustomInput::decode(json!({"type":" ","nullable":null})).is_err());
}

#[derive(Clone, Input)]
struct AmbiguousInput {
    value: Option<Option<String>>,
}
#[test]
fn invalid_field_shape_is_rejected_before_decoding() {
    assert!(AmbiguousInput::decode(json!({"value":null})).is_err());
}

#[derive(Clone, Resource)]
#[resource(name = "legacy-raw")]
struct LegacyRaw {
    r#type: String,
}
#[test]
fn existing_resource_raw_identifier_keeps_its_persisted_wire_name() {
    let legacy = json!({"r#type":"stored"});
    assert_eq!(LegacyRaw::decode(legacy.clone()).unwrap().encode(), legacy);
    assert_eq!(LegacyRaw::descriptor().fields[0].name, "r#type");
}

#[derive(Clone)]
struct LeakyInput;
impl Input for LeakyInput {
    fn encode(&self) -> rom::Value {
        json!(null)
    }
    fn decode(_: rom::Value) -> rom::Result<Self> {
        Err(rom::Error::invalid("input", "SECRET"))
    }
}
#[tokio::test]
async fn undeclared_custom_input_errors_stay_sanitized() {
    let action: Action<Inventory, LeakyInput> = Action::new("custom", |_, _| Ok(vec![]));
    let store = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Inventory::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .action(action),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let actor = Actor::trusted("test", "owner");
    runtime
        .execute(
            &actor,
            Command::create("stock", Inventory { available: 5 }).idempotency("create"),
        )
        .await
        .unwrap();
    let before = store.counts().unwrap();
    let result = runtime
        .execute(
            &actor,
            Command::action("stock", action, LeakyInput)
                .at_revision(1)
                .idempotency("bad"),
        )
        .await;
    assert_eq!(
        result.unwrap_err(),
        rom::Error::invalid(Inventory::KIND, "custom")
    );
    assert_eq!(store.counts().unwrap(), before);
    runtime.shutdown().await.unwrap();
}
