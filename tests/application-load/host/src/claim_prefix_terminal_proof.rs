//! Independent terminal grouped-prefix diagnostics; not mixed acceptance.
mod format;
#[cfg(test)]
mod tests;

use crate::database::Database;
use rom_sqlite::{ClaimPrefixSnapshot, StageObservation};
use serde_json::{Value, json};
use std::path::Path;

pub(crate) const MAXIMUM_BYTES: usize = 512 * 1024;
pub(crate) struct ClaimPrefixTerminalObservation {
    observer: StageObservation,
    baseline: ClaimPrefixSnapshot,
}
impl ClaimPrefixTerminalObservation {
    pub(crate) fn begin(database: &Database) -> rom::Result<Self> {
        let observer = crate::stage_measurements::enable(database)?;
        Ok(Self {
            baseline: observer.claim_prefix_snapshot(),
            observer,
        })
    }
    pub(crate) fn finish(
        &self,
        directory: &Path,
        runtime: &rom::Runtime,
        lifecycle_result: &rom::Result<()>,
        producers_stopped: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let status = runtime.status();
        let drained = status.as_ref().is_ok_and(|status| {
            status.intake == rom::IntakeState::Stopped && status.owned_work == 0
        });
        let final_snapshot = self.observer.claim_prefix_snapshot();
        let value = proof(
            Some(&self.baseline),
            Some(&final_snapshot),
            TerminalState {
                lifecycle_succeeded: lifecycle_result.is_ok(),
                producers_stopped,
                drained,
                status_available: status.is_ok(),
                intake: status.as_ref().ok().map(|status| status.intake),
                owned_work: status.as_ref().ok().map(|status| status.owned_work),
            },
        );
        crate::runner::write_bounded(
            directory,
            "load-final-claim-prefix.json",
            &value,
            MAXIMUM_BYTES,
        )?;
        if value["complete"] != true {
            return Err("terminal claim-prefix diagnostics lack complete quiescent scope".into());
        }
        Ok(())
    }
}
// Shared formatter seam permits bounded invalid/unavailable controls without a
// fake Runtime shutdown receipt. Actual finish obtains status from the Runtime.
struct TerminalState {
    lifecycle_succeeded: bool,
    producers_stopped: bool,
    drained: bool,
    status_available: bool,
    intake: Option<rom::IntakeState>,
    owned_work: Option<usize>,
}
fn proof(
    baseline: Option<&ClaimPrefixSnapshot>,
    final_snapshot: Option<&ClaimPrefixSnapshot>,
    state: TerminalState,
) -> Value {
    let fresh = format::fresh(baseline);
    let clean = format::clean(baseline) && format::clean(final_snapshot);
    let accounting_matches = state.status_available
        && state.intake == Some(rom::IntakeState::Stopped)
        && state.owned_work == Some(0);
    json!({
        "schema":"rom-application-load-terminal-claim-prefix-v1",
        "scope":"SQLite grouped native claim-prefix path after open through Host and worker shutdown",
        "semantics":"grouped-prefix-termination-and-native-transaction-outcome",
        "width_semantics":"staged-claims-excluding-withheld-candidate",
        "grouped_journal_only":true,"singleton_compatibility_included":false,
        "maximum_cells":format::MAXIMUM_CELLS,
        "diagnostic_only":true,"acceptance":false,"observer_enabled":true,
        "acknowledgement_attribution":false,"wal_or_fsync_attribution":false,"exclusive_cpu_time":false,
        "lifecycle_result":if state.lifecycle_succeeded {"succeeded"} else {"failed"},
        "intake":state.intake,"owned_work":state.owned_work,"status_available":state.status_available,
        "producers_stopped":state.producers_stopped,"drained_accounting":state.drained,
        "baseline":format::summary(baseline),"final":format::summary(final_snapshot),
        "fresh_baseline":fresh,"clean":clean,
        "complete":state.lifecycle_succeeded && state.producers_stopped && state.drained && accounting_matches && fresh && clean,
    })
}
