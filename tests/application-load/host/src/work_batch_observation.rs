//! Bounded boundary histograms; never native commit or synchronization attribution.
use rom::{WorkClaim, WorkPayload, WorkResult};
use serde::Serialize;
use std::{sync::Mutex, time::Duration};

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct WidthCounts {
    pub width: usize,
    pub calls: u64,
    pub failures: u64,
    pub elapsed_ns: u64,
    pub source_claims: u64,
    pub notification_claims: u64,
    pub resolution_only_claims: u64,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub prefix: Vec<WidthCounts>,
    pub atomic: Vec<WidthCounts>,
    /// Failed calls have no confirmed returned width; do not count them as idle.
    pub prefix_failed_calls: u64,
    pub prefix_failed_ns: u64,
    pub unavailable: bool,
}
struct Counts {
    prefix: [WidthCounts; 33],
    atomic: [WidthCounts; 33],
    prefix_failed_calls: u64,
    prefix_failed_ns: u64,
    unavailable: bool,
}
pub struct WorkBatchObservation {
    counts: Mutex<Counts>,
}
impl Default for WorkBatchObservation {
    fn default() -> Self {
        let widths = || {
            std::array::from_fn(|width| WidthCounts {
                width,
                ..Default::default()
            })
        };
        Self {
            counts: Mutex::new(Counts {
                prefix: widths(),
                atomic: widths(),
                prefix_failed_calls: 0,
                prefix_failed_ns: 0,
                unavailable: false,
            }),
        }
    }
}
impl WorkBatchObservation {
    pub fn prefix(&self, result: &rom::Result<Vec<WorkClaim>>, elapsed: Duration) {
        let Ok(mut counts) = self.counts.lock() else {
            return;
        };
        let Ok(ns) = u64::try_from(elapsed.as_nanos()) else {
            counts.unavailable = true;
            return;
        };
        match result {
            Ok(claims) => {
                if let Some(cell) = counts.prefix.get_mut(claims.len()) {
                    if !add(cell, ns, false, claims.iter()) {
                        counts.unavailable = true;
                    }
                } else {
                    counts.unavailable = true;
                }
            }
            Err(_) => {
                match counts
                    .prefix_failed_calls
                    .checked_add(1)
                    .zip(counts.prefix_failed_ns.checked_add(ns))
                {
                    Some((calls, elapsed)) => {
                        counts.prefix_failed_calls = calls;
                        counts.prefix_failed_ns = elapsed;
                    }
                    None => counts.unavailable = true,
                }
            }
        }
    }
    pub fn atomic(&self, width: usize, result: &rom::Result<Vec<WorkResult>>, elapsed: Duration) {
        let Ok(mut counts) = self.counts.lock() else {
            return;
        };
        let Ok(ns) = u64::try_from(elapsed.as_nanos()) else {
            counts.unavailable = true;
            return;
        };
        let Some(cell) = counts.atomic.get_mut(width) else {
            counts.unavailable = true;
            return;
        };
        let claims = result.iter().flatten().filter_map(|result| match result {
            WorkResult::Claimed(claim) => Some(claim.as_ref()),
            WorkResult::Changed | WorkResult::Idle => None,
        });
        if !add(cell, ns, result.is_err(), claims) {
            counts.unavailable = true;
        }
    }
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.counts.lock().ok().map(|counts| Snapshot {
            prefix: counts.prefix.to_vec(),
            atomic: counts.atomic.to_vec(),
            prefix_failed_calls: counts.prefix_failed_calls,
            prefix_failed_ns: counts.prefix_failed_ns,
            unavailable: counts.unavailable,
        })
    }
}
/// Commit all counters together. An exhausted counter cannot produce partial evidence.
fn add<'a>(
    cell: &mut WidthCounts,
    ns: u64,
    failed: bool,
    claims: impl Iterator<Item = &'a WorkClaim>,
) -> bool {
    let mut next = *cell;
    let Some(calls) = next.calls.checked_add(1) else {
        return false;
    };
    let Some(failures) = next.failures.checked_add(u64::from(failed)) else {
        return false;
    };
    let Some(elapsed) = next.elapsed_ns.checked_add(ns) else {
        return false;
    };
    next.calls = calls;
    next.failures = failures;
    next.elapsed_ns = elapsed;
    for claim in claims {
        let subtype = match claim.work.pending.payload {
            WorkPayload::Source(_) => Some(&mut next.source_claims),
            WorkPayload::Notification { .. } => Some(&mut next.notification_claims),
            WorkPayload::Action(_) => None,
        };
        if let Some(value) = subtype {
            let Some(updated) = value.checked_add(1) else {
                return false;
            };
            *value = updated;
        }
        if claim.resolution_only {
            let Some(updated) = next.resolution_only_claims.checked_add(1) else {
                return false;
            };
            next.resolution_only_claims = updated;
        }
    }
    *cell = next;
    true
}
#[cfg(test)]
#[path = "work_batch_observation_tests.rs"]
mod tests;
