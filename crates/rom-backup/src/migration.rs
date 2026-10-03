//! Shared conversion of the complete bounded Resource history and obligation graph.
use crate::maintenance_limits::check_snapshot as check_limits;
use crate::{BackupLimits, MigrationPlan, Snapshot, codec};
use rom::{Error, Result, WorkState};
use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
};

/// Convert representations without publication, revision changes or new business events.
/// A native adapter must restore the result into a fresh destination to fence old claims.
pub fn migrate_snapshot(
    mut snapshot: Snapshot,
    plan: &MigrationPlan,
    limits: BackupLimits,
) -> Result<Snapshot> {
    check_limits(&snapshot, limits)?;
    snapshot.validate()?;
    for step in &plan.steps {
        let index = snapshot
            .descriptors
            .iter()
            .position(|d| d.kind == step.before.kind)
            .ok_or(Error::Unregistered)?;
        if snapshot.descriptors[index] != step.before {
            return Err(Error::Unsupported(
                "migration source descriptor mismatch".into(),
            ));
        }
        // None is an unmigrated receipt. Bind its original codec exactly once.
        for receipt in &mut snapshot.receipts {
            if receipt.row.key.kind == step.before.kind && receipt.replay_version.is_none() {
                receipt.replay_version = Some(step.before.version);
            }
        }
        // Memoize each distinct input so repeated historical copies cannot diverge.
        // Both sides of the cache have their own logical byte budget.
        let mut converted = BTreeMap::new();
        let mut input_bytes = 0usize;
        let mut output_bytes = 0usize;
        let mut convert = |key: &rom::Key, value: &rom::Value| {
            if key.kind != step.before.kind {
                return Ok(value.clone());
            }
            let input = codec::encode(value, limits.max_bytes)?;
            if let Some(output) = converted.get(&input) {
                return Ok(rom::Value::clone(output));
            }
            input_bytes = input_bytes
                .checked_add(input.len())
                .ok_or(Error::TooLarge)?;
            if input_bytes > limits.max_bytes {
                return Err(Error::TooLarge);
            }
            step.before.reference_targets(Some(value))?;
            let output = catch_unwind(AssertUnwindSafe(|| (step.convert)(value.clone())))
                .unwrap_or(Err(Error::Panicked))?;
            step.after.reference_targets(Some(&output))?;
            output_bytes = output_bytes
                .checked_add(codec::encode(&output, limits.max_bytes)?.len())
                .ok_or(Error::TooLarge)?;
            if output_bytes > limits.max_bytes {
                return Err(Error::TooLarge);
            }
            converted.insert(input, output.clone());
            Ok(output)
        };
        for row in &mut snapshot.rows {
            *row = row.map_resource_values(&mut convert)?;
        }
        for receipt in &mut snapshot.receipts {
            receipt.row = receipt.row.map_resource_values(&mut convert)?;
        }
        for (_, row) in &mut snapshot.events {
            *row = row.map_resource_values(&mut convert)?;
        }
        snapshot.state.map_resource_values(&mut convert)?;
        snapshot.descriptors[index] = step.after.clone();
        crate::schema::rebuild_references(&mut snapshot, limits)?;
        snapshot.validate()?;
        check_limits(&snapshot, limits)?;
    }
    for record in snapshot.state.work.records() {
        if !matches!(record.state, WorkState::Done) {
            let validate = plan.work_validator.ok_or_else(|| {
                Error::Unsupported(
                    "migration requires unfinished-work compatibility validation".into(),
                )
            })?;
            catch_unwind(AssertUnwindSafe(|| validate(&record.pending)))
                .unwrap_or(Err(Error::Panicked))?;
        }
    }
    Ok(snapshot)
}
