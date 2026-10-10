use super::*;
use crate::storage_calls::{Counts, Snapshot, StorageCalls};
#[test]
fn complete_worst_case_numeric_evidence_fits_64_kib() {
    let calls: Vec<_> = StorageCalls::default()
        .snapshot()
        .unwrap()
        .into_iter()
        .map(|s| Snapshot {
            call: s.call,
            counts: Counts {
                observed_calls: u64::MAX,
                failed_calls: u64::MAX,
                total_ns: u64::MAX,
                maximum_ns: u64::MAX,
                observer_overflows: u64::MAX,
                overflow_counter_exhausted: true,
            },
        })
        .collect();
    let batches: Vec<_> = (0..100)
        .map(|_| Batch {
            completed_rows: 10000,
            elapsed_ms: u64::MAX,
            cumulative_call_count_and_ns: Some([[u64::MAX; 2]; 6]),
        })
        .collect();
    let proof = serde_json::json!({"semantics":"synchronous-storage-boundary-wall-time",
        "batch_operations":SELECTED,"baseline":calls,"batches":batches,"final":calls,
        "live_encoded_state_bytes_measured":false});
    assert!(serde_json::to_vec(&proof).unwrap().len() <= 65536);
}
#[test]
fn batch_capacity_refuses_extra_record_without_changing_prior_observations() {
    let storage = ObservedStorage::new(crate::application_tests::native_storage("sqlite"));
    let mut batches = vec![];
    for rows in 1..=100 {
        record(
            &mut batches,
            rows * 100,
            Duration::from_millis(rows as u64),
            &storage,
        )
        .unwrap();
    }
    assert_eq!(
        record(&mut batches, 10000, Duration::ZERO, &storage),
        Err(rom::Error::TooLarge)
    );
    assert_eq!(batches.len(), 100);
    assert_eq!(batches[99].completed_rows, 10000);
}
