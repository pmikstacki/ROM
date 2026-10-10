//! Canonical archive state remains separate from bounded native metadata.
use super::metadata::StorageMetadata;
use super::*;

fn populated() -> StorageState {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![PendingWork {
                id: "retained-work".into(),
                cause: Cause {
                    retry_epoch: 0,
                    root: "root".into(),
                    parent: None,
                    depth: 0,
                    started_at: 10,
                    path: vec![],
                },
                definition: "test".into(),
                version: 1,
                service_key: "service".into(),
                delivery_profile: DeliveryProfile::AtLeastOnce,
                not_before: None,
                payload: WorkPayload::Source(Row {
                    key: Key {
                        kind: "test".into(),
                        id: "target".into(),
                    },
                    revision: 1,
                    value: Some(json!({"title":"retained"})),
                    protected: ProtectedMetadata::default(),
                }),
            }],
        )
        .unwrap();
    state
}

#[test]
fn split_and_reassembly_preserve_canonical_archive_state() {
    let state = populated();
    let expected = serde_json::to_value(&state).unwrap();
    let (metadata, work, operator) = StorageMetadata::split(state);
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let restored: StorageMetadata = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(restored.reassemble(work, operator)).unwrap(),
        expected
    );
}

#[test]
fn native_metadata_excludes_retained_work_and_operator_collections() {
    let state = populated();
    let expected_epochs = state.retry_epochs();
    let expected_limits = state.storage_limits();
    let expected_cursor = state.journal_head("test");
    let (metadata, work, _) = StorageMetadata::split(state);
    assert_eq!(work.records().len(), 1);
    let wire = serde_json::to_value(&metadata).unwrap();
    assert!(wire.get("work").is_none());
    assert!(wire.get("operator").is_none());
    assert_eq!(metadata.retry_epochs(), expected_epochs);
    assert_eq!(metadata.storage_limits(), expected_limits);
    assert_eq!(metadata.journal_head("test"), expected_cursor);
}

#[test]
fn bounded_metadata_journal_uses_canonical_cursor_and_limit_rules() {
    let mut state = StorageState::new(StorageLimits {
        journal_rows: 1,
        ..StorageLimits::default()
    })
    .unwrap();
    let initial = state.journal_head("test");
    for id in ["first", "second"] {
        state
            .bundle(&Bundle {
                expected: None,
                receipt: Receipt {
                    retry_epoch: 0,
                    replay_version: None,
                    identity: id.into(),
                    fingerprint: id.into(),
                    row: Row {
                        key: Key {
                            kind: "test".into(),
                            id: id.into(),
                        },
                        revision: 1,
                        value: Some(json!({"title": id})),
                        protected: ProtectedMetadata::default(),
                    },
                },
                changed: true,
                effects: vec![],
                reactions: vec![],
                reaction_limits: None,
                completed_work: None,
            })
            .unwrap();
    }
    let metadata = StorageMetadata::from_state(&state);
    assert_eq!(
        metadata.journal("test", Some(&initial), 10, 1024),
        Err(Error::HistoryGap)
    );
    let cursor = JournalCursor {
        position: state.journal_floor(),
        ..state.journal_head("test")
    };
    for (rows, bytes) in [(10, 1024), (10, 1), (0, 1024)] {
        assert_eq!(
            metadata.journal("test", Some(&cursor), rows, bytes),
            state.journal("test", Some(&cursor), rows, bytes)
        );
    }
}

#[test]
fn incremental_bundle_matches_canonical_completion_and_child_enqueue() {
    use super::metadata::prepare_bundle;
    use crate::reaction_work::incremental_bridge::WorkImage;
    let mut state = populated();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim, got {other:?}"),
    };
    let mut child = claim.work.pending.clone();
    child.id = "child".into();
    child.cause.parent = Some(claim.work.pending.id.clone());
    child.cause.depth = 1;
    let bundle = Bundle {
        expected: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: "new".into(),
            fingerprint: "new".into(),
            row: Row {
                key: Key {
                    kind: "test".into(),
                    id: "new".into(),
                },
                revision: 1,
                value: Some(json!({"title":"new"})),
                protected: ProtectedMetadata::default(),
            },
        },
        changed: true,
        effects: vec![],
        reactions: vec![child],
        reaction_limits: Some(ReactionLimits::default()),
        completed_work: Some((claim.key(), 11)),
    };
    let metadata = StorageMetadata::from_state(&state);
    let mut image = WorkImage::from_ledger(state.work.clone(), state.retry_epochs()).unwrap();
    let delta = prepare_bundle(&metadata, &image, &bundle).unwrap();
    let mut expected = state.clone();
    let expected_retired = expected.bundle(&bundle).unwrap();
    let (after, work_delta, retired) = delta.into_parts();
    assert_eq!(retired, expected_retired);
    image.apply(work_delta).unwrap();
    let actual = after.reassemble(image.canonical(), state.operator);
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn rejected_bundle_does_not_publish_completion_or_counters() {
    use super::metadata::prepare_bundle;
    use crate::reaction_work::incremental_bridge::WorkImage;
    let mut state = populated();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim, got {other:?}"),
    };
    state.receipts = 1;
    state.limits.receipts = 1;
    let metadata = StorageMetadata::from_state(&state);
    let image = WorkImage::from_ledger(state.work.clone(), state.retry_epochs()).unwrap();
    let before_work = serde_json::to_value(image.canonical()).unwrap();
    let before_metadata = serde_json::to_value(&metadata).unwrap();
    let bundle = Bundle {
        expected: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: "new".into(),
            fingerprint: "new".into(),
            row: Row {
                key: Key {
                    kind: "test".into(),
                    id: "new".into(),
                },
                revision: 1,
                value: Some(json!({"title":"new"})),
                protected: ProtectedMetadata::default(),
            },
        },
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: Some((claim.key(), 11)),
    };
    assert!(matches!(
        prepare_bundle(&metadata, &image, &bundle),
        Err(Error::Overloaded)
    ));
    assert_eq!(state.bundle(&bundle), Err(Error::Overloaded));
    assert_eq!(
        serde_json::to_value(image.canonical()).unwrap(),
        before_work
    );
    assert_eq!(serde_json::to_value(&metadata).unwrap(), before_metadata);
}

#[test]
fn pre_revision_receipt_replay_bypasses_stale_claim() {
    use crate::storage_support::work::WorkImage;
    let mut state = populated();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim, got {other:?}"),
    };
    let mut stale = claim.key();
    stale.generation += 1;
    let metadata = StorageMetadata::from_state(&state);
    let image = WorkImage::from_ledger(state.work.clone(), state.retry_epochs()).unwrap();
    assert_eq!(
        metadata.check_retry_epoch(0, true, Some((&stale, 11)), &image),
        Ok(())
    );
    assert_eq!(
        metadata.check_retry_epoch(0, false, Some((&stale, 11)), &image),
        Err(Error::Conflict)
    );
    assert_eq!(
        metadata.check_retry_epoch(1, true, Some((&stale, 11)), &image),
        state.check_retry_epoch(1, true, Some((&stale, 11)))
    );
}

#[test]
fn causal_admission_below_floor_matches_canonical_pre_revision_checks() {
    use crate::storage_support::work::WorkImage;
    let mut state = populated();
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["retained-work"]["pending"]["cause"]["retry_epoch"] = json!(1);
    state.work = serde_json::from_value(wire).unwrap();
    state.retry_epochs = RetryEpochs {
        current: 2,
        admission_floor: 2,
        replay_floor: 1,
    };
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim, got {other:?}"),
    };
    let metadata = StorageMetadata::from_state(&state);
    let image = WorkImage::from_ledger(state.work.clone(), state.retry_epochs()).unwrap();
    for epoch in [0, 1, 2, 3] {
        for replay in [false, true] {
            for completed in [None, Some((&claim.key(), 11))] {
                assert_eq!(
                    metadata.check_retry_epoch(epoch, replay, completed, &image),
                    state.check_retry_epoch(epoch, replay, completed)
                );
            }
        }
    }
    assert_eq!(
        metadata.check_retry_epoch(1, false, None, &image),
        Err(Error::IdentityExpired)
    );
    assert_eq!(
        metadata.check_retry_epoch(1, false, Some((&claim.key(), 11)), &image),
        Ok(())
    );
}
