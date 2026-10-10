//! Singleton stage compatibility and explicit predecessor capability.
#[cfg(feature = "test-support")]
use crate::native_work_test_support::fixture;
use crate::native_work_test_support::{bundle, predecessor_fixture};
use rom::{Error, Storage, WorkResult, WorkUpdate};

#[test]
fn atomic_predecessor_rejects_groups_without_claiming_and_keeps_singleton() {
    let db = predecessor_fixture();
    db.commit(&bundle("a")).unwrap();
    let before = db.reaction_records().unwrap();
    assert!(matches!(
        db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 2]),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(db.reaction_records().unwrap(), before);
    assert!(matches!(
        db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }])
            .unwrap()
            .as_slice(),
        [WorkResult::Claimed(_)]
    ));
}

#[cfg(feature = "test-support")]
#[test]
fn atomic_singleton_preserves_exact_existing_stage_sample_counts() {
    use crate::{StageOperation, StorageStage};
    let db = fixture();
    db.commit(&bundle("a")).unwrap();
    let observed = db.observe_stages();
    db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }])
        .unwrap();
    let snapshot = observed.snapshot();
    for stage in [
        StorageStage::ConnectionLock,
        StorageStage::TransactionBegin,
        StorageStage::SharedPrepare,
        StorageStage::NativePublication,
        StorageStage::NativeCommit,
    ] {
        assert_eq!(
            snapshot
                .entries
                .iter()
                .find(|entry| entry.operation == StageOperation::WorkUpdate && entry.stage == stage)
                .unwrap()
                .samples,
            1
        );
    }
    assert_eq!(
        snapshot
            .entries
            .iter()
            .find(|entry| entry.operation == StageOperation::WorkUpdate
                && entry.stage == StorageStage::MetadataRead)
            .unwrap()
            .samples,
        0
    );
}
