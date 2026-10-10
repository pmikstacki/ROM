//! Fixed core boundary counts, independent of optional lossy event tracing.
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Counts all activity in one Runtime, including streams and background work.
/// Fields are advisory during concurrent work. Complete caller/background
/// producers and drain owned work before sampling terminal totals.
/// These counts do not identify individual HTTP requests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CoreOverloadStats {
    pub action_no_permits: u64,
    pub io_no_permits: u64,
    pub observation_generation_exhausted: u64,
    pub actor_generation_exhausted: u64,
    /// At least one event could not be represented after a counter saturated.
    /// Saturated counts are lower bounds; they cannot establish exact totals.
    pub overflowed: bool,
}

#[derive(Default)]
pub(crate) struct CoreOverloadCounters {
    pub(super) action_no_permits: AtomicU64,
    pub(super) io_no_permits: AtomicU64,
    pub(super) observation_generation_exhausted: AtomicU64,
    pub(super) actor_generation_exhausted: AtomicU64,
    overflowed: AtomicBool,
}
impl CoreOverloadCounters {
    pub(crate) fn snapshot(&self) -> CoreOverloadStats {
        CoreOverloadStats {
            action_no_permits: self.action_no_permits.load(Ordering::Relaxed),
            io_no_permits: self.io_no_permits.load(Ordering::Relaxed),
            observation_generation_exhausted: self
                .observation_generation_exhausted
                .load(Ordering::Relaxed),
            actor_generation_exhausted: self.actor_generation_exhausted.load(Ordering::Relaxed),
            overflowed: self.overflowed.load(Ordering::Relaxed),
        }
    }

    fn increment(&self, counter: &AtomicU64) {
        if counter
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .is_err()
        {
            self.overflowed.store(true, Ordering::Relaxed);
        }
    }
    pub(crate) fn record_action_no_permits(&self) {
        self.increment(&self.action_no_permits);
    }
    pub(crate) fn record_io_no_permits(&self) {
        self.increment(&self.io_no_permits);
    }
    pub(crate) fn record_observation_generation_exhausted(&self) {
        self.increment(&self.observation_generation_exhausted);
    }
    pub(crate) fn record_actor_generation_exhausted(&self) {
        self.increment(&self.actor_generation_exhausted);
    }
}
