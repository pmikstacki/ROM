use crate::policy::Owned;
use rom::{Resource, ResourceRef};
use rom_fields::DateTime;

#[derive(Clone, Resource)]
#[resource(name = "equipment")]
pub struct Equipment {
    pub owner: String,
    pub title: String,
    pub active: bool,
}
impl Owned for Equipment {
    fn owner(&self) -> &str {
        &self.owner
    }
}

#[derive(Clone, Resource)]
#[resource(name = "inspections")]
pub struct Inspection {
    pub owner: String,
    pub equipment: ResourceRef<Equipment>,
    pub notes: String,
    pub inspected_at: DateTime,
}
impl Owned for Inspection {
    fn owner(&self) -> &str {
        &self.owner
    }
}

#[derive(Clone, Resource)]
#[resource(name = "portal-settings")]
pub struct PortalSettings {
    pub owner: String,
    pub show_history: bool,
    pub columns: u64,
    pub layout: crate::WorkspaceLayout,
}
impl Owned for PortalSettings {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn validate(&self) -> rom::Result<()> {
        self.layout.validate_for(self.columns)
    }
}
