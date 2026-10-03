//! Task state, authorization and completion behavior.
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
pub fn task_policy(actor: &Actor, _: Access, task: &Task) -> bool {
    actor.authority == "local" && (actor.subject == task.owner || actor.subject == "admin")
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
pub const COMPLETE: Action<Task, ()> = Action::new("complete", complete);
