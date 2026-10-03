//! Offline upgrade of the actual reference application, including pending work and folder bytes.
mod checks;
pub(crate) mod legacy;
mod native;

use crate::{Notices, attachments, build, reference, session_actor, smoke::SmokeResult};
use native::Database;
use rom::{Result, Runtime, Storage};
use std::{path::Path, sync::Arc};

/// Frozen version-1 declarations share the application's other definitions and policies.
pub fn build_legacy(storage: Arc<dyn Storage>, notices: Notices) -> Result<Runtime> {
    legacy::declarations(notices)?.build(storage, Runtime::shared_cpu_pool(2)?)
}
/// Open the old application profile. Useful when the host owns process shutdown.
pub fn legacy_runtime(redb: bool, database: &Path) -> Result<Runtime> {
    build_legacy(
        Database::open(redb, database)?.storage(),
        Notices::default(),
    )
}
/// Explicit representation conversion with frozen pending-work compatibility checks.
pub fn migration_plan() -> Result<rom_backup::MigrationPlan> {
    legacy::migration_plan()
}
/// Leave a committed rejection pending as the final operation, with an independent reference fixture.
pub async fn prepare_scenario(runtime: &Runtime) -> Result<()> {
    checks::seed_restrict(runtime).await?;
    reference::prepare(runtime).await
}
/// Run the old profile and stop cleanly. Tests can instead exit after prepare_scenario.
pub async fn prepare(redb: bool, database: &Path, objects: &Path) -> SmokeResult<()> {
    let runtime = legacy_runtime(redb, database)?;
    let blobs = attachments::open(runtime.clone(), objects)?;
    attachments::attach(&blobs).await?;
    prepare_scenario(&runtime).await?;
    blobs.shutdown().await?;
    drop(blobs);
    runtime.shutdown().await?;
    Ok(())
}
/// Migrate an offline source, back it up, restore it, and recover with current declarations.
/// External attachment bytes stay in their original folder; the archive does not contain them.
pub async fn recover(
    redb: bool,
    source: &Path,
    migrated: &Path,
    archive: &Path,
    restored: &Path,
    objects: &Path,
) -> SmokeResult<()> {
    let original = native::source_bytes(source)?;
    let migrated_db = Database::migrate(redb, source, migrated, &migration_plan()?)?;
    migrated_db.backup(archive)?;
    drop(migrated_db);
    let restored_db = Database::restore(redb, archive, restored)?;
    let runtime = build(restored_db.storage(), Notices::default())?;
    let blobs = attachments::open(runtime.clone(), objects)?;
    checks::require(
        blobs.read(&session_actor(), attachments::ID).await? == attachments::CONTENT,
        "separately retained attachment bytes",
    )?;
    reference::recover(&runtime).await?;
    checks::rebuilt_restrict(&runtime).await?;
    checks::old_receipt_replay(&runtime, &restored_db).await?;
    blobs.shutdown().await?;
    drop(blobs);
    runtime.shutdown().await?;
    checks::require(
        native::source_bytes(source)? == original,
        "offline source bytes must remain unchanged",
    )?;
    Ok(())
}
/// Finite scratch-directory journey for both supported database adapters.
pub async fn run(redb: bool) -> SmokeResult<()> {
    let scratch = native::Scratch::new()?;
    let source = scratch.0.join("source");
    let objects = scratch.0.join("objects");
    prepare(redb, &source, &objects).await?;
    recover(
        redb,
        &source,
        &scratch.0.join("migrated"),
        &scratch.0.join("archive"),
        &scratch.0.join("restored"),
        &objects,
    )
    .await
}
