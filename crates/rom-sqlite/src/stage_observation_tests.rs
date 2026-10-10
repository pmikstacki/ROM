//! Actual native storage equivalence and bounded observation tests.
use crate::native_work_test_support::{bundle, claim, fixture};
use crate::{StageOperation, StorageStage};
use rom::*;

fn samples(snapshot: &crate::StageSnapshot, operation: StageOperation, stage: StorageStage) -> u64 {
    snapshot
        .entries
        .iter()
        .find(|entry| entry.operation == operation && entry.stage == stage)
        .unwrap()
        .samples
}
fn script(db: &crate::Sqlite) -> (Vec<Receipt>, Vec<WorkRecord>, [u64; 4]) {
    let first = db.commit(&bundle("first")).unwrap();
    let claimed = claim(db);
    db.reaction_update(WorkUpdate::Materialize {
        claim: claimed.key(),
        now: 0,
        children: vec![],
    })
    .unwrap();
    db.inject_fault(1);
    assert_eq!(db.commit(&bundle("rollback")), Err(Error::NotCommitted));
    let lost = bundle("lost");
    db.inject_fault(5);
    assert_eq!(db.commit(&lost), Err(Error::Unknown));
    let replay = db.commit(&lost).unwrap();
    db.retry_epochs().unwrap();
    (
        vec![first, replay],
        db.reaction_records().unwrap(),
        db.counts().unwrap(),
    )
}
#[test]
fn stages_count_real_success_rollback_replay_and_lost_ack_without_changing_results() {
    let disabled = fixture();
    assert!(disabled.stage_observation.get().is_none());
    let observed = fixture();
    let handle = observed.observe_stages();
    let duplicate_handle = observed.observe_stages();
    assert_eq!(script(&disabled), script(&observed));
    assert!(disabled.stage_observation.get().is_none());
    let snapshot = handle.snapshot();
    assert_eq!(snapshot, duplicate_handle.snapshot());
    assert_eq!(snapshot.entries.len(), 18);
    assert!(!snapshot.poison_recovered);
    for stage in [
        StorageStage::ConnectionLock,
        StorageStage::TransactionBegin,
        StorageStage::MetadataRead,
        StorageStage::SharedPrepare,
    ] {
        assert_eq!(samples(&snapshot, StageOperation::Commit, stage), 4);
    }
    assert_eq!(
        samples(
            &snapshot,
            StageOperation::Commit,
            StorageStage::NativePublication
        ),
        3
    );
    assert_eq!(
        samples(
            &snapshot,
            StageOperation::Commit,
            StorageStage::NativeCommit
        ),
        2
    );
    assert_eq!(
        samples(
            &snapshot,
            StageOperation::WorkUpdate,
            StorageStage::SharedPrepare
        ),
        2
    );
    assert_eq!(
        samples(
            &snapshot,
            StageOperation::WorkUpdate,
            StorageStage::NativeCommit
        ),
        2
    );
    assert_eq!(
        samples(
            &snapshot,
            StageOperation::RetryEpochs,
            StorageStage::MetadataRead
        ),
        1
    );
    for entry in &snapshot.entries {
        assert!(!entry.saturated);
        assert_eq!(entry.dropped_samples, 0);
        if entry.samples == 0 {
            assert_eq!(entry.elapsed_ns, 0);
        }
    }
    assert!(snapshot.entries.iter().any(|entry| entry.elapsed_ns > 0));
}

#[test]
fn rejected_shared_work_preparation_is_counted_without_publication_or_commit() {
    let db = fixture();
    let handle = db.observe_stages();
    assert_eq!(
        db.reaction_update(WorkUpdate::Materialize {
            claim: ClaimKey {
                id: "missing".into(),
                generation: 1
            },
            now: 0,
            children: vec![]
        }),
        Ok(WorkResult::Idle)
    );
    db.commit(&bundle("one")).unwrap();
    let before = handle.snapshot();
    assert_eq!(
        db.reaction_update(WorkUpdate::Materialize {
            claim: ClaimKey {
                id: "missing".into(),
                generation: 1
            },
            now: 0,
            children: vec![]
        }),
        Err(Error::Missing)
    );
    let after = handle.snapshot();
    assert_eq!(
        samples(
            &after,
            StageOperation::WorkUpdate,
            StorageStage::SharedPrepare
        ),
        samples(
            &before,
            StageOperation::WorkUpdate,
            StorageStage::SharedPrepare
        ) + 1
    );
    assert_eq!(
        samples(
            &after,
            StageOperation::WorkUpdate,
            StorageStage::NativePublication
        ),
        samples(
            &before,
            StageOperation::WorkUpdate,
            StorageStage::NativePublication
        )
    );
    assert_eq!(
        samples(
            &after,
            StageOperation::WorkUpdate,
            StorageStage::NativeCommit
        ),
        samples(
            &before,
            StageOperation::WorkUpdate,
            StorageStage::NativeCommit
        )
    );
}
