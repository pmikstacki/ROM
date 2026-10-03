//! Setting state, authorization and enable behavior.
use rom::{Access, Action, Actor, Intent, Resource, Result};

#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "settings")]
pub struct Setting {
    pub owner: String,
    pub enabled: bool,
    pub attempts: u64,
}

pub fn setting_policy(actor: &Actor, _: Access, setting: &Setting) -> bool {
    actor.authority == "local" && (actor.subject == setting.owner || actor.subject == "admin")
}
fn enable(setting: &mut Setting, value: bool) -> Result<Vec<Intent>> {
    setting.enabled = value;
    setting.attempts += 1;
    Ok(vec![])
}
pub const ENABLE: Action<Setting, bool> = Action::new("enable", enable);
