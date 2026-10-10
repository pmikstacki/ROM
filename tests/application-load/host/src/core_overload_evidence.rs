//! Private aggregate evidence. No request or principal attribution.
use rom::{CoreOverloadStats, IntakeState, Runtime, RuntimeStatus};
use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(crate) fn finish(
    directory: &Path,
    runtime: &Runtime,
    baseline: CoreOverloadStats,
    lifecycle_result: &rom::Result<()>,
    producers_stopped: bool,
) -> Result<()> {
    // Caller must finish all async producers BEFORE these two advisory snapshots.
    write_snapshot(
        directory,
        baseline,
        runtime.core_overload_stats(),
        runtime.status()?,
        lifecycle_result.is_ok(),
        producers_stopped,
    )
}

pub(super) fn write_snapshot(
    directory: &Path,
    baseline: CoreOverloadStats,
    final_counts: CoreOverloadStats,
    status: RuntimeStatus,
    lifecycle_succeeded: bool,
    producers_stopped: bool,
) -> Result<()> {
    let fresh_baseline = baseline == CoreOverloadStats::default();
    let overflowed = baseline.overflowed || final_counts.overflowed;
    let drained_accounting = status.intake == IntakeState::Stopped && status.owned_work == 0;
    let exact_counts = fresh_baseline && producers_stopped && drained_accounting && !overflowed;
    super::runner::write_bounded(
        directory,
        "load-final-core-overload-proof.json",
        &serde_json::json!({
            "schema": "rom-application-load-core-overload-v1",
            "scope": "all core boundary failures during Host lifecycle; not per-request attribution",
            "baseline": baseline,
            "final": final_counts,
            "overflowed": overflowed,
            "lifecycle_result": if lifecycle_succeeded { "succeeded" } else { "failed" },
            "intake": status.intake,
            "owned_work": status.owned_work,
            "drained_accounting": drained_accounting,
            "producers_stopped": producers_stopped,
            "fresh_baseline": fresh_baseline,
            "exact_counts": exact_counts,
        }),
        4096,
    )?;
    if !exact_counts {
        return Err("core overload evidence lacks exact terminal scope".into());
    }
    Ok(())
}
