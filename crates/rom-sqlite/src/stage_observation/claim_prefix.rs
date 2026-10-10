//! Grouped-prefix termination observations, separate from acknowledgement.
/// Why a native grouped claim stopped examining candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimPrefixReason {
    RequestedLimit,
    Idle,
    MaintenanceChanged,
    ResolutionBarrier,
    NotificationBarrier,
    ActionBarrier,
    DuplicateRoot,
}
/// A barrier is either admitted alone or withheld before publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimPrefixDisposition {
    Unrestricted,
    AdmittedSingleton,
    Withheld,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimPrefixMeasurement {
    pub reason: ClaimPrefixReason,
    /// Staged claimed records; excludes a withheld candidate.
    pub width: u8,
    pub disposition: ClaimPrefixDisposition,
    /// Native transaction outcomes, not caller acknowledgements or fsync counts.
    pub native_successes: u64,
    pub native_failures: u64,
    pub dropped_samples: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClaimPrefixSnapshot {
    /// Nonzero fixed buckets, bounded by seven reasons × 33 widths × three dispositions.
    pub entries: Vec<ClaimPrefixMeasurement>,
    pub overflow: bool,
    pub unavailable: bool,
}
impl super::StageObservation {
    /// Grouped native path only: requested max > 1 and current journal layout.
    /// Singleton compatibility operations remain in existing Work observations.
    pub fn claim_prefix_snapshot(&self) -> ClaimPrefixSnapshot {
        let aggregate = self.prefix_aggregate();
        let entries = aggregate
            .buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| {
                bucket.successes != 0 || bucket.failures != 0 || bucket.dropped != 0
            })
            .map(|(index, bucket)| ClaimPrefixMeasurement {
                reason: REASONS[index / 99],
                width: ((index / 3) % 33) as u8,
                disposition: DISPOSITIONS[index % 3],
                native_successes: bucket.successes,
                native_failures: bucket.failures,
                dropped_samples: bucket.dropped,
            })
            .collect();
        ClaimPrefixSnapshot {
            entries,
            overflow: aggregate.overflow,
            unavailable: aggregate.unavailable,
        }
    }
}

const REASONS: [ClaimPrefixReason; 7] = [
    ClaimPrefixReason::RequestedLimit,
    ClaimPrefixReason::Idle,
    ClaimPrefixReason::MaintenanceChanged,
    ClaimPrefixReason::ResolutionBarrier,
    ClaimPrefixReason::NotificationBarrier,
    ClaimPrefixReason::ActionBarrier,
    ClaimPrefixReason::DuplicateRoot,
];
const DISPOSITIONS: [ClaimPrefixDisposition; 3] = [
    ClaimPrefixDisposition::Unrestricted,
    ClaimPrefixDisposition::AdmittedSingleton,
    ClaimPrefixDisposition::Withheld,
];
#[derive(Clone, Copy, Default)]
struct Bucket {
    successes: u64,
    failures: u64,
    dropped: u64,
}
pub(super) struct Aggregate {
    buckets: [Bucket; 693],
    overflow: bool,
    unavailable: bool,
}
impl Default for Aggregate {
    fn default() -> Self {
        Self {
            buckets: [Bucket::default(); 693],
            overflow: false,
            unavailable: false,
        }
    }
}
fn index(reason: ClaimPrefixReason, width: usize, disposition: ClaimPrefixDisposition) -> usize {
    (reason as usize * 33 + width) * 3 + disposition as usize
}
impl super::StageObservation {
    fn prefix_aggregate(&self) -> std::sync::MutexGuard<'_, Aggregate> {
        self.inner.claim_prefix.lock().unwrap_or_else(|poisoned| {
            let mut aggregate = poisoned.into_inner();
            aggregate.unavailable = true;
            aggregate
        })
    }
    fn record_prefix(
        &self,
        reason: ClaimPrefixReason,
        width: usize,
        disposition: ClaimPrefixDisposition,
        failed: bool,
    ) {
        let mut aggregate = self.prefix_aggregate();
        if width > 32 {
            aggregate.unavailable = true;
            return;
        }
        let bucket = &mut aggregate.buckets[index(reason, width, disposition)];
        let values = bucket
            .successes
            .checked_add(u64::from(!failed))
            .zip(bucket.failures.checked_add(u64::from(failed)));
        if let Some((successes, failures)) = values {
            bucket.successes = successes;
            bucket.failures = failures;
        } else {
            bucket.dropped = bucket.dropped.saturating_add(1);
            aggregate.overflow = true;
        }
    }
}
/// No clocks, allocations or aggregate locks until an enabled observation ends.
pub(crate) struct Guard<'a> {
    observation: Option<&'a super::StageObservation>,
    reason: ClaimPrefixReason,
    disposition: ClaimPrefixDisposition,
}
impl<'a> Guard<'a> {
    pub(crate) fn new(observation: Option<&'a super::StageObservation>) -> Self {
        Self {
            observation,
            reason: ClaimPrefixReason::RequestedLimit,
            disposition: ClaimPrefixDisposition::Unrestricted,
        }
    }
    pub(crate) fn enabled(&self) -> bool {
        self.observation.is_some()
    }
    pub(crate) fn termination(
        &mut self,
        reason: ClaimPrefixReason,
        disposition: ClaimPrefixDisposition,
    ) {
        self.reason = reason;
        self.disposition = disposition;
    }
    pub(crate) fn finish(mut self, width: usize, failed: bool) {
        if let Some(observation) = self.observation.take() {
            observation.record_prefix(self.reason, width, self.disposition, failed);
        }
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(observation) = self.observation.take() {
            observation.prefix_aggregate().unavailable = true;
        }
    }
}

#[cfg(test)]
mod tests;
