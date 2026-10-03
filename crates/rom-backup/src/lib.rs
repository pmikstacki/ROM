//! Bounded private logical maintenance archives. Never exposes archives through Resource APIs.
//! SHA-256 detects accidental corruption, not hostile modification. Use a trusted parent
//! directory. Archives contain protected values; external blobs and deliveries are excluded.

mod archive;
mod codec;
mod collector;
mod legacy;
mod migration;
mod migration_plan;
mod model;
mod publication;
mod schema;
#[cfg(test)]
mod tests;

pub use archive::{read, write};
pub use collector::Collector;
pub use legacy::{
    bind_legacy_schema, upgrade_legacy_snapshot, upgrade_v1_archive, upgrade_v2_archive,
};
pub use migration::migrate_snapshot;
pub use migration_plan::{MigrationPlan, ResourceMigration};
pub use model::{Backend, BackupLimits, Manifest, Snapshot, StoredEffect};
pub use publication::Stage;
