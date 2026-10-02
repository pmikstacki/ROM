//! External application consumer: declarations and business functions, no repositories/controllers.
use rom::{Access, Action, Actor, Intent, Resource, Result, json};
#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "tasks")]
pub struct Task {
    pub owner: String,
    #[resource(rename = "display-title")]
    pub title: String,
    pub done: bool,
    pub note: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "settings")]
pub struct Setting {
    pub owner: String,
    pub enabled: bool,
    pub attempts: u64,
}

pub fn task_policy(actor: &Actor, _: Access, task: &Task) -> bool {
    actor.authority == "local" && (actor.subject == task.owner || actor.subject == "admin")
}
pub fn setting_policy(actor: &Actor, _: Access, setting: &Setting) -> bool {
    actor.authority == "local" && (actor.subject == setting.owner || actor.subject == "admin")
}
fn complete(task: &mut Task, _: ()) -> Result<Vec<Intent>> {
    if task.done {
        return Ok(vec![]);
    }
    task.done = true;
    Ok(vec![Intent::new(
        "task-completed",
        json!({"title":task.title}),
    )])
}
fn enable(setting: &mut Setting, value: bool) -> Result<Vec<Intent>> {
    setting.enabled = value;
    setting.attempts += 1;
    Ok(vec![])
}
pub const COMPLETE: Action<Task, ()> = Action::new("complete", complete);
pub const ENABLE: Action<Setting, bool> = Action::new("enable", enable);
pub fn declarations() -> rom::Builder {
    rom::Runtime::builder()
        .resource(Task::definition().policy(task_policy).action(COMPLETE))
        .resource(Setting::definition().policy(setting_policy).action(ENABLE))
}
