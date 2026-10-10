//! Aggregate terminal diagnostics, never workload or release acceptance.
mod host;
mod native;

use crate::{ObservedStorage, database::Database, work_batch_observation::Snapshot};
use rom_sqlite::{StageObservation, WorkUtilizationSnapshot};
use serde_json::json;
use std::path::Path;

pub(crate) struct WorkTerminalObservation {
    observer: StageObservation,
    native_baseline: WorkUtilizationSnapshot,
    host_baseline: Option<Snapshot>,
}
impl WorkTerminalObservation {
    pub(crate) fn begin(database: &Database, observed: &ObservedStorage) -> rom::Result<Self> {
        let observer = crate::stage_measurements::enable(database)?;
        Ok(Self {
            native_baseline: observer.work_utilization_snapshot(),
            host_baseline: observed.work_batches(),
            observer,
        })
    }
    pub(crate) fn finish(
        &self,
        directory: &Path,
        observed: &ObservedStorage,
        runtime: &rom::Runtime,
        lifecycle_result: &rom::Result<()>,
        producers_stopped: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let status = runtime.status();
        let drained = status.as_ref().is_ok_and(|status| {
            status.intake == rom::IntakeState::Stopped && status.owned_work == 0
        });
        let final_native = self.observer.work_utilization_snapshot();
        let final_host = observed.work_batches();
        let native_fresh = native::fresh(&self.native_baseline);
        let host_fresh = host::fresh(self.host_baseline.as_ref());
        let native_clean = native::clean(&self.native_baseline) && native::clean(&final_native);
        let host_clean =
            host::clean(self.host_baseline.as_ref()) && host::clean(final_host.as_ref());
        let native_complete = producers_stopped && drained && native_fresh && native_clean;
        let host_complete = producers_stopped && drained && host_fresh && host_clean;
        let common = json!({
            "diagnostic_only": true,
            "acceptance": false,
            "observer_enabled": true,
            "exclusive_cpu_time": false,
            "lifecycle_result": if lifecycle_result.is_ok() { "succeeded" } else { "failed" },
            "intake": status.as_ref().ok().map(|status| status.intake),
            "owned_work": status.as_ref().ok().map(|status| status.owned_work),
            "status_available": status.is_ok(),
            "producers_stopped": producers_stopped,
            "drained_accounting": drained,
        });
        let mut native = common.clone();
        native.as_object_mut().unwrap().extend(json!({
            "schema": "rom-application-load-terminal-work-utilization-v1",
            "scope": "SQLite native Work commit intervals after open through Host and worker shutdown",
            "semantics": "native-commit-wall-time-and-prepared-semantic-change",
            "wal_or_fsync_attribution": false,
            "baseline": native::summary(&self.native_baseline),
            "final": native::summary(&final_native),
            "fresh_baseline": native_fresh,
            "complete": native_complete,
        }).as_object().unwrap().clone());
        let mut host = common;
        host.as_object_mut().unwrap().extend(json!({
            "schema": "rom-application-load-terminal-work-batches-v1",
            "scope": "Host Storage Work boundary calls after wrapper construction through Host and worker shutdown",
            "semantics": "returned-prefix-and-requested-atomic-width",
            "native_commit_attribution": false,
            "unknown_outcome_classification": "unavailable",
            "unavailable_cause_classification": "unavailable",
            "baseline": host::summary(self.host_baseline.as_ref()),
            "final": host::summary(final_host.as_ref()),
            "fresh_baseline": host_fresh,
            "complete": host_complete,
        }).as_object().unwrap().clone());
        // Evaluate both writers even when the first exclusive create or sync fails.
        let native_result = crate::runner::write_bounded(
            directory,
            "load-final-work-utilization.json",
            &native,
            128 * 1024,
        );
        let host_result = crate::runner::write_bounded(
            directory,
            "load-final-work-batches.json",
            &host,
            32 * 1024,
        );
        native_result.and(host_result)?;
        if !native_complete || !host_complete {
            return Err("terminal Work diagnostics lack complete quiescent scope".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "work_terminal_formatter_tests.rs"]
mod tests;
