use crate::Code;
use rom::Resource;

#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "example-inventory", version = 1)]
pub struct Inventory {
    pub code: Code,
    pub available: u64,
}

#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "example-notes", version = 1)]
pub struct Note {
    pub message: String,
}
