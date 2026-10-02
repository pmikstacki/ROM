use rom::{
    Actor, ActorGate, AuthorizationRead, Error, Field, Invocation, Operation, Resource, Result,
    Runtime, Shape, Value, json,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
static DECODE_CALLS: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone)]
struct Flag(bool);
impl Field for Flag {
    fn shape() -> Shape {
        Shape::Bool
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        DECODE_CALLS.fetch_add(1, Ordering::SeqCst);
        value
            .as_bool()
            .map(Self)
            .ok_or_else(|| Error::invalid("flag", "boolean required"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "private-settings")]
struct Settings {
    enabled: Flag,
}
struct CurrentAuthority;
impl ActorGate for CurrentAuthority {
    fn check(&self, actor: &Actor, _: &mut dyn AuthorizationRead) -> Result<()> {
        if actor.subject == "disabled" {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
}
fn invocation(kind: &str, operation: Operation) -> Invocation {
    Invocation {
        kind: kind.into(),
        id: "one".into(),
        expected: None,
        idempotency: "probe".into(),
        operation,
    }
}
#[tokio::test]
async fn current_denial_precedes_registry_and_all_input_codecs() {
    let runtime = Runtime::builder()
        .actor_gate(Arc::new(CurrentAuthority))
        .resource(
            Settings::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    DECODE_CALLS.store(0, Ordering::SeqCst);
    let disabled = Actor::trusted("test", "disabled");
    for kind in ["private-settings", "unknown"] {
        for value in [json!(true), json!("invalid")] {
            for operation in [
                Operation::Create(json!({"enabled":value})),
                Operation::Replace(json!({"enabled":value})),
                Operation::Patch(std::collections::BTreeMap::from([(
                    "enabled".into(),
                    rom::FieldUpdate::Set(value.clone()),
                )])),
            ] {
                let command = invocation(kind, operation);
                assert_eq!(
                    runtime.invoke_projected(&disabled, command.clone()).await,
                    Err(Error::Denied),
                    "kind={kind}"
                );
                assert_eq!(
                    runtime.invoke(&disabled, command).await,
                    Err(Error::Denied),
                    "kind={kind}"
                );
            }
        }
    }
    assert_eq!(
        DECODE_CALLS.load(Ordering::SeqCst),
        0,
        "denied identity ran application codec"
    );
    // Positive control proves the registered codec is used for an authorized request.
    runtime
        .invoke(
            &Actor::trusted("test", "allowed"),
            invocation(
                "private-settings",
                Operation::Create(json!({"enabled":true})),
            ),
        )
        .await
        .unwrap();
    assert!(DECODE_CALLS.load(Ordering::SeqCst) > 0);
    runtime.shutdown().await.unwrap();
}
