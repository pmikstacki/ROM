//! Boundary counts retain failed prefixes without assigning a returned width.
use crate::work_batch_observation::{Snapshot, WidthCounts};
use serde_json::{Value, json};

pub(super) fn clean(snapshot: Option<&Snapshot>) -> bool {
    snapshot.is_some_and(|snapshot| {
        !snapshot.unavailable
            && snapshot.prefix.len() <= 33
            && snapshot.atomic.len() <= 33
            && snapshot
                .prefix
                .iter()
                .chain(&snapshot.atomic)
                .all(|entry| entry.width <= 32)
    })
}
pub(super) fn fresh(snapshot: Option<&Snapshot>) -> bool {
    clean(snapshot)
        && snapshot.is_some_and(|snapshot| {
            snapshot.prefix_failed_calls == 0
                && snapshot.prefix_failed_ns == 0
                && snapshot
                    .prefix
                    .iter()
                    .chain(&snapshot.atomic)
                    .all(|entry| !nonzero(entry))
        })
}
fn nonzero(entry: &WidthCounts) -> bool {
    entry.calls != 0
        || entry.failures != 0
        || entry.elapsed_ns != 0
        || entry.source_claims != 0
        || entry.notification_claims != 0
        || entry.resolution_only_claims != 0
}
fn cells(entries: &[WidthCounts]) -> Vec<Value> {
    entries
        .iter()
        .filter(|entry| nonzero(entry))
        .map(|entry| {
            json!({
                "width": entry.width,
                "calls": entry.calls.to_string(), "failures": entry.failures.to_string(),
                "elapsed_ns": entry.elapsed_ns.to_string(),
                "source_claims": entry.source_claims.to_string(),
                "notification_claims": entry.notification_claims.to_string(),
                "resolution_only_claims": entry.resolution_only_claims.to_string(),
            })
        })
        .collect()
}
pub(super) fn summary(snapshot: Option<&Snapshot>) -> Value {
    match snapshot {
        Some(snapshot) => json!({
            "prefix": cells(&snapshot.prefix), "atomic": cells(&snapshot.atomic),
            "prefix_failed_calls": snapshot.prefix_failed_calls.to_string(),
            "prefix_failed_ns": snapshot.prefix_failed_ns.to_string(),
            "unavailable": snapshot.unavailable,
        }),
        None => json!({"unavailable": true}),
    }
}
