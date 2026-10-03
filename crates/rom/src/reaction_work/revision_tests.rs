use super::*;

fn pending(id: &str, payload: WorkPayload) -> PendingWork {
    PendingWork {
        id: id.into(),
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
        delivery_profile: DeliveryProfile::AtLeastOnce,
        service_key: "service".into(),
        payload,
    }
}
fn ledger(payload: WorkPayload) -> WorkLedger {
    let mut ledger = WorkLedger::default();
    ledger
        .enqueue(&ReactionLimits::default(), vec![pending("work", payload)])
        .unwrap();
    ledger
}
fn claim(ledger: &mut WorkLedger, now: u64) -> WorkClaim {
    match ledger.apply(WorkUpdate::Claim { now }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim: {other:?}"),
    }
}
fn revision(ledger: &WorkLedger) -> u64 {
    serde_json::to_value(&ledger.records()[0]).unwrap()["revision"]
        .as_u64()
        .expect("work revision")
}
fn notification() -> WorkPayload {
    WorkPayload::Notification {
        source: Row {
            key: Key {
                kind: "test".into(),
                id: "1".into(),
            },
            revision: 1,
            value: Some(json!({})),
            protected: ProtectedMetadata::default(),
        },
        payload: json!({}),
    }
}
#[test]
fn claim_retry_and_terminal_updates_invalidate_each_prior_revision() {
    let mut ledger = ledger(WorkPayload::Action(json!({})));
    assert_eq!(revision(&ledger), 0);
    let first = claim(&mut ledger, 10);
    assert_eq!(revision(&ledger), 1);
    assert_eq!(serde_json::to_value(&first.work).unwrap()["revision"], 1);
    ledger
        .apply(WorkUpdate::Finish {
            claim: first.key(),
            now: 11,
            outcome: WorkOutcome::Retry,
        })
        .unwrap();
    assert_eq!(revision(&ledger), 2);
    assert_eq!(ledger.records()[0].generation, first.work.generation);
    let second = claim(&mut ledger, 12);
    assert_eq!(revision(&ledger), 3);
    ledger
        .apply(WorkUpdate::Finish {
            claim: second.key(),
            now: 13,
            outcome: WorkOutcome::Done,
        })
        .unwrap();
    assert_eq!(revision(&ledger), 4);
    let before = ledger.clone();
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 14 }).unwrap(),
        WorkResult::Idle
    );
    assert_eq!(ledger, before);
}
#[test]
fn delivery_updates_increment_once_and_preserve_claim_generation() {
    let mut ledger = ledger(notification());
    let first = claim(&mut ledger, 10);
    ledger
        .apply(WorkUpdate::DeliveryStarted {
            claim: first.key(),
            now: 11,
        })
        .unwrap();
    assert_eq!(revision(&ledger), 2);
    assert_eq!(ledger.records()[0].generation, first.work.generation);
    let unchanged = ledger.clone();
    ledger
        .apply(WorkUpdate::DeliveryStarted {
            claim: first.key(),
            now: 11,
        })
        .unwrap();
    assert_eq!(ledger, unchanged);
    ledger
        .apply(WorkUpdate::DeliveryFinished {
            claim: first.key(),
            now: 12,
            outcome: DeliveryOutcome::Accepted,
        })
        .unwrap();
    assert_eq!(revision(&ledger), 3);
    assert_eq!(ledger.records()[0].generation, first.work.generation);
}
#[test]
fn materialization_versions_parent_and_initializes_children() {
    let mut ledger = ledger(WorkPayload::Source(match notification() {
        WorkPayload::Notification { source, .. } => source,
        _ => unreachable!(),
    }));
    let first = claim(&mut ledger, 10);
    ledger
        .apply(WorkUpdate::Materialize {
            claim: first.key(),
            now: 11,
            children: vec![pending("child", WorkPayload::Action(json!({})))],
        })
        .unwrap();
    assert_eq!(
        serde_json::to_value(&ledger).unwrap()["work"]["work"]["revision"],
        2
    );
    assert_eq!(
        serde_json::to_value(&ledger).unwrap()["work"]["child"]["revision"],
        0
    );
}
#[test]
fn rejected_updates_preserve_revision_and_all_budgets() {
    let mut ledger = ledger(WorkPayload::Action(json!({})));
    let first = claim(&mut ledger, 10);
    let before = ledger.clone();
    assert!(matches!(
        ledger.apply(WorkUpdate::Finish {
            claim: ClaimKey {
                id: "work".into(),
                generation: first.work.generation + 1
            },
            now: 11,
            outcome: WorkOutcome::Done
        }),
        Err(Error::Conflict)
    ));
    assert_eq!(ledger, before);
    assert_eq!(revision(&ledger), 1);
}
#[test]
fn revision_overflow_rejects_claim_without_consuming_budget() {
    let ledger = ledger(WorkPayload::Action(json!({})));
    let mut value = serde_json::to_value(ledger).unwrap();
    value["work"]["work"]["revision"] = json!(u64::MAX);
    let mut ledger: WorkLedger = serde_json::from_value(value).unwrap();
    let before = ledger.clone();
    assert!(matches!(
        ledger.apply(WorkUpdate::Claim { now: 10 }),
        Err(Error::TooLarge)
    ));
    assert_eq!(ledger, before);
}
#[test]
fn restore_invalidates_revision_and_overflow_is_atomic() {
    let mut ledger = ledger(notification());
    claim(&mut ledger, 10);
    ledger.prepare_restore().unwrap();
    assert_eq!(revision(&ledger), 2);
    let first = claim(&mut ledger, 11);
    let mut value = serde_json::to_value(&ledger).unwrap();
    value["work"]["work"]["revision"] = json!(u64::MAX);
    let mut ledger: WorkLedger = serde_json::from_value(value).unwrap();
    let before = ledger.clone();
    assert!(matches!(ledger.prepare_restore(), Err(Error::TooLarge)));
    assert_eq!(ledger, before);
    assert_eq!(ledger.records()[0].generation, first.work.generation);
}

#[test]
fn bounded_snapshot_versions_invalidate_delivery_views() {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("work", notification())],
        )
        .unwrap();
    let first = claim(&mut state.work, 10);
    let before = state.work_snapshot(1, 4096).unwrap();
    state
        .update_work(WorkUpdate::DeliveryStarted {
            claim: first.key(),
            now: 11,
        })
        .unwrap();
    let after = state.work_snapshot(1, 4096).unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(before.records[0].generation, after.records[0].generation);
    assert_ne!(
        before.version(&before.records[0]),
        after.version(&after.records[0])
    );
    assert!(matches!(state.work_snapshot(0, 4096), Err(Error::TooLarge)));
    let bytes = serde_json::to_vec(&after).unwrap().len();
    assert!(matches!(
        state.work_snapshot(1, bytes - 1),
        Err(Error::TooLarge)
    ));
    state.work_snapshot(1, bytes).unwrap();
    state.prepare_restore().unwrap();
    assert_ne!(
        state.work_snapshot(1, 4096).unwrap().generation,
        before.generation
    );
}
#[test]
fn bounded_snapshot_rejects_colliding_opaque_handles() {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("work", notification())],
        )
        .unwrap();
    let mut wire = serde_json::to_value(&state.work).unwrap();
    wire["work"]["other"] = wire["work"]["work"].clone();
    state.work = serde_json::from_value(wire).unwrap();
    assert!(matches!(state.work_snapshot(2, 4096), Err(Error::Storage)));
}
#[test]
fn finish_preserves_attempt_and_root_budget_precedence_when_age_also_expires() {
    for (attempts, max_work, expected) in [
        (3, 256, StopReason::Attempts),
        (1, 1, StopReason::WorkBudget),
    ] {
        let ledger = ledger(notification());
        let mut wire = serde_json::to_value(ledger).unwrap();
        wire["limits"]["max_work"] = json!(max_work);
        wire["work"]["work"]["attempts"] = json!(attempts);
        wire["work"]["work"]["generation"] = json!(attempts);
        wire["work"]["work"]["state"] =
            json!({"Leased":{"until":5000,"generation":attempts,"resolution_only":null}});
        wire["roots"]["root"] = json!(attempts);
        let mut ledger: WorkLedger = serde_json::from_value(wire).unwrap();
        ledger
            .apply(WorkUpdate::Finish {
                claim: ClaimKey {
                    id: "work".into(),
                    generation: attempts,
                },
                now: 4000,
                outcome: WorkOutcome::Retry,
            })
            .unwrap();
        assert_eq!(ledger.records()[0].state, WorkState::Stopped(expected));
    }
}
