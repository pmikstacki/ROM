use super::*;
use crate::operator::*;

fn state() -> StorageState {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    let pending = PendingWork {
        id: "work".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: None,
            depth: 0,
            started_at: 10,
            path: vec![],
        },
        definition: "mail".into(),
        version: 1,
        service_key: "secret-service".into(),
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        payload: WorkPayload::Notification {
            source: Row {
                key: Key {
                    kind: "source".into(),
                    id: "1".into(),
                },
                revision: 1,
                value: Some(json!({"secret":"payload"})),
                protected: ProtectedMetadata::default(),
            },
            payload: json!({"secret":"message"}),
        },
    };
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![pending])
        .unwrap();
    state
}
fn control(state: &StorageState, key: &str) -> StorageWorkControl {
    let snapshot = state.work_snapshot(16, 65536).unwrap();
    StorageWorkControl {
        principal: "operator-a".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("work"),
            expected: snapshot.version(&snapshot.records[0]),
            key: key.into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        },
        decision: WorkControlDecision::Retry,
        now: 11,
    }
}
fn stopped_action() -> StorageState {
    let mut state = state();
    let invocation = Invocation {
        retry_epoch: 0,
        kind: "target".into(),
        id: "2".into(),
        expected: Some(7),
        idempotency: "frozen-action".into(),
        operation: Operation::Action {
            name: "resolve".into(),
            input: json!({"secret":"input"}),
        },
    };
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["pending"]["payload"] = json!({"Action":invocation});
    state.work = serde_json::from_value(wire).unwrap();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(c) => c,
        _ => panic!(),
    };
    state
        .update_work(WorkUpdate::Finish {
            claim: claim.key(),
            now: 11,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        })
        .unwrap();
    state
}
fn bytes(state: &StorageState) -> Vec<u8> {
    serde_json::to_vec(state).unwrap()
}
#[test]
fn exact_replay_precedes_cas_and_preserves_identity_budgets() {
    let mut state = state();
    let request = control(&state, "key");
    let before = state.work.records()[0].clone();
    let receipt = state.control_work(&request).unwrap();
    let after = state.work.records()[0].clone();
    assert_eq!(receipt.result.outcome, WorkControlOutcome::Scheduled);
    assert_eq!(after.pending, before.pending);
    assert_eq!(after.attempts, before.attempts);
    assert_eq!(after.generation, before.generation + 1);
    assert_eq!(after.revision, before.revision + 1);
    let persisted = bytes(&state);
    let replay = state.control_work(&request).unwrap();
    assert!(replay.result.replayed);
    assert_eq!(replay.result.version, receipt.result.version);
    assert_eq!(persisted, bytes(&state));
    assert_eq!(state.operator_receipt_count(), 1);
    let mut changed = request.clone();
    changed.request.expected.revision += 1;
    assert!(matches!(
        state.control_work(&changed),
        Err(Error::IdentityMismatch)
    ));
    assert_eq!(persisted, bytes(&state));
    let mut other = request.clone();
    other.principal = "operator-b".into();
    assert!(matches!(state.control_work(&other), Err(Error::Conflict)));
    assert_eq!(persisted, bytes(&state));
}
#[test]
fn stale_versions_and_receipt_limits_reject_atomically() {
    let mut state = state();
    let mut stale = control(&state, "stale");
    stale.request.expected.revision += 1;
    let before = bytes(&state);
    assert!(matches!(state.control_work(&stale), Err(Error::Conflict)));
    assert_eq!(before, bytes(&state));
    state.operator.limits.max_records = 1;
    let first = control(&state, "first");
    state.control_work(&first).unwrap();
    let second = control(&state, "second");
    let before = bytes(&state);
    assert!(matches!(
        state.control_work(&second),
        Err(Error::Overloaded)
    ));
    assert_eq!(before, bytes(&state));
    state.control_work(&first).unwrap();
}
#[test]
fn active_leases_hard_budgets_and_wrong_decisions_are_refused() {
    let mut state = state();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(c) => c,
        _ => panic!(),
    };
    let request = control(&state, "leased");
    let before = bytes(&state);
    assert!(matches!(state.control_work(&request), Err(Error::Conflict)));
    assert_eq!(before, bytes(&state));
    state
        .update_work(WorkUpdate::Finish {
            claim: claim.key(),
            now: 11,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        })
        .unwrap();
    let mut request = control(&state, "wrong");
    request.decision = WorkControlDecision::DeliveryAccepted {
        evidence: "verified".into(),
    };
    let before = bytes(&state);
    assert!(matches!(state.control_work(&request), Err(Error::Conflict)));
    assert_eq!(before, bytes(&state));
    let mut wire = serde_json::to_value(&state).unwrap();
    wire["work"]["work"]["work"]["attempts"] = json!(3);
    wire["work"]["work"]["work"]["generation"] = json!(3);
    wire["work"]["roots"]["root"] = json!(3);
    let mut exhausted: StorageState = serde_json::from_value(wire).unwrap();
    let request = control(&exhausted, "budget");
    let before = bytes(&exhausted);
    assert!(matches!(
        exhausted.control_work(&request),
        Err(Error::Conflict)
    ));
    assert_eq!(before, bytes(&exhausted));
}
#[test]
fn receipt_capacity_cannot_block_worker_completion_or_restore_replay() {
    let mut state = state();
    state.operator.limits.max_records = 1;
    let request = control(&state, "retry");
    state.control_work(&request).unwrap();
    let claim = match state.update_work(WorkUpdate::Claim { now: 11 }).unwrap() {
        WorkResult::Claimed(c) => c,
        _ => panic!(),
    };
    state
        .update_work(WorkUpdate::DeliveryFinished {
            claim: claim.key(),
            now: 12,
            outcome: DeliveryOutcome::Accepted,
        })
        .unwrap();
    state.prepare_restore().unwrap();
    assert!(state.control_work(&request).unwrap().result.replayed);
    assert_eq!(state.work.records()[0].state, WorkState::Done);
}
#[test]
fn snapshot_charges_receipts_and_all_metadata_without_truncation() {
    let mut state = state();
    let request = control(&state, "retry");
    state.control_work(&request).unwrap();
    assert!(matches!(
        state.work_snapshot(1, 65536),
        Err(Error::TooLarge)
    ));
    let snapshot = state.work_snapshot(2, 65536).unwrap();
    assert_eq!(snapshot.operator.receipts.len(), 1);
    let size = serde_json::to_vec(&snapshot).unwrap().len();
    assert!(matches!(
        state.work_snapshot(2, size - 1),
        Err(Error::TooLarge)
    ));
    state.work_snapshot(2, size).unwrap();
}
#[test]
fn malformed_claim_identity_cannot_publish_candidate_before_error() {
    let mut state = state();
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["pending"]["id"] = json!("wrong-key");
    state.work = serde_json::from_value(wire).unwrap();
    let before = bytes(&state);
    assert!(matches!(
        state.work.apply(WorkUpdate::Claim { now: 10 }),
        Err(Error::Storage)
    ));
    assert_eq!(before, bytes(&state));
}
#[test]
fn invalid_snapshot_generation_and_archive_generation_are_rejected() {
    let state = state();
    for generation in [String::new(), "x".repeat(129)] {
        let mut wire = serde_json::to_value(&state).unwrap();
        wire["generation"] = json!(generation);
        let invalid: StorageState = serde_json::from_value(wire).unwrap();
        assert!(invalid.work_snapshot(16, 65536).is_err());
        assert!(invalid.validate_archive(0, 0, &[]).is_err());
    }
}
#[test]
fn unverified_receipt_evidence_and_invalid_retained_epoch_fail_archive_validation() {
    let mut state = state();
    let mut request = control(&state, "verified");
    request.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    request.decision = WorkControlDecision::DeliveryAccepted {
        evidence: "provider-proof".into(),
    };
    state.control_work(&request).unwrap();
    state.validate_archive(0, 0, &[]).unwrap();
    for field in ["evidence", "epoch"] {
        let mut wire = serde_json::to_value(&state).unwrap();
        let receipt = wire["operator"]["receipts"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        if field == "evidence" {
            receipt["evidence"] = Value::Null;
        } else {
            receipt["request"]["retry_epoch"] = json!(1);
        }
        let invalid: StorageState = serde_json::from_value(wire).unwrap();
        assert!(invalid.validate_archive(0, 0, &[]).is_err());
    }
}
#[test]
fn retry_cannot_bypass_fanout_stops() {
    let mut state = state();
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["state"] = json!({"Stopped":"Fanout"});
    state.work = serde_json::from_value(wire).unwrap();
    let request = control(&state, "fanout");
    let before = bytes(&state);
    assert!(matches!(state.control_work(&request), Err(Error::Conflict)));
    assert_eq!(before, bytes(&state));
}
#[test]
fn epochs_control_new_admission_and_replay_without_destroying_historical_receipts() {
    let mut state = state();
    let request = control(&state, "old");
    state.control_work(&request).unwrap();
    let mut wire = serde_json::to_value(&state).unwrap();
    wire["retry_epochs"] = json!({"current":1,"admission_floor":1,"replay_floor":0});
    let mut state: StorageState = serde_json::from_value(wire).unwrap();
    assert!(state.control_work(&request).unwrap().result.replayed);
    let fresh = control(&state, "fresh");
    let before = bytes(&state);
    assert!(matches!(
        state.control_work(&fresh),
        Err(Error::IdentityExpired)
    ));
    assert_eq!(before, bytes(&state));
    let mut future = fresh.clone();
    future.request.retry_epoch = 2;
    assert!(matches!(
        state.control_work(&future),
        Err(Error::Invalid { .. })
    ));
    assert_eq!(before, bytes(&state));
    let mut resolve = control(&state, "resolve");
    resolve.request.retry_epoch = 1;
    resolve.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    resolve.decision = WorkControlDecision::DeliveryAccepted {
        evidence: "accepted".into(),
    };
    state.control_work(&resolve).unwrap();
    state
        .apply_retention(
            RetryEpochs {
                current: 1,
                admission_floor: 1,
                replay_floor: 1,
            },
            0,
            0,
            0,
        )
        .unwrap();
    assert!(state.work.records().is_empty());
    assert_eq!(state.operator_receipt_count(), 2);
    assert!(matches!(
        state.control_work(&request),
        Err(Error::IdentityExpired)
    ));
    assert!(state.control_work(&resolve).unwrap().result.replayed);
    state.prepare_restore().unwrap();
    assert!(state.control_work(&resolve).unwrap().result.replayed);
}
#[test]
fn reconciliation_preserves_original_budgets_and_fences_stale_completion() {
    let mut state = state();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(c) => c,
        _ => panic!(),
    };
    state
        .update_work(WorkUpdate::Finish {
            claim: claim.key(),
            now: 11,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        })
        .unwrap();
    let before = state.work.records()[0].clone();
    let mut retry = control(&state, "retry");
    retry.now = 12;
    state.control_work(&retry).unwrap();
    let after = state.work.records()[0].clone();
    assert_eq!(before.pending, after.pending);
    assert_eq!(before.attempts, after.attempts);
    assert_eq!(before.generation + 1, after.generation);
    assert_eq!(before.revision + 1, after.revision);
    let persisted = bytes(&state);
    assert!(matches!(
        state.update_work(WorkUpdate::Finish {
            claim: claim.key(),
            now: 12,
            outcome: WorkOutcome::Done
        }),
        Err(Error::Conflict)
    ));
    assert_eq!(persisted, bytes(&state));
    let mut resolve = control(&state, "resolve");
    resolve.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    resolve.decision = WorkControlDecision::DeliveryNotAccepted {
        evidence: "terminal-rejection".into(),
    };
    resolve.now = 3600 + 10;
    assert_eq!(
        state.control_work(&resolve).unwrap().result.outcome,
        WorkControlOutcome::Stopped(StopReason::Age)
    );
    assert_eq!(state.work.records()[0].attempts, 1);
    let mut accepted = control(&state, "accepted");
    accepted.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    accepted.decision = WorkControlDecision::DeliveryAccepted {
        evidence: "late-accepted".into(),
    };
    assert_eq!(
        state.control_work(&accepted).unwrap().result.outcome,
        WorkControlOutcome::Completed
    );
}
#[test]
fn byte_capacity_and_revision_overflow_cannot_leave_a_receipt() {
    let mut state = state();
    let request = control(&state, "bytes");
    state.operator.limits.max_bytes = 1;
    let before = bytes(&state);
    assert!(matches!(
        state.control_work(&request),
        Err(Error::Overloaded)
    ));
    assert_eq!(before, bytes(&state));
    state.operator.limits.max_bytes = 1048576;
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["revision"] = json!(u64::MAX);
    state.work = serde_json::from_value(wire).unwrap();
    let request = control(&state, "overflow");
    let before = bytes(&state);
    assert!(matches!(state.control_work(&request), Err(Error::TooLarge)));
    assert_eq!(before, bytes(&state));
    assert_eq!(state.operator_receipt_count(), 0);
}
#[test]
fn delivery_profile_and_operator_metadata_are_required_and_category_checked() {
    let mut state = state();
    let mut wire = serde_json::to_value(&state).unwrap();
    wire.as_object_mut().unwrap().remove("operator");
    assert!(serde_json::from_value::<StorageState>(wire).is_err());
    let mut pending = state.work.records()[0].pending.clone();
    let mut wire = serde_json::to_value(&pending).unwrap();
    wire.as_object_mut().unwrap().remove("delivery_profile");
    assert!(serde_json::from_value::<PendingWork>(wire).is_err());
    pending.payload = WorkPayload::Source(match pending.payload {
        WorkPayload::Notification { source, .. } => source,
        _ => panic!(),
    });
    pending.id = "source".into();
    pending.delivery_profile = DeliveryProfile::ProviderDeduplicated;
    let before = bytes(&state);
    assert!(matches!(
        state
            .work
            .enqueue(&ReactionLimits::default(), vec![pending]),
        Err(Error::Conflict)
    ));
    assert_eq!(before, bytes(&state));
}
#[test]
fn frozen_action_resolution_completes_without_consuming_attempts_or_mutation_budget() {
    let mut state = state();
    let invocation = Invocation {
        retry_epoch: 0,
        kind: "target".into(),
        id: "2".into(),
        expected: Some(7),
        idempotency: "frozen-action".into(),
        operation: Operation::Action {
            name: "resolve".into(),
            input: json!({"secret":"input"}),
        },
    };
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["pending"]["payload"] = json!({"Action":invocation});
    state.work = serde_json::from_value(wire).unwrap();
    let before = state.work.records()[0].clone();
    let mut resolve = control(&state, "action");
    resolve.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    resolve.decision = WorkControlDecision::ActionCommitted;
    let receipt = state.control_work(&resolve).unwrap();
    assert_eq!(receipt.result.outcome, WorkControlOutcome::Completed);
    assert_eq!(
        receipt.scope.target,
        Some(Key {
            kind: "target".into(),
            id: "2".into()
        })
    );
    assert_eq!(receipt.scope.source, None);
    assert_eq!(receipt.evidence, None);
    let after = state.work.records()[0].clone();
    assert_eq!(before.pending, after.pending);
    assert_eq!(after.attempts, 0);
    assert_eq!(state.work_snapshot(16, 65536).unwrap().roots["root"], 0);
    state.validate_archive(0, 0, &[]).unwrap();
}
#[test]
fn persisted_hold_is_not_claimable_and_requires_verified_reconciliation() {
    let mut state = state();
    let claim = match state.update_work(WorkUpdate::Claim { now: 10 }).unwrap() {
        WorkResult::Claimed(c) => c,
        _ => panic!(),
    };
    state
        .update_work(WorkUpdate::DeliveryStarted {
            claim: claim.key(),
            now: 11,
        })
        .unwrap();
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["work"]["pending"]["delivery_profile"] = json!("ReconcileBeforeRetry");
    wire["work"]["work"]["state"] = json!("AwaitingReconciliation");
    state.work = serde_json::from_value(wire).unwrap();
    state.validate_archive(0, 0, &[]).unwrap();
    state.prepare_restore().unwrap();
    assert_eq!(
        state.update_work(WorkUpdate::Claim { now: 12 }).unwrap(),
        WorkResult::Idle
    );
    let retry = control(&state, "held");
    let before = bytes(&state);
    assert!(matches!(state.control_work(&retry), Err(Error::Conflict)));
    assert_eq!(before, bytes(&state));
    let mut resolve = control(&state, "verified");
    resolve.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    resolve.decision = WorkControlDecision::DeliveryNotAccepted {
        evidence: "terminal-proof".into(),
    };
    assert_eq!(
        state.control_work(&resolve).unwrap().result.outcome,
        WorkControlOutcome::Scheduled
    );
    assert_eq!(state.work.records()[0].attempts, 1);
}
#[test]
fn stopped_action_retry_retains_redacted_failure_history_through_replay_retirement_and_restore() {
    let state = stopped_action();
    let mut request = control(&state, "retry-history");
    request.request.retry_epoch = 1;
    let mut wire = serde_json::to_value(&state).unwrap();
    wire["retry_epochs"] = json!({"current":1,"admission_floor":0,"replay_floor":0});
    let mut state: StorageState = serde_json::from_value(wire).unwrap();
    let before = state.work.records()[0].clone();
    let receipt = state.control_work(&request).unwrap();
    let prior = serde_json::to_value(&receipt).unwrap()["prior"].clone();
    assert_eq!(prior, json!({"state":{"Stopped":"Denied"},"delivery":null}));
    let after = state.work.records()[0].clone();
    assert_eq!(after.state, WorkState::Pending);
    assert_eq!(after.pending, before.pending);
    assert_eq!(after.attempts, before.attempts);
    assert_eq!(
        serde_json::to_value(state.control_work(&request).unwrap()).unwrap()["prior"],
        prior
    );
    let mut complete = control(&state, "complete-history");
    complete.request.retry_epoch = 1;
    complete.request.operation = WorkControlOperation::Reconcile { evidence_ref: None };
    complete.decision = WorkControlDecision::ActionCommitted;
    state.control_work(&complete).unwrap();
    state
        .apply_retention(
            RetryEpochs {
                current: 1,
                admission_floor: 1,
                replay_floor: 1,
            },
            0,
            0,
            0,
        )
        .unwrap();
    assert!(state.work.records().is_empty());
    state.prepare_restore().unwrap();
    let restored: StorageState = serde_json::from_slice(&bytes(&state)).unwrap();
    state = restored;
    let replay = state.control_work(&request).unwrap();
    assert!(replay.result.replayed);
    assert_eq!(serde_json::to_value(replay).unwrap()["prior"], prior);
    state.validate_archive(0, 0, &[]).unwrap();
}
#[test]
fn current_receipt_decoding_requires_prior_state_and_explicit_delivery() {
    let mut state = stopped_action();
    let request = control(&state, "required-history");
    let receipt = state.control_work(&request).unwrap();
    let wire = serde_json::to_value(receipt).unwrap();
    let mut missing = wire.clone();
    missing.as_object_mut().unwrap().remove("prior");
    assert!(serde_json::from_value::<WorkControlReceipt>(missing).is_err());
    for field in ["state", "delivery"] {
        let mut missing = wire.clone();
        missing["prior"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<WorkControlReceipt>(missing).is_err());
    }
    let mut unknown = wire;
    unknown["prior"]["payload"] = json!({"secret":"input"});
    assert!(serde_json::from_value::<WorkControlReceipt>(unknown).is_err());
}
#[test]
fn archive_validation_rejects_incoherent_retained_prior_control_states() {
    let mut state = stopped_action();
    let request = control(&state, "history");
    state.control_work(&request).unwrap();
    for prior in [
        json!({"state":"Leased","delivery":null}),
        json!({"state":"Done","delivery":null}),
        json!({"state":"AwaitingReconciliation","delivery":null}),
        json!({"state":{"Stopped":"Fanout"},"delivery":null}),
        json!({"state":{"Stopped":"Denied"},"delivery":"Unknown"}),
    ] {
        let mut wire = serde_json::to_value(&state).unwrap();
        let receipt = wire["operator"]["receipts"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        receipt["prior"] = prior;
        let invalid: StorageState = serde_json::from_value(wire).unwrap();
        assert!(invalid.validate_archive(0, 0, &[]).is_err());
    }
}
