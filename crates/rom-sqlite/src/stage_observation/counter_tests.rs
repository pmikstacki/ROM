//! Fixed-size overflow and poisoned-observer recovery do not affect storage decisions.
use super::*;
use std::time::Duration;

#[test]
fn overflow_drops_complete_samples_without_wrapping_or_growing_labels() {
    let handle = StageObservation::default();
    handle.record(
        StageOperation::Commit,
        StorageStage::NativeCommit,
        Duration::from_secs(u64::MAX),
    );
    let mut snapshot = handle.snapshot();
    assert_eq!(snapshot.entries.len(), 18);
    let index = StageOperation::Commit as usize * 6 + StorageStage::NativeCommit as usize;
    assert_eq!(snapshot.entries[index].samples, 0);
    assert_eq!(snapshot.entries[index].elapsed_ns, 0);
    assert_eq!(snapshot.entries[index].dropped_samples, 1);
    assert!(snapshot.entries[index].saturated);
    {
        let mut entries = handle.inner.entries.lock().unwrap();
        entries[index].samples = u64::MAX;
    }
    handle.record(
        StageOperation::Commit,
        StorageStage::NativeCommit,
        Duration::from_nanos(1),
    );
    snapshot = handle.snapshot();
    assert_eq!(snapshot.entries[index].samples, u64::MAX);
    assert_eq!(snapshot.entries[index].elapsed_ns, 0);
    assert_eq!(snapshot.entries[index].dropped_samples, 2);
    assert!(
        snapshot
            .entries
            .iter()
            .enumerate()
            .all(|(at, entry)| at == index
                || (entry.samples == 0
                    && entry.elapsed_ns == 0
                    && entry.dropped_samples == 0
                    && !entry.saturated))
    );
}

#[test]
fn poisoned_observation_recovers_and_does_not_reject_a_real_commit() {
    use rom::Storage;
    let db = crate::native_work_test_support::fixture();
    let handle = db.observe_stages();
    let worker = handle.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = worker.inner.entries.lock().unwrap();
            panic!("controlled observation mutex poisoning");
        })
        .join()
        .is_err()
    );
    db.commit(&crate::native_work_test_support::bundle("after-poison"))
        .unwrap();
    let snapshot = handle.snapshot();
    assert!(snapshot.poison_recovered);
    assert_eq!(
        snapshot.entries[StorageStage::NativeCommit as usize].samples,
        1
    );
    assert_eq!(db.counts().unwrap(), [1, 1, 1, 0]);
}

#[test]
fn disabled_timer_has_no_clock_sample_or_aggregate() {
    let timer = Timer::new(None, StageOperation::Commit, StorageStage::NativeCommit);
    assert!(timer.started_for_test_is_none());
    drop(timer);
}
