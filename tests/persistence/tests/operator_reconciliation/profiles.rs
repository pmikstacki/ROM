use super::support::*;
use rom::*;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn all_declared_profiles_preserve_their_uncertain_delivery_policy() {
    for redb in [false, true] {
        for profile in [
            DeliveryProfile::AtLeastOnce,
            DeliveryProfile::ProviderDeduplicated,
            DeliveryProfile::ReconcileBeforeRetry,
        ] {
            for (mode, outcome) in [
                (SendMode::Unknown, DeliveryOutcome::Unknown),
                (SendMode::Timeout, DeliveryOutcome::TimedOut),
                (SendMode::Panic, DeliveryOutcome::Panicked),
            ] {
                let fixture = Fixture::new(redb);
                let runtime = fixture.runtime(MAIL.delivery_profile(profile.clone()), mode);
                enqueue(&runtime).await;
                runtime.process_work(8).await.unwrap();
                let before = fixture.snapshot();
                assert_eq!(before.records[0].pending.delivery_profile, profile);
                assert_eq!(before.records[0].delivery, Some(outcome.clone()));
                fixture.clock.0.store(102, Ordering::SeqCst);
                if profile == DeliveryProfile::ReconcileBeforeRetry {
                    assert_eq!(before.records[0].state, WorkState::AwaitingReconciliation);
                    assert_eq!(runtime.process_work(8).await.unwrap(), 0);
                    assert_eq!(fixture.send_count(), 1);
                    assert_eq!(fixture.snapshot(), before);
                } else if outcome == DeliveryOutcome::Panicked {
                    assert_eq!(
                        before.records[0].state,
                        WorkState::Stopped(StopReason::CallbackPanicked)
                    );
                    assert_eq!(runtime.process_work(8).await.unwrap(), 0);
                    assert_eq!(fixture.send_count(), 1);
                } else {
                    assert_eq!(before.records[0].state, WorkState::Pending);
                    assert_eq!(runtime.process_work(8).await.unwrap(), 1);
                    let sends = fixture.sends.lock().unwrap();
                    assert_eq!(sends.len(), 2);
                    assert_eq!(sends[0].id, sends[1].id);
                    assert_eq!((sends[0].attempt, sends[1].attempt), (1, 2));
                }
                shutdown(runtime).await;
            }
        }
    }
}

#[tokio::test]
async fn confirmed_delivery_outcomes_remain_terminal_or_retryable_for_every_profile() {
    for redb in [false, true] {
        for profile in [
            DeliveryProfile::AtLeastOnce,
            DeliveryProfile::ProviderDeduplicated,
            DeliveryProfile::ReconcileBeforeRetry,
        ] {
            for (mode, state) in [
                (SendMode::Accepted, WorkState::Done),
                (
                    SendMode::Permanent,
                    WorkState::Stopped(StopReason::DeliveryPermanent),
                ),
                (SendMode::Retryable, WorkState::Pending),
            ] {
                let fixture = Fixture::new(redb);
                let runtime = fixture.runtime(MAIL.delivery_profile(profile.clone()), mode);
                enqueue(&runtime).await;
                assert_eq!(runtime.process_work(8).await.unwrap(), 1);
                assert_eq!(fixture.snapshot().records[0].state, state);
                assert_eq!(fixture.send_count(), 1);
                shutdown(runtime).await;
            }
        }
    }
}
