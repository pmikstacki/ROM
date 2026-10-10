//! Fixed grouped-native cells; never caller acknowledgement or fsync evidence.
use rom_sqlite::{
    ClaimPrefixDisposition as Disposition, ClaimPrefixMeasurement, ClaimPrefixReason as Reason,
    ClaimPrefixSnapshot,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) const MAXIMUM_CELLS: usize = 693;
fn valid_cell(entry: &ClaimPrefixMeasurement) -> bool {
    if entry.width > 32
        || (entry.native_successes == 0 && entry.native_failures == 0 && entry.dropped_samples == 0)
    {
        return false;
    }
    match (entry.reason, entry.disposition) {
        (Reason::RequestedLimit, Disposition::Unrestricted) => entry.width >= 2,
        (Reason::Idle | Reason::MaintenanceChanged, Disposition::Unrestricted) => entry.width < 32,
        (
            Reason::ResolutionBarrier | Reason::NotificationBarrier | Reason::ActionBarrier,
            Disposition::AdmittedSingleton,
        ) => entry.width == 1,
        (
            Reason::ResolutionBarrier
            | Reason::NotificationBarrier
            | Reason::ActionBarrier
            | Reason::DuplicateRoot,
            Disposition::Withheld,
        ) => entry.width > 0 && entry.width < 32,
        _ => false,
    }
}
fn valid(snapshot: &ClaimPrefixSnapshot) -> bool {
    if snapshot.entries.len() > MAXIMUM_CELLS {
        return false;
    }
    let mut seen = BTreeSet::new();
    snapshot.entries.iter().all(|entry| {
        valid_cell(entry)
            && seen.insert((
                entry.reason as usize,
                entry.width,
                entry.disposition as usize,
            ))
    })
}
pub(super) fn clean(snapshot: Option<&ClaimPrefixSnapshot>) -> bool {
    snapshot.is_some_and(|snapshot| {
        !snapshot.overflow
            && !snapshot.unavailable
            && valid(snapshot)
            && snapshot
                .entries
                .iter()
                .all(|entry| entry.dropped_samples == 0)
    })
}
pub(super) fn fresh(snapshot: Option<&ClaimPrefixSnapshot>) -> bool {
    clean(snapshot) && snapshot.is_some_and(|snapshot| snapshot.entries.is_empty())
}
pub(super) fn summary(snapshot: Option<&ClaimPrefixSnapshot>) -> Value {
    let Some(snapshot) = snapshot else {
        return json!({"available":false,"entries":null,"overflow":null,"unavailable":true,"valid":false});
    };
    if snapshot.entries.len() > MAXIMUM_CELLS {
        // Refuse the entire invalid array, never truncate it to plausible evidence.
        return json!({"available":true,"entries":null,"overflow":snapshot.overflow,"unavailable":snapshot.unavailable,"valid":false});
    }
    let entries: Vec<_> = snapshot
        .entries
        .iter()
        .map(|entry| {
            let reason = match entry.reason {
                Reason::RequestedLimit => "requested_limit",
                Reason::Idle => "idle",
                Reason::MaintenanceChanged => "maintenance_changed",
                Reason::ResolutionBarrier => "resolution_barrier",
                Reason::NotificationBarrier => "notification_barrier",
                Reason::ActionBarrier => "action_barrier",
                Reason::DuplicateRoot => "duplicate_root",
            };
            let disposition = match entry.disposition {
                Disposition::Unrestricted => "unrestricted",
                Disposition::AdmittedSingleton => "admitted_singleton",
                Disposition::Withheld => "withheld",
            };
            json!({"reason":reason,"width":entry.width,"disposition":disposition,
            "native_successes":entry.native_successes.to_string(),
            "native_failures":entry.native_failures.to_string(),
            "dropped_samples":entry.dropped_samples.to_string()})
        })
        .collect();
    json!({"available":true,"entries":entries,"overflow":snapshot.overflow,
        "unavailable":snapshot.unavailable,"valid":valid(snapshot)})
}
