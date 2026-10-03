//! Resource declarations and ordinary business functions.
use rom::{
    Action, Channel, Command, Field, Patch, Resource, Result, Shape, Snapshot, Target, Value,
};

/// Canonical stock code: uppercase ASCII letters, digits, and hyphens, at most 24 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct StockCode(String);
impl Field for StockCode {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        self.0.clone().into()
    }
    fn decode(value: Value) -> Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| rom::Error::invalid("code", "string required"))?;
        let value = value.trim().to_ascii_uppercase();
        if value.is_empty()
            || value.len() > 24
            || !value
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err(rom::Error::invalid("code", "invalid stock code"));
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "tasks")]
pub struct Task {
    pub title: String,
    pub done: bool,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "inventory")]
pub struct InventoryItem {
    pub code: StockCode,
    pub quantity: u64,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "dashboards")]
pub struct Dashboard {
    pub latest: String,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "settings")]
pub struct Settings {
    pub workshop: String,
    pub enabled: bool,
}

pub const NOTICE: Channel<String> = Channel::new("local-completions", 1);
pub const COMPLETE: Action<Task, ()> = Action::new("complete", |task, ()| {
    task.done = true;
    Ok(vec![])
});
pub const DISPLAY: Action<Dashboard, String> = Action::new("display", |dashboard, title| {
    dashboard.latest = title.clone();
    Ok(vec![NOTICE.intent(title)])
});
pub(crate) fn completed(task: &Snapshot<Task>) -> Result<Vec<Target<String>>> {
    Ok(task
        .value
        .as_ref()
        .filter(|task| task.done)
        .map(|task| vec![Target::new("workshop", task.title.clone())])
        .unwrap_or_default())
}
/// Typed patch uses the generated selector; no string field names or per-kind mutation route.
pub fn rename_task(id: &str, revision: u64, title: String) -> Command<Task> {
    Command::patch(id, Patch::new().set(Task::title_field(), title))
        .at_revision(revision)
        .idempotency("demo-rename")
}
