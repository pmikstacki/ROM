//! Bounded semantic Work commit diagnostics, never a durability or fsync claim.
use super::StageObservation;
use rom::{WorkUpdate, storage_support::work::WorkDelta};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkCommitOrigin {
    ClaimPrefix,
    Claim,
    Materialize,
    Finish,
    DeliveryStarted,
    DeliveryFinished,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticEffect {
    Unchanged,
    Changed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkCommitMeasurement {
    pub origin: WorkCommitOrigin,
    pub width: u8,
    pub semantic: SemanticEffect,
    pub native_successes: u64,
    pub native_failures: u64,
    pub elapsed_ns: u64,
    pub dropped_samples: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkUtilizationSnapshot {
    /// Only nonzero fixed buckets; at most 7 origins × 33 widths × 2 effects.
    pub entries: Vec<WorkCommitMeasurement>,
    pub overflow: bool,
    pub unavailable: bool,
}

#[derive(Clone, Copy, Default)]
struct Bucket {
    successes: u64,
    failures: u64,
    elapsed_ns: u64,
    dropped: u64,
}
pub(super) struct Aggregate {
    buckets: [Bucket; 462],
    overflow: bool,
    unavailable: bool,
}
impl Default for Aggregate {
    fn default() -> Self {
        Self {
            buckets: [Bucket::default(); 462],
            overflow: false,
            unavailable: false,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub(crate) origin: WorkCommitOrigin,
    pub(crate) width: usize,
    semantic: SemanticEffect,
}
impl Context {
    pub(crate) fn prefix() -> Self {
        Self {
            origin: WorkCommitOrigin::ClaimPrefix,
            width: 0,
            semantic: SemanticEffect::Unchanged,
        }
    }
    pub(crate) fn updates(updates: &[WorkUpdate]) -> Self {
        fn origin(update: &WorkUpdate) -> WorkCommitOrigin {
            match update {
                WorkUpdate::Claim { .. } => WorkCommitOrigin::Claim,
                WorkUpdate::Materialize { .. } => WorkCommitOrigin::Materialize,
                WorkUpdate::Finish { .. } => WorkCommitOrigin::Finish,
                WorkUpdate::DeliveryStarted { .. } => WorkCommitOrigin::DeliveryStarted,
                WorkUpdate::DeliveryFinished { .. } => WorkCommitOrigin::DeliveryFinished,
            }
        }
        let first = updates.first().map_or(WorkCommitOrigin::Other, origin);
        Self {
            origin: if updates.iter().all(|update| origin(update) == first) {
                first
            } else {
                WorkCommitOrigin::Other
            },
            width: updates.len(),
            semantic: SemanticEffect::Unchanged,
        }
    }
    pub(crate) fn include(&mut self, delta: &WorkDelta) {
        if delta.before_header() != delta.header()
            || delta
                .records()
                .any(|(_, before, after)| before != Some(after))
            || delta
                .roots()
                .any(|(_, before, after)| before != Some(after))
        {
            self.semantic = SemanticEffect::Changed;
        }
    }
}

impl StageObservation {
    fn utilization(&self) -> std::sync::MutexGuard<'_, Aggregate> {
        self.inner
            .work_utilization
            .lock()
            .unwrap_or_else(|poisoned| {
                let mut aggregate = poisoned.into_inner();
                aggregate.unavailable = true;
                aggregate
            })
    }
    pub(crate) fn unavailable_work_commit(&self) {
        self.utilization().unavailable = true;
    }
    pub(crate) fn record_work_commit(&self, context: Context, failed: bool, elapsed: Duration) {
        let mut aggregate = self.utilization();
        if context.width > 32 {
            aggregate.unavailable = true;
            return;
        }
        let index = (context.origin as usize * 33 + context.width) * 2 + context.semantic as usize;
        let bucket = &mut aggregate.buckets[index];
        let values = u64::try_from(elapsed.as_nanos()).ok().and_then(|elapsed| {
            Some((
                bucket.successes.checked_add(u64::from(!failed))?,
                bucket.failures.checked_add(u64::from(failed))?,
                bucket.elapsed_ns.checked_add(elapsed)?,
            ))
        });
        if let Some((successes, failures, elapsed_ns)) = values {
            bucket.successes = successes;
            bucket.failures = failures;
            bucket.elapsed_ns = elapsed_ns;
        } else {
            bucket.dropped = bucket.dropped.saturating_add(1);
            aggregate.overflow = true;
        }
    }
    pub fn work_utilization_snapshot(&self) -> WorkUtilizationSnapshot {
        let aggregate = self.utilization();
        let origins = [
            WorkCommitOrigin::ClaimPrefix,
            WorkCommitOrigin::Claim,
            WorkCommitOrigin::Materialize,
            WorkCommitOrigin::Finish,
            WorkCommitOrigin::DeliveryStarted,
            WorkCommitOrigin::DeliveryFinished,
            WorkCommitOrigin::Other,
        ];
        let entries = aggregate
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| {
                bucket.successes != 0 || bucket.failures != 0 || bucket.dropped != 0
            })
            .map(|(index, bucket)| WorkCommitMeasurement {
                origin: origins[index / 66],
                width: ((index / 2) % 33) as u8,
                semantic: if index % 2 == 0 {
                    SemanticEffect::Unchanged
                } else {
                    SemanticEffect::Changed
                },
                native_successes: bucket.successes,
                native_failures: bucket.failures,
                elapsed_ns: bucket.elapsed_ns,
                dropped_samples: bucket.dropped,
            })
            .collect();
        WorkUtilizationSnapshot {
            entries,
            overflow: aggregate.overflow,
            unavailable: aggregate.unavailable,
        }
    }
}

#[cfg(test)]
mod tests;
