//! Opt-in diagnostic counters. Never release admission or raw data tracing.
use crate::database::Database;
use rom_sqlite::{StageObservation, StageSnapshot};
use serde_json::{Value, json};

pub(crate) fn enable(database: &Database) -> rom::Result<StageObservation> {
    match database {
        Database::Sqlite(database) => Ok(database.observe_stages()),
        Database::Redb(_) => Err(rom::Error::Denied),
    }
}

pub(crate) fn summary(snapshot: StageSnapshot) -> Value {
    let entries: Vec<_> = snapshot
        .entries
        .into_iter()
        .map(|entry| {
            json!({
                "operation": entry.operation.as_str(),
                "stage": entry.stage.as_str(),
                "samples": entry.samples,
                "elapsed_ns": entry.elapsed_ns,
                "dropped_samples": entry.dropped_samples,
                "saturated": entry.saturated,
            })
        })
        .collect();
    json!({"entries": entries, "poison_recovered": snapshot.poison_recovered})
}

/// Diagnostic scope begins after opening the adapter and ends only after all
/// producers stop. Snapshots are aggregate wall time, never release admission.
pub(crate) struct LifecycleObservation {
    observer: StageObservation,
    baseline: StageSnapshot,
    publication_baseline: rom_sqlite::PublicationSnapshot,
}
impl LifecycleObservation {
    pub(crate) fn begin(database: &Database) -> rom::Result<Self> {
        let observer = enable(database)?;
        Ok(Self {
            baseline: observer.snapshot(),
            publication_baseline: observer.publication_snapshot(),
            observer,
        })
    }
    pub(crate) fn finish(
        &self,
        directory: &std::path::Path,
        runtime: &rom::Runtime,
        lifecycle_result: &rom::Result<()>,
        producers_stopped: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let status = runtime.status();
        let drained = status.as_ref().is_ok_and(|status| {
            status.intake == rom::IntakeState::Stopped && status.owned_work == 0
        });
        let final_stages = self.observer.snapshot();
        let final_publications = self.observer.publication_snapshot();
        let stage_clean = [&self.baseline, &final_stages].into_iter().all(|snapshot| {
            !snapshot.poison_recovered
                && snapshot
                    .entries
                    .iter()
                    .all(|entry| entry.dropped_samples == 0 && !entry.saturated)
        });
        let publication_clean = [&self.publication_baseline, &final_publications]
            .into_iter()
            .all(|snapshot| {
                !snapshot.poison_recovered
                    && snapshot
                        .entries
                        .iter()
                        .all(|entry| entry.dropped_samples == 0 && !entry.saturated)
            });
        let fresh_baseline = self
            .baseline
            .entries
            .iter()
            .all(|entry| entry.samples == 0 && entry.elapsed_ns == 0)
            && self
                .publication_baseline
                .entries
                .iter()
                .all(|entry| entry.samples == 0 && entry.encoded_bytes == 0);
        let complete =
            producers_stopped && drained && fresh_baseline && stage_clean && publication_clean;
        let mut stages = json!({
            "schema": "rom-application-load-terminal-storage-stages-v1",
            "scope": "SQLite adapter stages after open through Host and worker shutdown; not per-request attribution",
            "semantics": "fixed-category-adapter-stage-wall-time",
            "diagnostic_only": true,
            "acceptance": false,
            "observer_enabled": true,
            "exclusive_cpu_time": false,
            "boundary_totals_include_these_stages": true,
            "baseline": summary(self.baseline.clone()),
            "final": summary(final_stages),
        });
        let mut publications = crate::publication_measurements::proof(
            self.publication_baseline.clone(),
            &[],
            final_publications,
        );
        publications["schema"] = json!("rom-application-load-terminal-storage-publication-v1");
        publications["diagnostic_only"] = json!(true);
        publications["observer_enabled"] = json!(true);
        for proof in [&mut stages, &mut publications] {
            proof["lifecycle_result"] = json!(if lifecycle_result.is_ok() {
                "succeeded"
            } else {
                "failed"
            });
            proof["intake"] = json!(status.as_ref().ok().map(|status| status.intake));
            proof["owned_work"] = json!(status.as_ref().ok().map(|status| status.owned_work));
            proof["status_available"] = json!(status.is_ok());
            proof["producers_stopped"] = json!(producers_stopped);
            proof["drained_accounting"] = json!(drained);
            proof["fresh_baseline"] = json!(fresh_baseline);
            proof["overflowed"] = json!(!stage_clean || !publication_clean);
            proof["complete"] = json!(complete);
        }
        // Attempt both fixed files even when the first create/write/sync fails.
        let stage_result = super::runner::write_bounded(
            directory,
            "load-final-storage-stages.json",
            &stages,
            16 * 1024,
        );
        let publication_result = super::runner::write_bounded(
            directory,
            "load-final-storage-publication.json",
            &publications,
            16 * 1024,
        );
        stage_result.and(publication_result)?;
        if !complete {
            return Err("terminal SQLite stage evidence lacks complete quiescent scope".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "stage_measurements_tests.rs"]
mod tests;
