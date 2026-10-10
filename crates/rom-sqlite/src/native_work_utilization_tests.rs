//! Real WAL/FULL transactions distinguish semantic changes from acknowledgements.
use crate::native_work_test_support::{Scratch, bundle, file_fixture};
use crate::{SemanticEffect, Sqlite, StageObservation, WorkCommitOrigin};
use rom::{Error, Storage, WorkClaim, WorkResult, WorkState, WorkUpdate};
use std::sync::Arc;

fn database(rows: usize) -> (Scratch, Sqlite) {
    let scratch = Scratch::new();
    let db = file_fixture(&scratch.path("database"));
    for row in 0..rows {
        db.commit(&bundle(&format!("{row:02}"))).unwrap();
    }
    let connection = db.connection.lock().unwrap();
    let mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let synchronous: i64 = connection
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(synchronous, 2);
    drop(connection);
    (scratch, db)
}

fn materialize(claim: &WorkClaim) -> WorkUpdate {
    WorkUpdate::Materialize {
        claim: claim.key(),
        now: 0,
        children: vec![],
    }
}

fn assert_commit(
    observation: &StageObservation,
    origin: WorkCommitOrigin,
    width: u8,
    semantic: SemanticEffect,
    successes: u64,
) {
    let snapshot = observation.work_utilization_snapshot();
    assert!(!snapshot.overflow && !snapshot.unavailable);
    assert_eq!(snapshot.entries.len(), 1);
    let entry = &snapshot.entries[0];
    assert_eq!(
        (entry.origin, entry.width, entry.semantic),
        (origin, width, semantic)
    );
    assert_eq!(entry.native_successes, successes);
    assert_eq!(entry.native_failures, 0);
    assert_eq!(entry.dropped_samples, 0);
    // Elapsed time is informational; no platform-dependent timing threshold.
    assert_eq!(observation.snapshot().entries.len(), 18);
    assert_eq!(observation.publication_snapshot().entries.len(), 10);
}

#[test]
fn idle_prefix_records_zero_width_without_semantic_change() {
    let (_scratch, db) = database(0);
    let before = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    let observation = db.observe_stages();
    assert!(db.reaction_claim_prefix(0, 3).unwrap().is_empty());
    assert_eq!(db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(), before);
    assert_commit(
        &observation,
        WorkCommitOrigin::ClaimPrefix,
        0,
        SemanticEffect::Unchanged,
        1,
    );
}

#[test]
fn changed_prefix_records_returned_width_not_requested_limit() {
    let (_scratch, db) = database(3);
    let observation = db.observe_stages();
    assert_eq!(db.reaction_claim_prefix(0, 32).unwrap().len(), 3);
    assert_commit(
        &observation,
        WorkCommitOrigin::ClaimPrefix,
        3,
        SemanticEffect::Changed,
        1,
    );
}

#[test]
fn materialization_groups_three_changes_into_one_native_commit() {
    for (atomic, width, commits) in [(true, 3, 1), (false, 1, 3)] {
        let (scratch, db) = database(3);
        let claims = db.reaction_claim_prefix(0, 3).unwrap();
        assert_eq!(claims.len(), 3);
        let observation = db.observe_stages();
        if atomic {
            assert_eq!(
                db.reaction_updates_atomic(claims.iter().map(materialize).collect())
                    .unwrap(),
                vec![WorkResult::Changed; 3]
            );
        } else {
            for claim in &claims {
                assert_eq!(
                    db.reaction_update(materialize(claim)).unwrap(),
                    WorkResult::Changed
                );
            }
        }
        let ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
        assert!(
            ledger
                .records
                .iter()
                .all(|record| record.state == WorkState::Done)
        );
        drop(db);
        let reopened = Sqlite::open(scratch.path("database")).unwrap();
        assert_eq!(
            reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
            ledger
        );
        assert_commit(
            &observation,
            WorkCommitOrigin::Materialize,
            width,
            SemanticEffect::Changed,
            commits,
        );
    }
}

#[test]
fn mixed_update_kinds_use_other_origin_and_requested_batch_width() {
    let (_scratch, db) = database(1);
    let claims = db.reaction_claim_prefix(0, 2).unwrap();
    let observation = db.observe_stages();
    assert_eq!(
        db.reaction_updates_atomic(vec![materialize(&claims[0]), WorkUpdate::Claim { now: 0 }])
            .unwrap(),
        vec![WorkResult::Changed, WorkResult::Idle]
    );
    assert_commit(
        &observation,
        WorkCommitOrigin::Other,
        2,
        SemanticEffect::Changed,
        1,
    );
}

#[test]
fn precommit_rollback_has_no_native_commit_sample_and_reopens_unchanged() {
    let (scratch, db) = database(3);
    let claims = db.reaction_claim_prefix(0, 3).unwrap();
    let before = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    let observation = db.observe_stages();
    db.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == 0 {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    let result = db.reaction_updates_atomic(claims.iter().map(materialize).collect());
    db.on_commit(None);
    assert_eq!(result, Err(Error::NotCommitted));
    assert_eq!(db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(), before);
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before
    );
    assert!(observation.work_utilization_snapshot().entries.is_empty());
}

#[test]
fn acknowledgement_unknown_retains_native_success_and_committed_ledger() {
    let (scratch, db) = database(3);
    let claims = db.reaction_claim_prefix(0, 3).unwrap();
    let observation = db.observe_stages();
    db.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == usize::MAX {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    let result = db.reaction_updates_atomic(claims.iter().map(materialize).collect());
    db.on_commit(None);
    assert_eq!(result, Err(Error::Unknown));
    let committed = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    assert_eq!(committed.records.len(), 3);
    assert!(
        committed
            .records
            .iter()
            .all(|record| record.state == WorkState::Done)
    );
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        committed
    );
    assert_commit(
        &observation,
        WorkCommitOrigin::Materialize,
        3,
        SemanticEffect::Changed,
        1,
    );
}
