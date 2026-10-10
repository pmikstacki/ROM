//! Fixed cumulative batch pairs; complete errors and maxima remain in terminal counts.
use crate::{ObservedStorage, storage_calls::Call};
use serde::Serialize;
use std::time::Duration;
pub const SELECTED: [Call; 6] = [
    Call::Commit,
    Call::Claim,
    Call::Materialize,
    Call::Finish,
    Call::ReactionRecords,
    Call::RetryEpochs,
];
#[derive(Serialize)]
pub struct Batch {
    pub completed_rows: usize,
    pub elapsed_ms: u64,
    pub cumulative_call_count_and_ns: Option<[[u64; 2]; 6]>,
}
pub fn record(
    batches: &mut Vec<Batch>,
    rows: usize,
    elapsed: Duration,
    storage: &ObservedStorage,
) -> rom::Result<()> {
    if batches.len() >= 100 || rows > 10000 {
        return Err(rom::Error::TooLarge);
    }
    let pairs = storage.calls().map(|calls| {
        std::array::from_fn(|index| {
            let counts = calls[SELECTED[index] as usize].counts;
            [counts.observed_calls, counts.total_ns]
        })
    });
    batches.push(Batch {
        completed_rows: rows,
        elapsed_ms: u64::try_from(elapsed.as_millis()).map_err(|_| rom::Error::TooLarge)?,
        cumulative_call_count_and_ns: pairs,
    });
    Ok(())
}
#[cfg(test)]
#[path = "seed_measurements_tests.rs"]
mod tests;
