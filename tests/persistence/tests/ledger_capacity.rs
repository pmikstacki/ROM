use rom::*;
fn pending(notification: bool) -> PendingWork {
    let row = Row {
        key: Key {
            kind: "tasks".into(),
            id: "1".into(),
        },
        revision: 1,
        value: Some(json!({"value":true})),
        protected: Default::default(),
    };
    PendingWork {
        id: "step".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "root".into(),
            parent: None,
            depth: 1,
            started_at: 0,
            path: vec![],
        },
        definition: "reaction".into(),
        version: 1,
        service_key: "service".into(),
        delivery_profile: rom::DeliveryProfile::AtLeastOnce,
        payload: if notification {
            WorkPayload::Notification {
                source: row,
                payload: json!({"body":"message"}),
            }
        } else {
            WorkPayload::Source(row)
        },
    }
}
fn minimum(notification: bool) -> (WorkLedger, ReactionLimits) {
    for max_bytes in 1..4096 {
        let mut ledger = WorkLedger::default();
        let limits = ReactionLimits {
            max_bytes,
            ..Default::default()
        };
        if ledger.enqueue(&limits, vec![pending(notification)]).is_ok() {
            return (ledger, limits);
        }
    }
    panic!("no fitting bound")
}
fn claim(ledger: &mut WorkLedger, now: u64) -> WorkClaim {
    let result = ledger.apply(WorkUpdate::Claim { now });
    assert!(
        matches!(result, Ok(WorkResult::Claimed(_))),
        "accepted work must progress: {result:?}"
    );
    let WorkResult::Claimed(c) = result.unwrap() else {
        unreachable!()
    };
    *c
}
fn check_bytes(ledger: &WorkLedger, limits: &ReactionLimits) {
    assert!(serde_json::to_vec(ledger).unwrap().len() <= limits.max_bytes);
}
#[test]
fn accepted_work_can_claim_retry_and_stop_at_the_smallest_byte_budget() {
    let (mut ledger, limits) = minimum(false);
    for now in [1, 9, 99] {
        let c = claim(&mut ledger, now);
        check_bytes(&ledger, &limits);
        ledger
            .apply(WorkUpdate::Finish {
                claim: c.key(),
                now,
                outcome: WorkOutcome::Retry,
            })
            .unwrap();
        check_bytes(&ledger, &limits);
    }
    assert_eq!(
        ledger.records()[0].state,
        WorkState::Stopped(StopReason::Attempts)
    );
}
#[test]
fn every_delivery_outcome_and_stop_reason_fit_reserved_capacity() {
    for outcome in [
        DeliveryOutcome::Accepted,
        DeliveryOutcome::Retryable,
        DeliveryOutcome::Permanent,
        DeliveryOutcome::Unknown,
        DeliveryOutcome::TimedOut,
        DeliveryOutcome::Panicked,
    ] {
        let (mut ledger, limits) = minimum(true);
        let c = claim(&mut ledger, 9);
        ledger
            .apply(WorkUpdate::DeliveryStarted {
                claim: c.key(),
                now: 9,
            })
            .unwrap();
        check_bytes(&ledger, &limits);
        ledger
            .apply(WorkUpdate::DeliveryFinished {
                claim: c.key(),
                now: 9,
                outcome,
            })
            .unwrap();
        check_bytes(&ledger, &limits);
    }
    for reason in [
        StopReason::Depth,
        StopReason::WorkBudget,
        StopReason::Attempts,
        StopReason::Age,
        StopReason::Fanout,
        StopReason::Denied,
        StopReason::Conflict,
        StopReason::Invalid,
        StopReason::Missing,
        StopReason::DefinitionChanged,
        StopReason::CallbackPanicked,
        StopReason::Unavailable,
        StopReason::DeliveryPermanent,
    ] {
        let (mut ledger, limits) = minimum(false);
        let c = claim(&mut ledger, 9);
        ledger
            .apply(WorkUpdate::Finish {
                claim: c.key(),
                now: 9,
                outcome: WorkOutcome::Stop(reason),
            })
            .unwrap();
        check_bytes(&ledger, &limits);
    }
}
#[test]
fn rejected_materialization_leaves_capacity_to_finish_parent() {
    let (mut ledger, limits) = minimum(false);
    let c = claim(&mut ledger, 1);
    let before = ledger.clone();
    let mut child = pending(false);
    child.id = "child".into();
    child.payload = WorkPayload::Action(
        json!({"kind":"notes","id":"one","expected":null,"idempotency":"frozen","operation":{"type":"create","input":{"input":"frozen"}}}),
    );
    assert_eq!(
        ledger.apply(WorkUpdate::Materialize {
            claim: c.key(),
            now: 1,
            children: vec![child]
        }),
        Err(Error::Overloaded)
    );
    assert_eq!(ledger, before);
    ledger
        .apply(WorkUpdate::Finish {
            claim: c.key(),
            now: 1,
            outcome: WorkOutcome::Stop(StopReason::Unavailable),
        })
        .unwrap();
    check_bytes(&ledger, &limits);
}
#[test]
fn restored_generation_growth_and_exhaustion_fit_at_capacity() {
    let (mut ledger, limits) = minimum(true);
    let c = claim(&mut ledger, 0);
    ledger
        .apply(WorkUpdate::DeliveryStarted {
            claim: c.key(),
            now: 0,
        })
        .unwrap();
    // Model an older valid ledger after many retries/restarts. Attempt/root budgets
    // stay authoritative; generation is an independent fence and may be much larger.
    let mut value = serde_json::to_value(&ledger).unwrap();
    value["work"]["step"]["generation"] = json!(9_999_999_999_999_999_999u64);
    value["work"]["step"]["state"]["Leased"]["generation"] = json!(9_999_999_999_999_999_999u64);
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state.work = serde_json::from_value(value).unwrap();
    state.validate_archive(0, 0, &[]).unwrap();
    state.prepare_restore().unwrap();
    assert_eq!(
        state.work.records()[0].generation,
        10_000_000_000_000_000_000
    );
    check_bytes(&state.work, &limits);
    let c = claim(&mut state.work, 1);
    assert_eq!(c.work.delivery, Some(DeliveryOutcome::Unknown));
    state
        .work
        .apply(WorkUpdate::DeliveryFinished {
            claim: c.key(),
            now: 1,
            outcome: DeliveryOutcome::Accepted,
        })
        .unwrap();
    check_bytes(&state.work, &limits);
}
#[test]
fn legacy_insufficient_headroom_is_explicitly_unsupported_without_mutation() {
    let (ledger, _) = minimum(false);
    let value = serde_json::to_value(&ledger).unwrap();
    // The old admission rule counted only the current encoding, without reserving
    // lifecycle growth. Find that boundary independently of metadata field widths.
    let mut legacy: WorkLedger = (1..4096)
        .find_map(|max_bytes| {
            let mut candidate = value.clone();
            candidate["limits"]["max_bytes"] = json!(max_bytes);
            let ledger: WorkLedger = serde_json::from_value(candidate).unwrap();
            (serde_json::to_vec(&ledger).unwrap().len() == max_bytes).then_some(ledger)
        })
        .unwrap();
    let before = legacy.clone();
    assert!(matches!(
        legacy.apply(WorkUpdate::Claim { now: 1 }),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(legacy, before);
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state.work = legacy;
    assert!(matches!(
        state.check_limits(&StorageLimits::default()),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        state.validate_archive(0, 0, &[]),
        Err(Error::Unsupported(_))
    ));
    let before = serde_json::to_value(&state).unwrap();
    assert!(matches!(
        state.prepare_restore(),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(serde_json::to_value(&state).unwrap(), before);
}

#[test]
fn admitted_materialized_children_keep_their_lifecycle_reservation() {
    let parent = pending(false);
    let mut child = parent.clone();
    child.id = "z-child".into();
    child.payload = WorkPayload::Action(
        json!({"kind":"notes","id":"one","expected":null,"idempotency":"frozen","operation":{"type":"create","input":{"frozen":true}}}),
    );
    let limits = (1..4096)
        .find_map(|max_bytes| {
            let limits = ReactionLimits {
                max_bytes,
                ..Default::default()
            };
            WorkLedger::default()
                .enqueue(&limits, vec![parent.clone(), child.clone()])
                .is_ok()
                .then_some(limits)
        })
        .unwrap();
    let mut ledger = WorkLedger::default();
    ledger.enqueue(&limits, vec![parent]).unwrap();
    let c = claim(&mut ledger, 1);
    ledger
        .apply(WorkUpdate::Materialize {
            claim: c.key(),
            now: 1,
            children: vec![child],
        })
        .unwrap();
    check_bytes(&ledger, &limits);
    let c = claim(&mut ledger, 9);
    assert_eq!(c.work.pending.id, "z-child");
    ledger
        .apply(WorkUpdate::Finish {
            claim: c.key(),
            now: 9,
            outcome: WorkOutcome::Retry,
        })
        .unwrap();
    let c = claim(&mut ledger, 99);
    ledger
        .apply(WorkUpdate::Finish {
            claim: c.key(),
            now: 99,
            outcome: WorkOutcome::Done,
        })
        .unwrap();
    check_bytes(&ledger, &limits);
}
#[test]
fn resolution_lease_maximum_timestamp_width_fits_reservation() {
    let (mut ledger, limits) = minimum(false);
    let now = u64::MAX - 64;
    let c = claim(&mut ledger, now);
    assert!(c.resolution_only);
    check_bytes(&ledger, &limits);
    ledger
        .apply(WorkUpdate::Finish {
            claim: c.key(),
            now,
            outcome: WorkOutcome::Stop(StopReason::DefinitionChanged),
        })
        .unwrap();
    check_bytes(&ledger, &limits);
}
