use super::*;
use crate::StageObservation;

#[test]
fn every_fixed_reason_width_disposition_bucket_is_independent() {
    let handle = StageObservation::default();
    assert!(handle.claim_prefix_snapshot().entries.is_empty());
    for reason in REASONS {
        for width in 0..=32 {
            for disposition in DISPOSITIONS {
                handle.record_prefix(reason, width, disposition, false);
                handle.record_prefix(reason, width, disposition, true);
            }
        }
    }
    let snapshot = handle.claim_prefix_snapshot();
    assert_eq!(snapshot.entries.len(), 693);
    assert!(!snapshot.unavailable && !snapshot.overflow);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| entry.native_successes == 1
                && entry.native_failures == 1
                && entry.dropped_samples == 0)
    );
}

#[test]
fn overflow_drops_whole_sample_without_wrapping_or_partial_update() {
    let handle = StageObservation::default();
    let bucket_index = index(
        ClaimPrefixReason::Idle,
        0,
        ClaimPrefixDisposition::Unrestricted,
    );
    {
        let mut aggregate = handle.prefix_aggregate();
        aggregate.buckets[bucket_index] = Bucket {
            successes: u64::MAX,
            failures: 7,
            dropped: u64::MAX,
        };
    }
    handle.record_prefix(
        ClaimPrefixReason::Idle,
        0,
        ClaimPrefixDisposition::Unrestricted,
        false,
    );
    let snapshot = handle.claim_prefix_snapshot();
    assert!(snapshot.overflow && !snapshot.unavailable);
    let entry = snapshot.entries[0];
    assert_eq!(
        (
            entry.native_successes,
            entry.native_failures,
            entry.dropped_samples
        ),
        (u64::MAX, 7, u64::MAX)
    );
}

#[test]
fn invalid_width_and_unfinished_enabled_guard_are_unavailable_without_success() {
    let handle = StageObservation::default();
    handle.record_prefix(
        ClaimPrefixReason::Idle,
        33,
        ClaimPrefixDisposition::Unrestricted,
        false,
    );
    assert!(handle.claim_prefix_snapshot().unavailable);
    assert!(handle.claim_prefix_snapshot().entries.is_empty());
    let unfinished = StageObservation::default();
    drop(Guard::new(Some(&unfinished)));
    assert!(unfinished.claim_prefix_snapshot().unavailable);
    assert!(unfinished.claim_prefix_snapshot().entries.is_empty());
}

#[test]
fn poisoned_aggregate_marks_unavailable_and_disabled_guard_never_accesses_it() {
    let handle = StageObservation::default();
    let poisoned = handle.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = poisoned.inner.claim_prefix.lock().unwrap();
            panic!("controlled prefix aggregate poisoning");
        })
        .join()
        .is_err()
    );
    let disabled = Guard::new(None);
    assert!(!disabled.enabled());
    disabled.finish(16, false);
    drop(Guard::new(None));
    // Inspect poison directly before any recovering snapshot or enabled guard.
    // An accidental disabled aggregate access would already set unavailable.
    {
        let aggregate = handle.inner.claim_prefix.lock().err().unwrap().into_inner();
        assert!(!aggregate.unavailable);
        assert!(
            aggregate
                .buckets
                .iter()
                .all(|b| b.successes == 0 && b.failures == 0)
        );
    }
    let guard = Guard::new(Some(&handle));
    guard.finish(16, false);
    let snapshot = handle.claim_prefix_snapshot();
    assert!(snapshot.unavailable);
    assert_eq!(snapshot.entries[0].native_successes, 1);
}
