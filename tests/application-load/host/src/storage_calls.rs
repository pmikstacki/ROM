//! Fixed storage boundary observations. No payload, identity, or adapter behavior changes.
use serde::Serialize;
use std::{sync::Mutex, time::Instant};
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Call {
    AcquireOwner,
    RetryEpochs,
    Register,
    ReactionRecords,
    WorkSnapshot,
    ControlWork,
    Load,
    Snapshot,
    QueryRead,
    Receipt,
    Commit,
    JournalHead,
    Journal,
    Claim,
    Materialize,
    Finish,
    DeliveryStarted,
    DeliveryFinished,
    ClaimPrefix,
    ClaimLive,
    AtomicWorkUpdates,
}
const CALLS: [Call; 21] = [
    Call::AcquireOwner,
    Call::RetryEpochs,
    Call::Register,
    Call::ReactionRecords,
    Call::WorkSnapshot,
    Call::ControlWork,
    Call::Load,
    Call::Snapshot,
    Call::QueryRead,
    Call::Receipt,
    Call::Commit,
    Call::JournalHead,
    Call::Journal,
    Call::Claim,
    Call::Materialize,
    Call::Finish,
    Call::DeliveryStarted,
    Call::DeliveryFinished,
    Call::ClaimPrefix,
    Call::ClaimLive,
    Call::AtomicWorkUpdates,
];
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Counts {
    pub observed_calls: u64,
    pub failed_calls: u64,
    pub total_ns: u64,
    pub maximum_ns: u64,
    pub observer_overflows: u64,
    pub overflow_counter_exhausted: bool,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub call: Call,
    pub counts: Counts,
}
pub struct StorageCalls {
    counts: Mutex<[Counts; 21]>,
}
impl Default for StorageCalls {
    fn default() -> Self {
        Self {
            counts: Mutex::new([Counts::default(); 21]),
        }
    }
}
impl StorageCalls {
    pub fn measure<T>(
        &self,
        call: Call,
        action: impl FnOnce() -> rom::Result<T>,
    ) -> rom::Result<T> {
        let started = Instant::now();
        let result = action();
        self.record(
            call,
            u64::try_from(started.elapsed().as_nanos()).ok(),
            result.is_err(),
        );
        result
    }
    fn record(&self, call: Call, elapsed: Option<u64>, failed: bool) {
        let Ok(mut counts) = self.counts.lock() else {
            return;
        };
        let value = &mut counts[call as usize];
        let next = value
            .observed_calls
            .checked_add(1)
            .zip(value.failed_calls.checked_add(u64::from(failed)))
            .zip(elapsed.and_then(|ns| value.total_ns.checked_add(ns)));
        if let Some(((calls, failures), total)) = next {
            value.observed_calls = calls;
            value.failed_calls = failures;
            value.total_ns = total;
            value.maximum_ns = value.maximum_ns.max(elapsed.unwrap_or(0));
        } else if let Some(next) = value.observer_overflows.checked_add(1) {
            value.observer_overflows = next;
        } else {
            value.overflow_counter_exhausted = true;
        }
    }
    pub fn snapshot(&self) -> Option<Vec<Snapshot>> {
        self.counts.lock().ok().map(|counts| {
            CALLS
                .iter()
                .map(|call| Snapshot {
                    call: *call,
                    counts: counts[*call as usize],
                })
                .collect()
        })
    }
}
#[cfg(test)]
#[path = "storage_calls_tests.rs"]
mod tests;
