//! Real file databases; initial RED covers absent grouped termination samples.
use crate::native_work_test_support::{Scratch, bundle, file_fixture};
use crate::{ClaimPrefixDisposition, ClaimPrefixReason, ClaimPrefixSnapshot, Sqlite};
use rom::{Storage, WorkState};

fn database(rows: usize) -> (Scratch, Sqlite) {
    let scratch = Scratch::new();
    let db = file_fixture(&scratch.path("database"));
    for row in 0..rows {
        db.commit(&bundle(&format!("{row:02}"))).unwrap();
    }
    (scratch, db)
}

fn assert_grouped_sample(snapshot: ClaimPrefixSnapshot, reason: ClaimPrefixReason, width: u8) {
    assert_barrier(
        snapshot,
        reason,
        width,
        ClaimPrefixDisposition::Unrestricted,
    );
}

#[test]
fn grouped_empty_database_records_idle_zero_width() {
    let (scratch, db) = database(0);
    let observation = db.observe_stages();
    let claims = db.reaction_claim_prefix(0, 16).unwrap();
    let records = db.reaction_records().unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    let restored = reopened.reaction_records().unwrap();
    drop(reopened);
    assert!(claims.is_empty() && records.is_empty());
    assert_eq!(restored, records);
    assert_grouped_sample(
        observation.claim_prefix_snapshot(),
        ClaimPrefixReason::Idle,
        0,
    );
}

#[test]
fn grouped_exact_requested_capacity_records_limit_without_peeking() {
    let (scratch, db) = database(3);
    let before = db.reaction_records().unwrap();
    let observation = db.observe_stages();
    let claims = db.reaction_claim_prefix(0, 3).unwrap();
    let records = db.reaction_records().unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    let restored = reopened.reaction_records().unwrap();
    drop(reopened);
    assert_eq!(claims.len(), 3);
    assert_eq!(restored, records);
    for (previous, current) in before.iter().zip(&records) {
        assert_eq!(previous.pending, current.pending);
        assert_eq!((current.attempts, current.generation), (1, 1));
        assert!(matches!(current.state, WorkState::Leased { .. }));
    }
    assert_grouped_sample(
        observation.claim_prefix_snapshot(),
        ClaimPrefixReason::RequestedLimit,
        3,
    );
}

#[test]
fn grouped_eligible_exhaustion_records_idle_after_three_claims() {
    let (scratch, db) = database(3);
    let observation = db.observe_stages();
    let claims = db.reaction_claim_prefix(0, 16).unwrap();
    let records = db.reaction_records().unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    let restored = reopened.reaction_records().unwrap();
    drop(reopened);
    assert_eq!(claims.len(), 3);
    assert_eq!(restored, records);
    assert!(records.iter().all(|r| r.attempts == 1 && r.generation == 1));
    assert_grouped_sample(
        observation.claim_prefix_snapshot(),
        ClaimPrefixReason::Idle,
        3,
    );
}

fn database_bundles(bundles: Vec<rom::Bundle>) -> (Scratch, Sqlite) {
    let scratch = Scratch::new();
    let db = file_fixture(&scratch.path("database"));
    for bundle in bundles {
        db.commit(&bundle).unwrap();
    }
    (scratch, db)
}
fn barrier_bundle(id: &str, action: bool) -> rom::Bundle {
    let mut value = bundle(id);
    let source = value.receipt.row.clone();
    value.reactions[0].payload = if action {
        // Durable Action payloads are complete Invocation values, not inputs.
        let invocation = rom::Invocation {
            retry_epoch: value.reactions[0].cause.retry_epoch,
            kind: source.key.kind.clone(),
            id: source.key.id.clone(),
            expected: Some(source.revision),
            idempotency: value.reactions[0].id.clone(),
            operation: rom::Operation::Action {
                name: "probe-action".into(),
                input: rom::json!({"input": 1}),
            },
        };
        rom::WorkPayload::Action(serde_json::to_value(invocation).unwrap())
    } else {
        rom::WorkPayload::Notification {
            source,
            payload: rom::json!("notice"),
        }
    };
    value
}
fn assert_barrier(
    snapshot: ClaimPrefixSnapshot,
    reason: ClaimPrefixReason,
    width: u8,
    disposition: ClaimPrefixDisposition,
) {
    assert!(!snapshot.overflow && !snapshot.unavailable);
    assert_eq!(snapshot.entries.len(), 1);
    let entry = snapshot.entries[0];
    assert_eq!(
        (entry.reason, entry.width, entry.disposition),
        (reason, width, disposition)
    );
    assert_eq!(
        (
            entry.native_successes,
            entry.native_failures,
            entry.dropped_samples
        ),
        (1, 0, 0)
    );
}

#[test]
fn first_notification_and_action_are_admitted_singletons() {
    for action in [false, true] {
        let (scratch, db) = database_bundles(vec![barrier_bundle("00", action), bundle("01")]);
        let before = db.reaction_records().unwrap();
        let observation = db.observe_stages();
        let claims = db.reaction_claim_prefix(0, 16).unwrap();
        let after = db.reaction_records().unwrap();
        drop(db);
        let reopened = Sqlite::open(scratch.path("database")).unwrap();
        assert_eq!(reopened.reaction_records().unwrap(), after);
        drop(reopened);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].work.pending, before[0].pending);
        assert_eq!((claims[0].work.attempts, claims[0].work.generation), (1, 1));
        assert_eq!(after[1], before[1]);
        assert_barrier(
            observation.claim_prefix_snapshot(),
            if action {
                ClaimPrefixReason::ActionBarrier
            } else {
                ClaimPrefixReason::NotificationBarrier
            },
            1,
            ClaimPrefixDisposition::AdmittedSingleton,
        );
    }
}

#[test]
fn withheld_notification_action_and_duplicate_root_keep_their_budgets() {
    for kind in 0..3 {
        let first = bundle("00");
        let mut second = if kind < 2 {
            barrier_bundle("01", kind == 1)
        } else {
            bundle("01")
        };
        if kind == 2 {
            second.reactions[0].cause.root = first.reactions[0].cause.root.clone();
        }
        let (scratch, db) = database_bundles(vec![first, second]);
        let before = db.reaction_records().unwrap();
        let observation = db.observe_stages();
        let claims = db.reaction_claim_prefix(0, 16).unwrap();
        let after = db.reaction_records().unwrap();
        drop(db);
        let reopened = Sqlite::open(scratch.path("database")).unwrap();
        assert_eq!(reopened.reaction_records().unwrap(), after);
        drop(reopened);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].work.pending, before[0].pending);
        assert_eq!(after[1], before[1]);
        assert_eq!((after[1].attempts, after[1].generation), (0, 0));
        assert_barrier(
            observation.claim_prefix_snapshot(),
            match kind {
                0 => ClaimPrefixReason::NotificationBarrier,
                1 => ClaimPrefixReason::ActionBarrier,
                _ => ClaimPrefixReason::DuplicateRoot,
            },
            1,
            ClaimPrefixDisposition::Withheld,
        );
    }
}

#[test]
fn resolution_precedes_notification_and_withheld_resolution_is_not_published() {
    for first_barrier in [true, false] {
        let mut ordinary = bundle("00");
        ordinary.reactions[0].cause.started_at = 3600;
        let barrier = barrier_bundle(if first_barrier { "00" } else { "01" }, false);
        let input = if first_barrier {
            vec![barrier]
        } else {
            vec![ordinary, barrier]
        };
        let (scratch, db) = database_bundles(input);
        let before = db.reaction_records().unwrap();
        let observation = db.observe_stages();
        let claims = db.reaction_claim_prefix(3600, 16).unwrap();
        let after = db.reaction_records().unwrap();
        drop(db);
        let reopened = Sqlite::open(scratch.path("database")).unwrap();
        assert_eq!(reopened.reaction_records().unwrap(), after);
        drop(reopened);
        assert_eq!(claims.len(), 1);
        if first_barrier {
            assert!(claims[0].resolution_only);
            assert_eq!((claims[0].work.attempts, claims[0].work.generation), (0, 1));
            assert_eq!(claims[0].stop_reason, Some(rom::StopReason::Age));
        } else {
            assert_eq!(after[1], before[1]);
        }
        assert_barrier(
            observation.claim_prefix_snapshot(),
            ClaimPrefixReason::ResolutionBarrier,
            1,
            if first_barrier {
                ClaimPrefixDisposition::AdmittedSingleton
            } else {
                ClaimPrefixDisposition::Withheld
            },
        );
    }
}

#[test]
fn reconciliation_maintenance_is_changed_zero_width_not_idle() {
    let mut value = barrier_bundle("00", false);
    value.reactions[0].delivery_profile = rom::DeliveryProfile::ReconcileBeforeRetry;
    let (scratch, db) = database_bundles(vec![value]);
    let claim = crate::native_work_test_support::claim(&db);
    db.reaction_update(rom::WorkUpdate::DeliveryStarted {
        claim: claim.key(),
        now: 0,
    })
    .unwrap();
    let before = db.reaction_records().unwrap();
    let observation = db.observe_stages();
    let claims = db.reaction_claim_prefix(31, 16).unwrap();
    let after = db.reaction_records().unwrap();
    let old_claim_live = db.reaction_claim_live(&claim.key(), 31).unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    assert_eq!(reopened.reaction_records().unwrap(), after);
    drop(reopened);
    assert!(claims.is_empty());
    assert_eq!((before[0].attempts, before[0].generation), (1, 1));
    assert_eq!(after[0].pending, before[0].pending);
    assert_eq!(after[0].state, WorkState::AwaitingReconciliation);
    assert_eq!(after[0].attempts, before[0].attempts);
    // await_reconciliation invalidates the original claim's fencing generation.
    assert_eq!(after[0].generation, before[0].generation + 1);
    assert_eq!(old_claim_live, Some(false));
    assert_grouped_sample(
        observation.claim_prefix_snapshot(),
        ClaimPrefixReason::MaintenanceChanged,
        0,
    );
}

#[test]
fn precommit_and_publication_refusals_are_unavailable_without_commit_success() {
    for rejected_point in [0, 1] {
        let (scratch, db) = database(3);
        let before = db.reaction_records().unwrap();
        let observation = db.observe_stages();
        db.on_commit(Some(std::sync::Arc::new(move |point| {
            if point == rejected_point {
                Err(rom::Error::Storage)
            } else {
                Ok(())
            }
        })));
        let result = db.reaction_claim_prefix(0, 3);
        db.on_commit(None);
        let after = db.reaction_records().unwrap();
        drop(db);
        let reopened = Sqlite::open(scratch.path("database")).unwrap();
        let restored = reopened.reaction_records().unwrap();
        drop(reopened);
        assert_eq!(result, Err(rom::Error::NotCommitted));
        assert_eq!(after, before);
        assert_eq!(restored, before);
        let snapshot = observation.claim_prefix_snapshot();
        assert!(snapshot.unavailable && !snapshot.overflow);
        assert!(snapshot.entries.is_empty());
    }
}

#[test]
fn acknowledgement_unknown_retains_native_success_and_committed_claims() {
    let (scratch, db) = database(3);
    let observation = db.observe_stages();
    db.on_commit(Some(std::sync::Arc::new(|point| {
        if point == usize::MAX {
            Err(rom::Error::Storage)
        } else {
            Ok(())
        }
    })));
    let result = db.reaction_claim_prefix(0, 3);
    db.on_commit(None);
    let after = db.reaction_records().unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    let restored = reopened.reaction_records().unwrap();
    drop(reopened);
    assert_eq!(result, Err(rom::Error::Unknown));
    assert_eq!(restored, after);
    assert!(after.iter().all(|record| record.attempts == 1
        && record.generation == 1
        && matches!(record.state, WorkState::Leased { .. })));
    assert_grouped_sample(
        observation.claim_prefix_snapshot(),
        ClaimPrefixReason::RequestedLimit,
        3,
    );
}

#[test]
fn deferred_foreign_key_failure_records_native_failure_not_acknowledged_success() {
    let (scratch, db) = database(3);
    let before = db.reaction_records().unwrap();
    db.connection.lock().unwrap().execute_batch(
        "PRAGMA foreign_keys=ON;
         CREATE TABLE prefix_fault_parent(id INTEGER PRIMARY KEY);
         CREATE TABLE prefix_fault_child(parent INTEGER REFERENCES prefix_fault_parent(id) DEFERRABLE INITIALLY DEFERRED);
         CREATE TRIGGER prefix_fault AFTER UPDATE ON work_records BEGIN
           INSERT INTO prefix_fault_child VALUES(1);
         END;"
    ).unwrap();
    let observation = db.observe_stages();
    let result = db.reaction_claim_prefix(0, 3);
    // Canonical inventory rejects fixture-only tables. Check native rollback
    // directly, then remove only the injected fault schema before canonical reads.
    let (autocommit, fault_rows) = {
        let connection = db.connection.lock().unwrap();
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM prefix_fault_child", [], |row| {
                row.get(0)
            })
            .unwrap();
        let autocommit = connection.is_autocommit();
        connection.execute_batch(
            "DROP TRIGGER prefix_fault; DROP TABLE prefix_fault_child; DROP TABLE prefix_fault_parent;"
        ).unwrap();
        (autocommit, rows)
    };
    let after = db.reaction_records().unwrap();
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    let restored = reopened.reaction_records().unwrap();
    drop(reopened);
    assert_eq!(result, Err(rom::Error::Unknown));
    assert!(autocommit);
    assert_eq!(fault_rows, 0);
    assert_eq!(after, before);
    assert_eq!(restored, before);
    let snapshot = observation.claim_prefix_snapshot();
    assert!(!snapshot.unavailable && !snapshot.overflow);
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        (
            snapshot.entries[0].native_successes,
            snapshot.entries[0].native_failures
        ),
        (0, 1)
    );
    assert_eq!(
        (snapshot.entries[0].reason, snapshot.entries[0].width),
        (ClaimPrefixReason::RequestedLimit, 3)
    );
}

#[test]
fn disabled_and_singleton_compatibility_paths_have_no_grouped_samples() {
    let (scratch, db) = database(2);
    assert!(db.stage_observation.get().is_none());
    assert_eq!(db.reaction_claim_prefix(0, 2).unwrap().len(), 2);
    assert!(db.stage_observation.get().is_none());
    let late = db.observe_stages();
    assert!(late.claim_prefix_snapshot().entries.is_empty());
    drop(db);
    let reopened = Sqlite::open(scratch.path("database")).unwrap();
    assert!(
        reopened
            .reaction_records()
            .unwrap()
            .iter()
            .all(|r| r.attempts == 1 && r.generation == 1)
    );
    drop(reopened);

    let (_scratch, db) = database(1);
    let observation = db.observe_stages();
    assert_eq!(db.reaction_claim_prefix(0, 1).unwrap().len(), 1);
    drop(db);
    assert!(observation.claim_prefix_snapshot().entries.is_empty());
    assert_eq!(
        observation.work_utilization_snapshot().entries[0].origin,
        crate::WorkCommitOrigin::ClaimPrefix
    );

    let scratch = Scratch::new();
    let db = Sqlite::open_connection_for_layout(
        &scratch.path("predecessor"),
        rom::StorageLimits::default(),
        rom_backup::BackupLimits::default(),
        None,
        false,
    )
    .unwrap();
    db.register(&[crate::native_work_test_support::descriptor()])
        .unwrap();
    db.commit(&bundle("00")).unwrap();
    let observation = db.observe_stages();
    assert_eq!(db.reaction_claim_prefix(0, 16).unwrap().len(), 1);
    drop(db);
    assert!(observation.claim_prefix_snapshot().entries.is_empty());
    assert_eq!(
        observation.work_utilization_snapshot().entries[0].origin,
        crate::WorkCommitOrigin::ClaimPrefix
    );
}
