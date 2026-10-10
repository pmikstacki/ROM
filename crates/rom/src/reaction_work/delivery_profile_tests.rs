//! Pure persisted delivery lifecycle scenarios; provider calls are tested separately.
use super::*;
use crate::operator::{
    WorkControlOperation, WorkControlOutcome, WorkControlRequest, WorkHandle, WorkVersion,
};

fn pending(id: &str, profile: DeliveryProfile) -> PendingWork {
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
        definition: "channel".into(),
        version: 1,
        service_key: "service".into(),
        not_before: None,
        delivery_profile: profile,
        payload: WorkPayload::Notification {
            source: Row {
                key: Key {
                    kind: "test".into(),
                    id: "1".into(),
                },
                revision: 1,
                value: Some(json!({})),
                protected: ProtectedMetadata::default(),
            },
            payload: json!({"message":"frozen"}),
        },
    }
}
fn ledger(profile: DeliveryProfile) -> WorkLedger {
    ledger_with_limits(profile, ReactionLimits::default())
}
fn ledger_with_limits(profile: DeliveryProfile, limits: ReactionLimits) -> WorkLedger {
    let mut ledger = WorkLedger::default();
    ledger
        .enqueue(&limits, vec![pending("a", profile)])
        .unwrap();
    ledger
}
fn claim(ledger: &mut WorkLedger, now: u64) -> WorkClaim {
    match ledger.apply(WorkUpdate::Claim { now }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim: {other:?}"),
    }
}
fn started(ledger: &mut WorkLedger) -> WorkClaim {
    let claim = claim(ledger, 10);
    ledger
        .apply(WorkUpdate::DeliveryStarted {
            claim: claim.key(),
            now: 11,
        })
        .unwrap();
    claim
}
fn finish(
    ledger: &mut WorkLedger,
    claim: &WorkClaim,
    outcome: DeliveryOutcome,
) -> Result<WorkResult> {
    ledger.apply(WorkUpdate::DeliveryFinished {
        claim: claim.key(),
        now: 12,
        outcome,
    })
}
fn control(ledger: &WorkLedger, decision: WorkControlDecision) -> StorageWorkControl {
    StorageWorkControl {
        principal: "operator".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("a"),
            expected: WorkVersion {
                generation: "generation".into(),
                revision: ledger.work["a"].revision,
            },
            key: "control".into(),
            retry_epoch: 0,
            operation: if decision == WorkControlDecision::Retry {
                WorkControlOperation::Retry
            } else {
                WorkControlOperation::Reconcile { evidence_ref: None }
            },
        },
        decision,
        now: 12,
    }
}

#[test]
fn retry_profiles_keep_frozen_identity_and_default_unknown_retry() {
    for profile in [
        DeliveryProfile::AtLeastOnce,
        DeliveryProfile::ProviderDeduplicated,
    ] {
        let mut ledger = ledger(profile);
        let first = started(&mut ledger);
        finish(&mut ledger, &first, DeliveryOutcome::Unknown).unwrap();
        assert_eq!(ledger.work["a"].state, WorkState::Pending);
        assert_eq!(ledger.work["a"].due, 13);
        assert_eq!(ledger.work["a"].generation, 1);
        let second = claim(&mut ledger, 13);
        assert_eq!(second.work.pending, first.work.pending);
        assert_eq!(second.work.attempts, 2);
    }
}

fn assert_uncertain_hold(outcome: DeliveryOutcome) {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    let first = started(&mut ledger);
    let frozen = ledger.work["a"].pending.clone();
    finish(&mut ledger, &first, outcome.clone()).unwrap();
    let held = &ledger.work["a"];
    assert_eq!(held.state, WorkState::AwaitingReconciliation);
    assert_eq!(held.pending, frozen);
    assert_eq!(held.delivery, Some(outcome));
    assert_eq!((held.attempts, held.generation, held.revision), (1, 2, 3));
    assert_eq!(ledger.roots["root"], 1);
    let before = ledger.clone();
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 4000 }).unwrap(),
        WorkResult::Idle
    );
    assert!(matches!(
        finish(&mut ledger, &first, DeliveryOutcome::Accepted),
        Err(Error::Conflict)
    ));
    assert_eq!(ledger, before);
}
#[test]
fn unknown_hold_profile_fences_without_consuming_retry_budgets() {
    assert_uncertain_hold(DeliveryOutcome::Unknown);
}
#[test]
fn timed_out_hold_profile_fences_without_consuming_retry_budgets() {
    assert_uncertain_hold(DeliveryOutcome::TimedOut);
}
#[test]
fn panicked_hold_profile_fences_without_consuming_retry_budgets() {
    assert_uncertain_hold(DeliveryOutcome::Panicked);
}

#[test]
fn expired_started_delivery_holds_while_active_and_unstarted_leases_are_not_held() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    started(&mut ledger);
    let active = ledger.clone();
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 39 }).unwrap(),
        WorkResult::Idle
    );
    assert_eq!(ledger, active);
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 40 }).unwrap(),
        WorkResult::Changed
    );
    assert_eq!(ledger.work["a"].state, WorkState::AwaitingReconciliation);
    assert_eq!(
        (
            ledger.work["a"].attempts,
            ledger.work["a"].generation,
            ledger.work["a"].revision
        ),
        (1, 2, 3)
    );
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 40 }).unwrap(),
        WorkResult::Idle
    );
    let mut fresh = self::ledger(DeliveryProfile::ReconcileBeforeRetry);
    let first = claim(&mut fresh, 10);
    let second = claim(&mut fresh, 40);
    assert_eq!(second.work.pending, first.work.pending);
    assert_eq!(
        (
            second.work.attempts,
            second.work.generation,
            second.work.revision
        ),
        (2, 2, 2)
    );
}

#[test]
fn expired_hold_conversion_does_not_skip_other_ready_work() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    started(&mut ledger);
    ledger
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("b", DeliveryProfile::AtLeastOnce)],
        )
        .unwrap();
    let other = claim(&mut ledger, 40);
    assert_eq!(other.work.pending.id, "b");
    assert_eq!(ledger.work["a"].state, WorkState::AwaitingReconciliation);
    assert_eq!(ledger.work["a"].attempts, 1);
    assert_eq!(ledger.work["a"].revision, 3);
    assert_eq!(other.work.revision, 1);
    assert_eq!(ledger.roots["root"], 2);
}

#[test]
fn restore_holds_only_unresolved_started_profile_and_fences_each_lease_once() {
    for (profile, start, expected) in [
        (
            DeliveryProfile::ReconcileBeforeRetry,
            true,
            WorkState::AwaitingReconciliation,
        ),
        (
            DeliveryProfile::ReconcileBeforeRetry,
            false,
            WorkState::Pending,
        ),
        (DeliveryProfile::AtLeastOnce, true, WorkState::Pending),
        (
            DeliveryProfile::ProviderDeduplicated,
            true,
            WorkState::Pending,
        ),
    ] {
        let mut ledger = ledger(profile);
        let first = if start {
            started(&mut ledger)
        } else {
            claim(&mut ledger, 10)
        };
        let before = ledger.work["a"].clone();
        ledger.prepare_restore().unwrap();
        let restored = &ledger.work["a"];
        assert_eq!(restored.state, expected);
        assert_eq!(restored.pending, before.pending);
        assert_eq!(restored.delivery, before.delivery);
        assert_eq!(restored.attempts, 1);
        assert_eq!(restored.generation, 2);
        assert_eq!(restored.revision, before.revision + 1);
        assert_eq!(ledger.roots["root"], 1);
        assert!(matches!(
            finish(&mut ledger, &first, DeliveryOutcome::Accepted),
            Err(Error::Conflict)
        ));
        if expected == WorkState::AwaitingReconciliation {
            let held = ledger.clone();
            ledger.prepare_restore().unwrap();
            assert_eq!(ledger, held);
            assert_eq!(
                ledger.apply(WorkUpdate::Claim { now: 40 }).unwrap(),
                WorkResult::Idle
            );
        } else {
            assert_eq!(claim(&mut ledger, 12).work.attempts, 2);
        }
    }
}

#[test]
fn confirmed_delivery_outcomes_keep_existing_terminal_and_retry_semantics() {
    for (profile, outcome, expected) in [
        (
            DeliveryProfile::ReconcileBeforeRetry,
            DeliveryOutcome::Accepted,
            WorkState::Done,
        ),
        (
            DeliveryProfile::ReconcileBeforeRetry,
            DeliveryOutcome::Permanent,
            WorkState::Stopped(StopReason::DeliveryPermanent),
        ),
        (
            DeliveryProfile::ReconcileBeforeRetry,
            DeliveryOutcome::Retryable,
            WorkState::Pending,
        ),
        (
            DeliveryProfile::AtLeastOnce,
            DeliveryOutcome::Panicked,
            WorkState::Stopped(StopReason::CallbackPanicked),
        ),
    ] {
        let mut ledger = ledger(profile);
        let first = started(&mut ledger);
        finish(&mut ledger, &first, outcome.clone()).unwrap();
        assert_eq!(ledger.work["a"].state, expected);
        assert_eq!(ledger.work["a"].delivery, Some(outcome));
        assert_eq!(
            (
                ledger.work["a"].attempts,
                ledger.work["a"].generation,
                ledger.work["a"].revision
            ),
            (1, 1, 3)
        );
    }
}

#[test]
fn generic_finish_retry_cannot_make_started_unknown_delivery_resendable() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    let first = started(&mut ledger);
    ledger
        .apply(WorkUpdate::Finish {
            claim: first.key(),
            now: 12,
            outcome: WorkOutcome::Retry,
        })
        .unwrap();
    assert_eq!(ledger.work["a"].state, WorkState::AwaitingReconciliation);
    assert_eq!(ledger.work["a"].generation, 2);
}

#[test]
fn plain_retry_cannot_bypass_unresolved_evidence_after_a_lifecycle_stop() {
    for outcome in [
        DeliveryOutcome::Unknown,
        DeliveryOutcome::TimedOut,
        DeliveryOutcome::Panicked,
    ] {
        let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
        let first = started(&mut ledger);
        ledger
            .apply(WorkUpdate::Finish {
                claim: first.key(),
                now: 12,
                outcome: WorkOutcome::Stop(StopReason::Denied),
            })
            .unwrap();
        ledger.work.get_mut("a").unwrap().delivery = Some(outcome);
        let request = control(&ledger, WorkControlDecision::Retry);
        let before = ledger.clone();
        assert!(matches!(
            ledger.control("a", &request),
            Err(Error::Conflict)
        ));
        assert_eq!(ledger, before);
    }
}

#[test]
fn verified_not_accepted_releases_hold_under_original_budgets() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    let first = started(&mut ledger);
    finish(&mut ledger, &first, DeliveryOutcome::Unknown).unwrap();
    assert_eq!(ledger.work["a"].state, WorkState::AwaitingReconciliation);
    let request = control(
        &ledger,
        WorkControlDecision::DeliveryNotAccepted {
            evidence: "verified".into(),
        },
    );
    assert_eq!(
        ledger.control("a", &request).unwrap(),
        WorkControlOutcome::Scheduled
    );
    assert_eq!(ledger.work["a"].delivery, Some(DeliveryOutcome::Permanent));
    assert_eq!(ledger.work["a"].state, WorkState::Pending);
    assert_eq!(ledger.roots["root"], 1);
    assert_eq!(claim(&mut ledger, 12).work.attempts, 2);
}

#[test]
fn verified_accepted_completes_hold_without_retrying_or_consuming_budget() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    let first = started(&mut ledger);
    finish(&mut ledger, &first, DeliveryOutcome::TimedOut).unwrap();
    let request = control(
        &ledger,
        WorkControlDecision::DeliveryAccepted {
            evidence: "verified".into(),
        },
    );
    assert_eq!(
        ledger.control("a", &request).unwrap(),
        WorkControlOutcome::Completed
    );
    assert_eq!(ledger.work["a"].state, WorkState::Done);
    assert_eq!(ledger.work["a"].delivery, Some(DeliveryOutcome::Accepted));
    assert_eq!(
        (
            ledger.work["a"].attempts,
            ledger.work["a"].generation,
            ledger.work["a"].revision
        ),
        (1, 3, 4)
    );
    assert_eq!(ledger.roots["root"], 1);
    assert_eq!(
        ledger.apply(WorkUpdate::Claim { now: 40 }).unwrap(),
        WorkResult::Idle
    );
}

#[test]
fn uncertainty_holds_at_exhausted_budgets_and_verified_rejection_retains_stop_precedence() {
    for (limits, expected) in [
        (
            ReactionLimits {
                max_attempts: 1,
                ..ReactionLimits::default()
            },
            StopReason::Attempts,
        ),
        (
            ReactionLimits {
                max_work: 1,
                ..ReactionLimits::default()
            },
            StopReason::WorkBudget,
        ),
        (
            ReactionLimits {
                max_attempts: 1,
                max_age_seconds: 1,
                ..ReactionLimits::default()
            },
            StopReason::Age,
        ),
    ] {
        let mut ledger = ledger_with_limits(DeliveryProfile::ReconcileBeforeRetry, limits);
        let first = started(&mut ledger);
        finish(&mut ledger, &first, DeliveryOutcome::Unknown).unwrap();
        assert_eq!(ledger.work["a"].state, WorkState::AwaitingReconciliation);
        let request = control(
            &ledger,
            WorkControlDecision::DeliveryNotAccepted {
                evidence: "verified".into(),
            },
        );
        assert_eq!(
            ledger.control("a", &request).unwrap(),
            WorkControlOutcome::Stopped(expected.clone())
        );
        assert_eq!(ledger.work["a"].state, WorkState::Stopped(expected));
        assert_eq!(ledger.work["a"].delivery, Some(DeliveryOutcome::Permanent));
        assert_eq!(ledger.work["a"].attempts, 1);
        assert_eq!(ledger.roots["root"], 1);
    }
}

#[test]
fn default_profile_plain_retry_keeps_existing_unknown_delivery_policy() {
    let mut ledger = ledger(DeliveryProfile::AtLeastOnce);
    let first = started(&mut ledger);
    ledger
        .apply(WorkUpdate::Finish {
            claim: first.key(),
            now: 12,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        })
        .unwrap();
    let request = control(&ledger, WorkControlDecision::Retry);
    assert_eq!(
        ledger.control("a", &request).unwrap(),
        WorkControlOutcome::Scheduled
    );
    assert_eq!(claim(&mut ledger, 12).work.attempts, 2);
}

#[test]
fn another_records_revision_overflow_rolls_back_prior_hold_conversion() {
    let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
    started(&mut ledger);
    ledger
        .enqueue(
            &ReactionLimits::default(),
            vec![pending("b", DeliveryProfile::AtLeastOnce)],
        )
        .unwrap();
    ledger.work.get_mut("b").unwrap().revision = u64::MAX;
    let before = ledger.clone();
    assert!(matches!(
        ledger.apply(WorkUpdate::Claim { now: 40 }),
        Err(Error::TooLarge)
    ));
    assert_eq!(ledger, before);
}

#[test]
fn hold_generation_or_revision_overflow_rolls_back_all_candidate_changes() {
    for field in ["generation", "revision"] {
        for recovery in ["finish", "expired", "restore"] {
            let mut ledger = ledger(DeliveryProfile::ReconcileBeforeRetry);
            started(&mut ledger);
            let mut wire = serde_json::to_value(&ledger).unwrap();
            wire["work"]["a"][field] = json!(u64::MAX);
            if field == "generation" {
                wire["work"]["a"]["state"]["Leased"]["generation"] = json!(u64::MAX);
            }
            let mut ledger: WorkLedger = serde_json::from_value(wire).unwrap();
            let before = ledger.clone();
            let result = match recovery {
                "restore" => ledger.prepare_restore(),
                "expired" => ledger.apply(WorkUpdate::Claim { now: 40 }).map(|_| ()),
                _ => ledger
                    .apply(WorkUpdate::DeliveryFinished {
                        claim: ClaimKey {
                            id: "a".into(),
                            generation: ledger.work["a"].generation,
                        },
                        now: 12,
                        outcome: DeliveryOutcome::Unknown,
                    })
                    .map(|_| ()),
            };
            assert!(
                matches!(result, Err(Error::TooLarge)),
                "{field}/{recovery}: {result:?}"
            );
            assert_eq!(ledger, before);
        }
    }
}
