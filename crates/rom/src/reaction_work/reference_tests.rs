//! Independent old-model comparison for the incremental replacement.
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
        not_before: None,
        delivery_profile: DeliveryProfile::AtLeastOnce,
        service_key: "service".into(),
        payload,
    }
}
fn row() -> Row {
    Row {
        key: Key {
            kind: "test".into(),
            id: "1".into(),
        },
        revision: 1,
        value: Some(json!({})),
        protected: ProtectedMetadata::default(),
    }
}
fn same(current: &WorkLedger, old: &reference::WorkLedger) {
    assert_eq!(
        serde_json::to_value(current).unwrap(),
        serde_json::to_value(old).unwrap()
    );
}
fn step(
    current: &mut WorkLedger,
    old: &mut reference::WorkLedger,
    update: WorkUpdate,
) -> Result<WorkResult> {
    let actual = current.apply(update.clone());
    assert_eq!(actual, old.apply(update));
    same(current, old);
    actual
}
fn claim(current: &mut WorkLedger, old: &mut reference::WorkLedger, now: u64) -> WorkClaim {
    match step(current, old, WorkUpdate::Claim { now }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim: {other:?}"),
    }
}
#[test]
fn frozen_oracle_matches_all_five_updates_and_duplicate_enqueue() {
    for payload in [
        WorkPayload::Action(json!({})),
        WorkPayload::Source(row()),
        WorkPayload::Notification {
            source: row(),
            payload: json!({}),
        },
    ] {
        let mut current = WorkLedger::default();
        let mut old = reference::WorkLedger::default();
        let work = pending("work", payload.clone());
        for _ in 0..2 {
            assert_eq!(
                current.enqueue(&ReactionLimits::default(), vec![work.clone()]),
                old.enqueue(&ReactionLimits::default(), vec![work.clone()])
            );
            same(&current, &old);
        }
        let first = claim(&mut current, &mut old, 10);
        assert!(
            step(
                &mut current,
                &mut old,
                WorkUpdate::Finish {
                    claim: ClaimKey {
                        id: "work".into(),
                        generation: first.work.generation + 1
                    },
                    now: 11,
                    outcome: WorkOutcome::Done
                }
            )
            .is_err()
        );
        match payload {
            WorkPayload::Source(_) => {
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::Materialize {
                        claim: first.key(),
                        now: 11,
                        children: vec![pending("child", WorkPayload::Action(json!({})))],
                    },
                )
                .unwrap();
                let child = claim(&mut current, &mut old, 11);
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::Finish {
                        claim: child.key(),
                        now: 12,
                        outcome: WorkOutcome::Done,
                    },
                )
                .unwrap();
            }
            WorkPayload::Notification { .. } => {
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::DeliveryStarted {
                        claim: first.key(),
                        now: 11,
                    },
                )
                .unwrap();
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::DeliveryFinished {
                        claim: first.key(),
                        now: 12,
                        outcome: DeliveryOutcome::Accepted,
                    },
                )
                .unwrap();
            }
            WorkPayload::Action(_) => {
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::Finish {
                        claim: first.key(),
                        now: 11,
                        outcome: WorkOutcome::Retry,
                    },
                )
                .unwrap();
                let second = claim(&mut current, &mut old, 12);
                step(
                    &mut current,
                    &mut old,
                    WorkUpdate::Finish {
                        claim: second.key(),
                        now: 13,
                        outcome: WorkOutcome::Done,
                    },
                )
                .unwrap();
            }
        }
        step(&mut current, &mut old, WorkUpdate::Claim { now: 14 }).unwrap();
    }
}

#[test]
fn frozen_oracle_preserves_atomic_id_ordered_prefix_and_late_overflow() {
    for overflow in [false, true] {
        let mut current = WorkLedger::default();
        let limits = ReactionLimits {
            max_age_seconds: 2,
            ..ReactionLimits::default()
        };
        let mut delayed = pending("a-delayed", WorkPayload::Action(json!({})));
        delayed.not_before = Some(100);
        current
            .enqueue(
                &limits,
                vec![
                    delayed,
                    pending("z-eligible", WorkPayload::Action(json!({}))),
                ],
            )
            .unwrap();
        if overflow {
            let mut value = serde_json::to_value(&current).unwrap();
            value["work"]["z-eligible"]["generation"] = json!(u64::MAX);
            current = serde_json::from_value(value).unwrap();
        }
        let mut old = serde_json::from_value(serde_json::to_value(&current).unwrap()).unwrap();
        let before = current.clone();
        let result = step(&mut current, &mut old, WorkUpdate::Claim { now: 12 });
        if overflow {
            assert_eq!(result, Err(Error::TooLarge));
            assert_eq!(current, before);
        } else {
            assert!(matches!(result, Ok(WorkResult::Claimed(_))));
            assert_eq!(
                current.records()[0].state,
                WorkState::Stopped(StopReason::Age)
            );
            assert_eq!(
                serde_json::to_value(&current).unwrap()["work"]["a-delayed"]["revision"],
                1
            );
        }
    }
}

#[test]
fn frozen_oracle_rejects_orphan_root_but_retains_zero_attempt_members() {
    let mut current = WorkLedger::default();
    current
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("work", WorkPayload::Source(row()))],
        )
        .unwrap();
    let old: reference::WorkLedger =
        serde_json::from_value(serde_json::to_value(&current).unwrap()).unwrap();
    assert_eq!(current.validate_archive(), Ok(()));
    assert_eq!(old.validate_archive(), Ok(()));
    let mut wire = serde_json::to_value(&current).unwrap();
    wire["work"] = json!({});
    let current: WorkLedger = serde_json::from_value(wire.clone()).unwrap();
    let old: reference::WorkLedger = serde_json::from_value(wire).unwrap();
    assert_eq!(current.validate_archive(), Err(Error::Storage));
    assert_eq!(old.validate_archive(), Err(Error::Storage));
}
