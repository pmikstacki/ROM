//! Native table identities and the persisted format version.
use redb::TableDefinition;

pub(super) const ROWS: TableDefinition<(&str, &str), &str> = TableDefinition::new("resources");
pub(super) const RECEIPTS: TableDefinition<&str, &str> = TableDefinition::new("receipts");
pub(super) const EVENTS: TableDefinition<&str, &str> = TableDefinition::new("events");
pub(super) const EFFECTS: TableDefinition<(&str, u64), &str> = TableDefinition::new("effects");
pub(super) const META: TableDefinition<&str, u64> = TableDefinition::new("rom_metadata");
pub(super) const FORMAT: u64 = 5;
pub(super) const STATE: TableDefinition<&str, &str> = TableDefinition::new("rom_state");
