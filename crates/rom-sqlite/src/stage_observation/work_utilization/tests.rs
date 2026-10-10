//! Aggregates reject unavailable samples and never wrap numeric counters.
use super::*;
use crate::stage_observation::Timer;

#[test]
fn every_fixed_bucket_remains_separate_and_snapshots_omit_zeros() {
    let handle = StageObservation::default();
    assert!(handle.work_utilization_snapshot().entries.is_empty());
    for origin in [
        WorkCommitOrigin::ClaimPrefix,
        WorkCommitOrigin::Claim,
        WorkCommitOrigin::Materialize,
        WorkCommitOrigin::Finish,
        WorkCommitOrigin::DeliveryStarted,
        WorkCommitOrigin::DeliveryFinished,
        WorkCommitOrigin::Other,
    ] {
        for width in 0..=32 {
            for semantic in [SemanticEffect::Unchanged, SemanticEffect::Changed] {
                handle.record_work_commit(
                    Context {
                        origin,
                        width,
                        semantic,
                    },
                    false,
                    Duration::from_nanos(1),
                );
            }
        }
    }
    let snapshot = handle.work_utilization_snapshot();
    assert_eq!(snapshot.entries.len(), 462);
    assert!(!snapshot.overflow && !snapshot.unavailable);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| entry.native_successes == 1
                && entry.native_failures == 0
                && entry.elapsed_ns == 1)
    );
}

#[test]
fn failed_native_commit_is_distinct_and_overflow_drops_whole_sample() {
    let handle = StageObservation::default();
    let context = Context::prefix();
    handle.record_work_commit(context, true, Duration::from_nanos(7));
    handle.record_work_commit(context, false, Duration::from_secs(u64::MAX));
    let snapshot = handle.work_utilization_snapshot();
    assert_eq!(snapshot.entries.len(), 1);
    let entry = snapshot.entries[0];
    assert_eq!(
        (
            entry.native_successes,
            entry.native_failures,
            entry.elapsed_ns,
            entry.dropped_samples
        ),
        (0, 1, 7, 1)
    );
    assert!(snapshot.overflow && !snapshot.unavailable);
    {
        let mut aggregate = handle.utilization();
        aggregate.buckets[0].successes = u64::MAX;
        aggregate.buckets[0].dropped = u64::MAX;
    }
    handle.record_work_commit(context, false, Duration::from_nanos(1));
    let entry = handle.work_utilization_snapshot().entries[0];
    assert_eq!(entry.native_successes, u64::MAX);
    assert_eq!(entry.elapsed_ns, 7);
    assert_eq!(entry.dropped_samples, u64::MAX);
}

#[test]
fn invalid_width_poison_and_unfinished_timer_are_unavailable_not_success() {
    let handle = StageObservation::default();
    let mut invalid = Context::prefix();
    invalid.width = 33;
    handle.record_work_commit(invalid, false, Duration::ZERO);
    assert!(handle.work_utilization_snapshot().unavailable);
    assert!(handle.work_utilization_snapshot().entries.is_empty());
    let poisoned = handle.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = poisoned.inner.work_utilization.lock().unwrap();
            panic!("controlled utilization mutex poisoning");
        })
        .join()
        .is_err()
    );
    handle.record_work_commit(Context::prefix(), false, Duration::from_nanos(1));
    assert!(handle.work_utilization_snapshot().unavailable);
    let unfinished = StageObservation::default();
    drop(Timer::work_commit(
        Some(&unfinished),
        Some(Context::prefix()),
    ));
    assert!(unfinished.work_utilization_snapshot().unavailable);
    assert!(unfinished.work_utilization_snapshot().entries.is_empty());
    let disabled = Timer::work_commit(None, None);
    assert!(disabled.started_for_test_is_none());
    drop(disabled);
}
