mod files;
mod identity;
mod model;
mod storage;
pub use files::Scratch;
pub use identity::{
    activation, after, base, enabled, human, identity_targets, provision, record_definition,
    shutdown,
};
pub use model::{Ledger, Record, RecordV2, conversion, plan};
pub use storage::Database;
pub const NOW: u64 = 1_800_000_000;
pub const BOUND: std::time::Duration = std::time::Duration::from_secs(3);
