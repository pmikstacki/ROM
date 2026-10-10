use crate::{Equipment, policy::Owned};
use rom::{Action, Intent, Resource, ResourceRef, Result, json};

#[derive(Clone, Resource)]
#[resource(name = "work-orders")]
pub struct WorkOrder {
    pub owner: String,
    pub equipment: ResourceRef<Equipment>,
    pub title: String,
    pub completed: bool,
}
impl Owned for WorkOrder {
    fn owner(&self) -> &str {
        &self.owner
    }
}

pub const COMPLETE: Action<WorkOrder, ()> = Action::new("complete", complete);

fn complete(work: &mut WorkOrder, (): ()) -> Result<Vec<Intent>> {
    work.completed = true;
    Ok(vec![Intent::new(
        "maintenance-completed",
        json!({"equipment": work.equipment.id()}),
    )])
}
