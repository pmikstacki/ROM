//! Native table identities and the persisted format version.
use redb::TableDefinition;

pub(super) const ROWS: TableDefinition<(&str, &str), &str> = TableDefinition::new("resources");
pub(super) const RECEIPTS: TableDefinition<&str, &str> = TableDefinition::new("receipts");
pub(super) const EVENTS: TableDefinition<&str, &str> = TableDefinition::new("events");
pub(super) const EFFECTS: TableDefinition<(&str, u64), &str> = TableDefinition::new("effects");
pub(super) const META: TableDefinition<&str, u64> = TableDefinition::new("rom_metadata");
pub(super) const FORMAT: u64 = rom_backup::STORAGE_FORMAT as u64;
pub(super) const JOURNAL_FORMAT: u64 = 11;
pub(super) const POSITIONS: TableDefinition<u64, &str> = TableDefinition::new("journal_positions");
pub(super) const STATE: TableDefinition<&str, &str> = TableDefinition::new("rom_state");
pub(super) const WORK: TableDefinition<&str, &str> = TableDefinition::new("work_records");
pub(super) const ROOTS: TableDefinition<&str, &str> = TableDefinition::new("work_roots");
pub(super) const ACTIVE: TableDefinition<&str, u8> = TableDefinition::new("work_active");
