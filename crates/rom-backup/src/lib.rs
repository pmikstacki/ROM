//! Bounded private logical maintenance archives. Never exposes archives through Resource APIs.
//! SHA-256 detects accidental corruption, not hostile modification. Use a trusted parent
//! directory. Archives contain protected values; external blobs and deliveries are excluded.

mod archive;
mod codec;
mod collector;
#[cfg(test)]
mod epoch_upgrade_tests;
mod legacy;
mod maintenance_limits;
mod migration;
mod migration_plan;
mod model;
mod publication;
mod retention;
mod retention_policy;
mod schema;
#[cfg(test)]
mod tests;

pub use archive::{read, write};
pub use collector::Collector;
pub use legacy::{
    bind_legacy_schema, upgrade_current_snapshot, upgrade_legacy_snapshot, upgrade_v1_archive,
    upgrade_v2_archive, upgrade_v3_archive, upgrade_v4_archive, validate_legacy_retry_epochs,
};
pub use migration::migrate_snapshot;
pub use migration_plan::{MigrationPlan, ResourceMigration};
pub use model::{Backend, BackupLimits, Manifest, STORAGE_FORMAT, Snapshot, StoredEffect};
pub use publication::Stage;
pub use retention::retain_snapshot;
pub use retention_policy::{RetentionPolicy, RetentionReport};
