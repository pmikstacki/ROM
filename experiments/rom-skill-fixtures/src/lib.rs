//! Prototype author template; trusted host registration, no custom infrastructure.
use rom::{Action, Builder, Error, Field, Presence, Resource, Result, Runtime, Shape, Value};
#[derive(Clone, Debug, PartialEq)]
pub struct Title(pub String);
impl Field for Title {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        Value::String(self.0.clone())
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_str()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| Self(v.into()))
            .ok_or_else(|| Error::invalid("title", "blank"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "tasks")]
pub struct Task {
    pub title: Title,
    pub done: bool,
    pub memo: Presence<Option<String>>,
}
#[derive(Clone, Resource)]
#[resource(name = "counters")]
pub struct Counter {
    pub count: u64,
}
pub const COMPLETE: Action<Task, ()> = Action::new("complete", |task, ()| {
    task.done = true;
    Ok(vec![])
});
/// Synthetic fixture owner; applications supply their actual policy.
pub fn definitions() -> Builder {
    Runtime::builder()
        .resource(
            Task::definition()
                .policy(|a, _, _| a.subject == "owner")
                .allow_all_fields()
                .action(COMPLETE),
        )
        .resource(
            Counter::definition()
                .policy(|a, _, _| a.subject == "owner")
                .allow_all_fields(),
        )
}
