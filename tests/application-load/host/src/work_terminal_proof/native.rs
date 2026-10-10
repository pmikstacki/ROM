//! Fixed native cells and precision-preserving numeric totals.
use rom_sqlite::{SemanticEffect, WorkCommitOrigin, WorkUtilizationSnapshot};
use serde_json::{Value, json};

pub(super) fn fresh(snapshot: &WorkUtilizationSnapshot) -> bool {
    clean(snapshot) && snapshot.entries.is_empty()
}
pub(super) fn clean(snapshot: &WorkUtilizationSnapshot) -> bool {
    !snapshot.overflow
        && !snapshot.unavailable
        && snapshot.entries.len() <= 462
        && snapshot
            .entries
            .iter()
            .all(|entry| entry.width <= 32 && entry.dropped_samples == 0)
}
pub(super) fn summary(snapshot: &WorkUtilizationSnapshot) -> Value {
    let entries: Vec<_> = snapshot
        .entries
        .iter()
        .map(|entry| {
            let origin = match entry.origin {
                WorkCommitOrigin::ClaimPrefix => "claim_prefix",
                WorkCommitOrigin::Claim => "claim",
                WorkCommitOrigin::Materialize => "materialize",
                WorkCommitOrigin::Finish => "finish",
                WorkCommitOrigin::DeliveryStarted => "delivery_started",
                WorkCommitOrigin::DeliveryFinished => "delivery_finished",
                WorkCommitOrigin::Other => "other",
            };
            let semantic = match entry.semantic {
                SemanticEffect::Unchanged => "unchanged",
                SemanticEffect::Changed => "changed",
            };
            json!({
                "origin": origin, "width": entry.width, "semantic": semantic,
                "native_successes": entry.native_successes.to_string(),
                "native_failures": entry.native_failures.to_string(),
                "elapsed_ns": entry.elapsed_ns.to_string(),
                "dropped_samples": entry.dropped_samples.to_string(),
            })
        })
        .collect();
    json!({"entries": entries, "overflow": snapshot.overflow, "unavailable": snapshot.unavailable})
}
