//! Frozen scheduling floors under the shared persisted lifecycle protocol.
use super::*;
use crate::operator::{WorkControlOperation, WorkControlRequest, WorkHandle, WorkVersion};

fn pending(not_before: u64) -> PendingWork {
    serde_json::from_value(json!({
        "id":"delayed", "cause":{"root":"root", "parent":null, "depth":0,
        "started_at":10, "path":[], "retry_epoch":0}, "definition":"mail",
        "version":1, "service_key":"service", "delivery_profile":"AtLeastOnce",
        "not_before":not_before, "payload":{"Notification":{"source":{
            "key":{"kind":"notes","id":"1"},"revision":1,"value":{},
            "protected":{}},"payload":{"message":"frozen"}}}
    }))
    .unwrap()
}
fn ledger(not_before: u64) -> WorkLedger {
    let mut ledger = WorkLedger::default();
    ledger
        .enqueue(&ReactionLimits::default(), vec![pending(not_before)])
        .unwrap();
    ledger
}
fn claim(ledger: &mut WorkLedger, now: u64) -> WorkClaim {
    match ledger.apply(WorkUpdate::Claim { now }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim: {other:?}"),
    }
}
#[test]
fn frozen_floor_prevents_early_claim_without_consuming_budgets() {
    let mut ledger = ledger(20);
    let before = ledger.clone();
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 19 }).unwrap(),
        WorkResult::Idle
    );
    assert_eq!(ledger, before);
    let claimed = claim(&mut ledger, 20);
    assert_eq!(claimed.work.due, 20);
    assert_eq!(claimed.work.attempts, 1);
    assert!(!claimed.resolution_only);
}
#[test]
fn timing_is_part_of_frozen_identity_and_exact_duplicate_is_unchanged() {
    let mut ledger = ledger(20);
    let before = ledger.clone();
    ledger
        .enqueue(&ReactionLimits::default(), vec![pending(20)])
        .unwrap();
    assert_eq!(ledger, before);
    assert_eq!(
        ledger.enqueue(&ReactionLimits::default(), vec![pending(21)]),
        Err(Error::IdentityMismatch)
    );
    assert_eq!(ledger, before);
}
#[test]
fn restore_fences_claims_without_erasing_floor_or_attempts() {
    let mut ledger = ledger(20);
    let old = claim(&mut ledger, 20);
    ledger.prepare_restore().unwrap();
    assert_eq!(ledger.records()[0].due, 20);
    assert_eq!(ledger.records()[0].attempts, 1);
    assert!(ledger.records()[0].generation > old.work.generation);
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 19 }).unwrap(),
        WorkResult::Idle
    );
    assert_eq!(
        ledger.apply(WorkUpdate::DeliveryStarted {
            claim: old.key(),
            now: 21
        }),
        Err(Error::Conflict)
    );
}
#[test]
fn operator_retry_cannot_move_due_below_floor() {
    let mut ledger = ledger(20);
    ledger.work.get_mut("delayed").unwrap().state = WorkState::Stopped(StopReason::Denied);
    let control = StorageWorkControl {
        principal: "operator".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("delayed"),
            expected: WorkVersion {
                generation: "fixture".into(),
                revision: 0,
            },
            key: "retry".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        },
        decision: WorkControlDecision::Retry,
        now: 11,
    };
    ledger.control("delayed", &control).unwrap();
    assert_eq!(ledger.records()[0].due, 20);
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 19 }).unwrap(),
        WorkResult::Idle
    );
}
#[test]
fn backwards_clock_cannot_start_or_finish_delivery_before_floor() {
    let mut ledger = ledger(20);
    let claimed = claim(&mut ledger, 20);
    let before = ledger.clone();
    assert_eq!(
        ledger.apply(WorkUpdate::DeliveryStarted {
            claim: claimed.key(),
            now: 19
        }),
        Err(Error::Conflict)
    );
    assert_eq!(
        ledger.apply(WorkUpdate::DeliveryFinished {
            claim: claimed.key(),
            now: 19,
            outcome: DeliveryOutcome::Accepted
        }),
        Err(Error::Conflict)
    );
    assert_eq!(ledger, before);
}
#[test]
fn age_deadline_before_floor_stops_without_claim_or_attempt() {
    let mut ledger = ledger(u64::MAX);
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 3610 }).unwrap(),
        WorkResult::Changed
    );
    let record = &ledger.records()[0];
    assert_eq!(record.state, WorkState::Stopped(StopReason::Age));
    assert_eq!(record.attempts, 0);
    assert_eq!(record.generation, 0);
    assert_eq!(record.delivery, None);
    assert_eq!(ledger.root_usage()["root"], 0);
}
#[test]
fn archive_rejects_due_below_frozen_floor() {
    let mut ledger = ledger(20);
    ledger.work.get_mut("delayed").unwrap().due = 19;
    assert_eq!(ledger.validate_archive(), Err(Error::Storage));
}
#[test]
fn retry_respects_floor_and_overflow_does_not_publish_candidate() {
    let mut ledger = ledger(20);
    let claimed = claim(&mut ledger, 20);
    ledger
        .apply(WorkUpdate::Finish {
            claim: claimed.key(),
            now: 20,
            outcome: WorkOutcome::Retry,
        })
        .unwrap();
    assert_eq!(ledger.records()[0].due, 21);
    let claimed = claim(&mut ledger, 21);
    let before = ledger.clone();
    // A maximal lease expiry cannot be represented; no partial update is published.
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: u64::MAX }),
        Err(Error::TooLarge)
    );
    assert_eq!(ledger, before);
    assert_eq!(
        ledger
            .apply(WorkUpdate::Finish {
                claim: claimed.key(),
                now: 22,
                outcome: WorkOutcome::Done
            })
            .unwrap(),
        WorkResult::Changed
    );
}
