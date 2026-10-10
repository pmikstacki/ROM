use super::*;

fn epochs(current: u64, admission_floor: u64, replay_floor: u64) -> RetryEpochs {
    RetryEpochs {
        current,
        admission_floor,
        replay_floor,
    }
}
fn row() -> Row {
    Row {
        key: Key {
            kind: "notes".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(json!({"text":"one"})),
        protected: ProtectedMetadata::default(),
    }
}
fn pending(id: &str, root: &str, epoch: u64) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            retry_epoch: epoch,
            root: root.into(),
            parent: None,
            depth: 0,
            started_at: 0,
            path: vec![],
        },
        definition: "react".into(),
        version: 1,
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        service_key: "service".into(),
        payload: WorkPayload::Source(row()),
    }
}
fn state() -> StorageState {
    StorageState::new(StorageLimits::default()).unwrap()
}
fn claim(state: &mut StorageState) -> ClaimKey {
    let WorkResult::Claimed(claim) = state.work.apply(WorkUpdate::Claim { now: 0 }).unwrap() else {
        panic!()
    };
    claim.key()
}
fn finish(state: &mut StorageState, outcome: WorkOutcome) {
    let claim = claim(state);
    state
        .work
        .apply(WorkUpdate::Finish {
            claim,
            now: 0,
            outcome,
        })
        .unwrap();
}
fn encoded(state: &StorageState) -> Value {
    serde_json::to_value(state).unwrap()
}

#[test]
fn retry_epoch_defaults_and_journal_retention_are_explicit_and_repeatable() {
    let mut state = state();
    let mut legacy = encoded(&state);
    legacy.as_object_mut().unwrap().remove("retry_epochs");
    assert_eq!(
        serde_json::from_value::<StorageState>(legacy)
            .unwrap()
            .retry_epochs(),
        RetryEpochs::default()
    );
    state.head = 3;
    state.receipts = 3;
    state.effects = 2;
    state.events = (1..=3)
        .map(|position| JournalEvent {
            position,
            identity: format!("event-{position}"),
            row: row(),
        })
        .collect();
    let generation = state.generation.clone();
    let report = state.apply_retention(epochs(2, 1, 1), 2, 2, 1).unwrap();
    assert_eq!(report.journal_identities, ["event-1", "event-2"]);
    assert_eq!((report.work_records, report.work_roots), (0, 0));
    assert_eq!(state.journal_floor(), 2);
    assert_eq!(state.head, 3);
    assert_eq!(state.generation, generation);
    assert_eq!(state.retry_epochs(), epochs(2, 1, 1));
    assert!(
        state
            .apply_retention(epochs(2, 1, 1), 2, 1, 0)
            .unwrap()
            .journal_identities
            .is_empty()
    );
    state
        .validate_archive(1, 0, &[("event-3".into(), row())])
        .unwrap();
}
#[test]
fn invalid_retention_never_mutates_state() {
    let mut state = state();
    state.apply_retention(epochs(3, 2, 1), 0, 0, 0).unwrap();
    for (epochs, through, receipts) in [
        (epochs(2, 2, 1), 0, 0),
        (epochs(3, 1, 1), 0, 0),
        (epochs(3, 2, 0), 0, 0),
        (epochs(3, 1, 2), 0, 0),
        (epochs(3, 2, 1), 1, 0),
        (epochs(3, 2, 1), 0, 1),
    ] {
        let before = encoded(&state);
        assert!(state.apply_retention(epochs, through, receipts, 0).is_err());
        assert_eq!(encoded(&state), before);
    }
}
#[test]
fn retention_removes_complete_roots_and_preserves_newer_budgets() {
    let mut state = state();
    state.apply_retention(epochs(2, 0, 0), 0, 0, 0).unwrap();
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![
                pending("a", "old", 0),
                pending("b", "old", 0),
                pending("z", "new", 2),
            ],
        )
        .unwrap();
    finish(&mut state, WorkOutcome::Done);
    finish(&mut state, WorkOutcome::Done);
    let retained = state.work.records().pop().unwrap();
    let report = state.apply_retention(epochs(2, 1, 1), 0, 0, 0).unwrap();
    assert_eq!((report.work_records, report.work_roots), (2, 1));
    assert_eq!(state.work.records(), [retained]);
    state.validate_archive(0, 0, &[]).unwrap();
}
#[test]
fn unresolved_or_unknown_roots_block_retention_atomically() {
    for stopped in [false, true] {
        let mut state = state();
        state
            .work
            .enqueue(
                &ReactionLimits::default(),
                vec![pending("a", "root", 0), pending("b", "root", 0)],
            )
            .unwrap();
        finish(&mut state, WorkOutcome::Done);
        if stopped {
            finish(&mut state, WorkOutcome::Stop(StopReason::Denied));
        }
        let before = encoded(&state);
        assert_eq!(
            state.apply_retention(epochs(1, 1, 1), 0, 0, 0).unwrap_err(),
            Error::Conflict
        );
        assert_eq!(encoded(&state), before);
    }
    let mut state = state();
    let mut work = pending("notice", "root", 0);
    work.payload = WorkPayload::Notification {
        source: row(),
        payload: json!({"opaque":true}),
    };
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![work])
        .unwrap();
    let claim = claim(&mut state);
    state
        .work
        .apply(WorkUpdate::DeliveryStarted {
            claim: claim.clone(),
            now: 0,
        })
        .unwrap();
    state
        .work
        .apply(WorkUpdate::Finish {
            claim,
            now: 0,
            outcome: WorkOutcome::Done,
        })
        .unwrap();
    let before = encoded(&state);
    assert_eq!(
        state.apply_retention(epochs(1, 1, 1), 0, 0, 0).unwrap_err(),
        Error::Conflict
    );
    assert_eq!(encoded(&state), before);
}
#[test]
fn causal_admission_requires_the_current_stored_claim_in_the_same_epoch() {
    let mut state = state();
    state.apply_retention(epochs(2, 2, 0), 0, 0, 0).unwrap();
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![pending("a", "root", 1)])
        .unwrap();
    let claim = claim(&mut state);
    assert_eq!(
        state.check_retry_epoch(1, false, None),
        Err(Error::IdentityExpired)
    );
    state
        .check_retry_epoch(1, false, Some((&claim, 0)))
        .unwrap();
    assert_eq!(
        state.check_retry_epoch(0, false, Some((&claim, 0))),
        Err(Error::Conflict)
    );
    assert_eq!(
        state.check_retry_epoch(1, false, Some((&claim, 30))),
        Err(Error::Conflict)
    );
    let stale = ClaimKey {
        id: claim.id.clone(),
        generation: claim.generation + 1,
    };
    assert_eq!(
        state.check_retry_epoch(1, false, Some((&stale, 0))),
        Err(Error::Conflict)
    );
    state.check_retry_epoch(1, true, Some((&stale, 0))).unwrap();
    assert!(state.check_retry_epoch(3, true, None).is_err());
}
#[test]
fn archive_and_enqueue_reject_inconsistent_work_epochs() {
    let mut state = state();
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![pending("a", "root", 1)])
        .unwrap();
    assert!(state.validate_archive(0, 0, &[]).is_err());
    let before = encoded(&state);
    assert!(
        state
            .work
            .enqueue(&ReactionLimits::default(), vec![pending("b", "root", 0)])
            .is_err()
    );
    assert_eq!(encoded(&state), before);
}

#[test]
fn only_accepted_done_notifications_are_retirable() {
    for outcome in [
        None,
        Some(DeliveryOutcome::Unknown),
        Some(DeliveryOutcome::TimedOut),
        Some(DeliveryOutcome::Panicked),
        Some(DeliveryOutcome::Retryable),
        Some(DeliveryOutcome::Permanent),
        Some(DeliveryOutcome::Accepted),
    ] {
        let mut state = state();
        let mut work = pending("notice", "root", 0);
        work.payload = WorkPayload::Notification {
            source: row(),
            payload: json!({"opaque":true}),
        };
        state
            .work
            .enqueue(&ReactionLimits::default(), vec![work])
            .unwrap();
        finish(&mut state, WorkOutcome::Done);
        // Simulate a persisted low-level completion with no accepted delivery proof.
        let mut serialized = encoded(&state);
        serialized["work"]["work"]["notice"]["delivery"] = serde_json::to_value(&outcome).unwrap();
        state = serde_json::from_value(serialized).unwrap();
        let before = encoded(&state);
        let result = state.apply_retention(epochs(1, 1, 1), 0, 0, 0);
        if outcome == Some(DeliveryOutcome::Accepted) {
            assert_eq!(result.unwrap().work_records, 1);
        } else {
            assert_eq!(result.unwrap_err(), Error::Conflict);
            assert_eq!(encoded(&state), before);
        }
    }
}

#[test]
fn archive_requires_action_epoch_to_match_cause_and_floors_to_cover_obligations() {
    let mut state = state();
    state.apply_retention(epochs(2, 0, 0), 0, 0, 0).unwrap();
    let mut work = pending("action", "root", 1);
    work.payload = WorkPayload::Action(
        serde_json::to_value(Invocation {
            retry_epoch: 0,
            kind: "notes".into(),
            id: "one".into(),
            expected: Some(1),
            idempotency: "action".into(),
            operation: Operation::Delete,
        })
        .unwrap(),
    );
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![work])
        .unwrap();
    assert_eq!(state.validate_archive(0, 0, &[]), Err(Error::Storage));
    let mut serialized = encoded(&state);
    serialized["work"]["work"]["action"]["pending"]["payload"]["Action"]["retry_epoch"] = json!(1);
    state = serde_json::from_value(serialized).unwrap();
    state.validate_archive(0, 0, &[]).unwrap();
    let mut serialized = encoded(&state);
    serialized["retry_epochs"] = serde_json::to_value(epochs(2, 2, 2)).unwrap();
    state = serde_json::from_value(serialized).unwrap();
    assert_eq!(state.validate_archive(0, 0, &[]), Err(Error::Storage));
}

#[test]
fn replay_floor_and_resolution_only_claims_cannot_be_bypassed() {
    let mut state = state();
    state.apply_retention(epochs(2, 2, 1), 0, 0, 0).unwrap();
    assert_eq!(
        state.check_retry_epoch(0, true, None),
        Err(Error::IdentityExpired)
    );
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![pending("a", "root", 1)])
        .unwrap();
    let WorkResult::Claimed(claim) = state.work.apply(WorkUpdate::Claim { now: 3601 }).unwrap()
    else {
        panic!()
    };
    assert!(claim.resolution_only);
    assert_eq!(
        state.check_retry_epoch(1, false, Some((&claim.key(), 3601))),
        Err(Error::Conflict)
    );
    state
        .check_retry_epoch(1, true, Some((&claim.key(), 3601)))
        .unwrap();
}

#[test]
fn materialization_cannot_change_the_parent_epoch() {
    let mut state = state();
    state
        .work
        .enqueue(&ReactionLimits::default(), vec![pending("a", "root", 0)])
        .unwrap();
    let claim = claim(&mut state);
    let mut child = pending("b", "root", 1);
    child.payload = WorkPayload::Action(json!({"opaque":true}));
    let before = encoded(&state);
    assert!(
        state
            .work
            .apply(WorkUpdate::Materialize {
                claim,
                now: 0,
                children: vec![child]
            })
            .is_err()
    );
    assert_eq!(encoded(&state), before);
}

#[test]
fn bundle_cannot_persist_future_or_unrelated_reaction_epochs() {
    let mut state = state();
    state.apply_retention(epochs(2, 0, 0), 0, 0, 0).unwrap();
    for epoch in [1, 3] {
        let before = encoded(&state);
        let bundle = Bundle {
            expected: None,
            receipt: Receipt {
                identity: "request".into(),
                fingerprint: "input".into(),
                row: row(),
                replay_version: Some(1),
                retry_epoch: 0,
            },
            changed: true,
            effects: vec![],
            reactions: vec![pending("source", "request", epoch)],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        };
        assert!(state.bundle(&bundle).is_err());
        assert_eq!(encoded(&state), before);
    }
}

#[test]
fn work_update_rejects_action_epoch_mismatch_without_finishing_source() {
    let mut state = state();
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("source", "root", 0)],
        )
        .unwrap();
    let claim = claim(&mut state);
    let mut child = pending("action", "root", 0);
    child.payload = WorkPayload::Action(
        serde_json::to_value(Invocation {
            retry_epoch: 1,
            kind: "notes".into(),
            id: "one".into(),
            expected: Some(1),
            idempotency: "action".into(),
            operation: Operation::Delete,
        })
        .unwrap(),
    );
    let before = encoded(&state);
    assert!(
        state
            .update_work(WorkUpdate::Materialize {
                claim,
                now: 0,
                children: vec![child]
            })
            .is_err()
    );
    assert_eq!(encoded(&state), before);
}
