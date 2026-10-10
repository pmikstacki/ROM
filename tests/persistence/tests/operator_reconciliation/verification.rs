use super::support::*;
use rom::operator::*;
use rom::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Copy)]
enum Verify {
    Accepted,
    NotAccepted,
    Unresolved,
    Timeout,
    Panic,
    Empty,
    Oversized,
}
fn registration(mode: Verify, calls: Arc<AtomicUsize>) -> ChannelRegistration<String> {
    MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
        .verifier(move |lookup: DeliveryReconciliation| {
            calls.fetch_add(1, Ordering::SeqCst);
            assert!(!lookup.id.is_empty());
            assert_eq!(
                lookup.evidence_ref,
                Some("opaque-provider-reference".into())
            );
            async move {
                match mode {
                    Verify::Accepted => DeliveryVerification::Accepted {
                        evidence: "provider-audit".into(),
                    },
                    Verify::NotAccepted => DeliveryVerification::NotAccepted {
                        evidence: "terminal-rejection".into(),
                    },
                    Verify::Unresolved => DeliveryVerification::Unresolved,
                    Verify::Timeout => std::future::pending().await,
                    Verify::Panic => panic!("PRIVATE-VERIFIER-PANIC"),
                    Verify::Empty => DeliveryVerification::Accepted {
                        evidence: String::new(),
                    },
                    Verify::Oversized => DeliveryVerification::NotAccepted {
                        evidence: "PRIVATE-EVIDENCE".repeat(100),
                    },
                }
            }
        })
        // Immediate cases test semantic results; the pending case tests a short deadline.
        .verification_timeout(match mode {
            Verify::Timeout => Duration::from_millis(20),
            _ => Duration::from_secs(2),
        })
}

#[tokio::test]
async fn accepted_and_terminal_not_accepted_commit_once_without_a_provider_send() {
    for redb in [false, true] {
        for accepted in [false, true] {
            let fixture = Fixture::new(redb);
            let calls = Arc::new(AtomicUsize::new(0));
            let runtime = fixture.runtime(
                registration(
                    if accepted {
                        Verify::Accepted
                    } else {
                        Verify::NotAccepted
                    },
                    calls.clone(),
                ),
                SendMode::Unknown,
            );
            hold(&fixture, &runtime).await;
            let before = fixture.snapshot();
            let counts = fixture.db().counts();
            let original = request(&fixture, "terminal");
            let result = runtime
                .work_control(&actor(), original.clone())
                .await
                .unwrap();
            assert_eq!(
                result.outcome,
                if accepted {
                    WorkControlOutcome::Completed
                } else {
                    WorkControlOutcome::Scheduled
                }
            );
            assert!(!result.replayed);
            let after = fixture.snapshot();
            let record = &after.records[0];
            assert_eq!(record.pending, before.records[0].pending);
            assert_eq!(record.attempts, before.records[0].attempts);
            assert_eq!(after.roots, before.roots);
            assert_eq!(record.revision, before.records[0].revision + 1);
            assert_eq!(
                record.delivery,
                Some(if accepted {
                    DeliveryOutcome::Accepted
                } else {
                    DeliveryOutcome::Permanent
                })
            );
            assert_eq!(after.operator.receipts.len(), 1);
            let replay = runtime.work_control(&actor(), original).await.unwrap();
            assert!(replay.replayed);
            assert_eq!(replay.version, result.version);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(fixture.send_count(), 1);
            assert_eq!(fixture.db().counts(), counts);
            assert_eq!(
                runtime.process_work(1).await.unwrap(),
                usize::from(!accepted)
            );
            assert_eq!(fixture.send_count(), if accepted { 1 } else { 2 });
            shutdown(runtime).await;
        }
    }
}

#[tokio::test]
async fn unresolved_eventual_lookup_miss_cannot_grant_resend_before_later_acceptance() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let calls = Arc::new(AtomicUsize::new(0));
        let queried = calls.clone();
        let id = Arc::new(std::sync::Mutex::new(None::<String>));
        let observed = id.clone();
        let registration = MAIL
            .delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
            .verifier(move |lookup: DeliveryReconciliation| {
                let first = queried.fetch_add(1, Ordering::SeqCst) == 0;
                let mut prior = observed.lock().unwrap();
                if let Some(id) = &*prior {
                    assert_eq!(&lookup.id, id);
                } else {
                    *prior = Some(lookup.id);
                }
                async move {
                    if first {
                        DeliveryVerification::Unresolved
                    } else {
                        DeliveryVerification::Accepted {
                            evidence: "late-provider-acceptance".into(),
                        }
                    }
                }
            })
            .verification_timeout(Duration::from_secs(2));
        let runtime = fixture.runtime(registration, SendMode::Unknown);
        hold(&fixture, &runtime).await;
        let before = fixture.snapshot();
        let unknown = request(&fixture, "lookup-miss");
        let result = runtime
            .work_control(&actor(), unknown.clone())
            .await
            .unwrap();
        assert_eq!(result.outcome, WorkControlOutcome::Unresolved);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.version, unknown.expected);
        assert!(!result.replayed);
        assert_eq!(fixture.snapshot(), before);
        assert_eq!(runtime.process_work(8).await.unwrap(), 0);
        let accepted = runtime
            .work_control(&actor(), request(&fixture, "lookup-later"))
            .await
            .unwrap();
        assert_eq!(accepted.outcome, WorkControlOutcome::Completed);
        assert_eq!(fixture.send_count(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            id.lock().unwrap().as_ref().unwrap(),
            &before.records[0].pending.id
        );
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn unresolved_timeout_panic_and_malformed_evidence_preserve_the_full_hold() {
    for redb in [false, true] {
        for mode in [
            Verify::Unresolved,
            Verify::Timeout,
            Verify::Panic,
            Verify::Empty,
            Verify::Oversized,
        ] {
            let fixture = Fixture::new(redb);
            let calls = Arc::new(AtomicUsize::new(0));
            let runtime = fixture.runtime(registration(mode, calls.clone()), SendMode::Unknown);
            hold(&fixture, &runtime).await;
            let before = fixture.snapshot();
            let req = request(&fixture, "unresolved");
            let result = tokio::time::timeout(
                Duration::from_secs(3),
                runtime.work_control(&actor(), req.clone()),
            )
            .await
            .unwrap();
            match result {
                Ok(result) => {
                    assert_eq!(result.outcome, WorkControlOutcome::Unresolved);
                    assert_eq!(result.version, req.expected);
                    assert!(!result.replayed);
                }
                Err(error) => assert!(!format!("{error:?}").contains("PRIVATE")),
            }
            assert_eq!(fixture.snapshot(), before);
            assert_eq!(runtime.process_work(8).await.unwrap(), 0);
            let mut retry = request(&fixture, "unsafe-retry");
            retry.operation = WorkControlOperation::Retry;
            assert!(runtime.work_control(&actor(), retry).await.is_err());
            assert_eq!(fixture.snapshot(), before);
            assert_eq!(fixture.send_count(), 1);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            shutdown(runtime).await;
        }
    }
}

#[tokio::test]
async fn terminal_not_accepted_cannot_reset_exhausted_attempts_or_root_budget() {
    for redb in [false, true] {
        for attempts in [false, true] {
            let fixture = Fixture::new(redb);
            let calls = Arc::new(AtomicUsize::new(0));
            let limits = if attempts {
                ReactionLimits {
                    max_attempts: 1,
                    ..Default::default()
                }
            } else {
                ReactionLimits {
                    max_work: 1,
                    ..Default::default()
                }
            };
            let runtime = fixture
                .configure(
                    fixture.builder().reaction_limits(limits),
                    registration(Verify::NotAccepted, calls),
                    SendMode::Unknown,
                )
                .build(fixture.storage(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            hold(&fixture, &runtime).await;
            let before = fixture.snapshot();
            let result = runtime
                .work_control(&actor(), request(&fixture, "bounded-rejection"))
                .await
                .unwrap();
            assert_eq!(
                result.outcome,
                WorkControlOutcome::Stopped(if attempts {
                    StopReason::Attempts
                } else {
                    StopReason::WorkBudget
                })
            );
            let after = fixture.snapshot();
            assert_eq!(after.roots, before.roots);
            assert_eq!(after.records[0].attempts, 1);
            assert_eq!(runtime.process_work(1).await.unwrap(), 0);
            assert_eq!(fixture.send_count(), 1);
            shutdown(runtime).await;
        }
    }
}

#[tokio::test]
async fn active_lease_is_rejected_before_external_verification() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let calls = Arc::new(AtomicUsize::new(0));
        let runtime = fixture.runtime(
            registration(Verify::Accepted, calls.clone()),
            SendMode::Unknown,
        );
        enqueue(&runtime).await;
        let claim = match fixture
            .storage()
            .reaction_update(WorkUpdate::Claim { now: 100 })
            .unwrap()
        {
            WorkResult::Claimed(claim) => claim,
            _ => panic!("claim missing"),
        };
        fixture
            .storage()
            .reaction_update(WorkUpdate::DeliveryStarted {
                claim: claim.key(),
                now: 100,
            })
            .unwrap();
        let before = fixture.snapshot();
        assert_eq!(
            runtime
                .work_control(&actor(), request(&fixture, "active"))
                .await,
            Err(Error::Conflict)
        );
        assert_eq!(fixture.snapshot(), before);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        shutdown(runtime).await;
    }
}

#[test]
fn zero_and_unrepresentable_verification_timeouts_are_invalid_registrations() {
    let fixture = Fixture::new(false);
    for timeout in [Duration::ZERO, Duration::MAX] {
        let configured = fixture.configure(
            fixture.builder(),
            MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
                .verification_timeout(timeout),
            SendMode::Unknown,
        );
        assert!(
            configured
                .build(fixture.storage(), Runtime::shared_cpu_pool(1).unwrap())
                .is_err()
        );
    }
}
