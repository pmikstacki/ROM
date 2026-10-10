//! Numeric overflow and poisoned publication observers cannot affect native decisions.
use super::*;
use rom::Storage;

#[test]
fn publication_overflow_drops_whole_samples_and_keeps_fixed_capacity() {
    let handle = StageObservation::default();
    {
        let mut entries = handle.inner.publications.lock().unwrap();
        entries[0].encoded_bytes = u64::MAX;
    }
    handle.record_publication(StageOperation::Commit, PublicationCategory::Metadata, 1);
    let snapshot = handle.publication_snapshot();
    assert_eq!(snapshot.entries.len(), 10);
    assert_eq!(snapshot.entries[0].samples, 0);
    assert_eq!(snapshot.entries[0].encoded_bytes, u64::MAX);
    assert_eq!(snapshot.entries[0].dropped_samples, 1);
    assert!(snapshot.entries[0].saturated);
    {
        let mut entries = handle.inner.publications.lock().unwrap();
        entries[0].samples = u64::MAX;
        entries[0].encoded_bytes = 0;
    }
    handle.record_publication(StageOperation::Commit, PublicationCategory::Metadata, 1);
    let snapshot = handle.publication_snapshot();
    assert_eq!(snapshot.entries[0].samples, u64::MAX);
    assert_eq!(snapshot.entries[0].encoded_bytes, 0);
    assert_eq!(snapshot.entries[0].dropped_samples, 2);
    handle.record_publication(
        StageOperation::RetryEpochs,
        PublicationCategory::Metadata,
        1,
    );
    assert_eq!(snapshot, handle.publication_snapshot());
    assert!(
        snapshot.entries[1..]
            .iter()
            .all(|e| e.samples == 0 && e.encoded_bytes == 0)
    );
}
#[test]
fn poisoned_publication_recorder_recovers_during_real_sql_publication() {
    let db = crate::native_work_test_support::fixture();
    let handle = db.observe_stages();
    let worker = handle.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = worker.inner.publications.lock().unwrap();
            panic!("controlled publication recorder poison");
        })
        .join()
        .is_err()
    );
    db.commit(&crate::native_work_test_support::bundle("after-poison"))
        .unwrap();
    let snapshot = handle.publication_snapshot();
    assert!(snapshot.poison_recovered);
    assert_eq!(snapshot.entries[0].samples, 1);
    assert!(snapshot.entries[0].encoded_bytes > 0);
    assert_eq!(db.counts().unwrap(), [1, 1, 1, 0]);
}
