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

struct Paused {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    calls: Arc<AtomicUsize>,
}
impl Paused {
    fn new() -> Self {
        Self {
            entered: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn registration(&self) -> ChannelRegistration<String> {
        let entered = self.entered.clone();
        let release = self.release.clone();
        let calls = self.calls.clone();
        MAIL.delivery_profile(DeliveryProfile::ReconcileBeforeRetry)
            .verification_timeout(Duration::from_secs(2))
            .verifier(move |_: DeliveryReconciliation| {
                let wait = calls.fetch_add(1, Ordering::SeqCst) == 0;
                let entered = entered.clone();
                let release = release.clone();
                async move {
                    if wait {
                        entered.notify_one();
                        release.notified().await;
                    }
                    DeliveryVerification::Accepted {
                        evidence: "provider-audit".into(),
                    }
                }
            })
    }
    async fn entered(&self) {
        tokio::time::timeout(Duration::from_secs(3), self.entered.notified())
            .await
            .unwrap();
    }
    fn finish(&self) {
        self.release.notify_one();
    }
}
fn begin(
    runtime: &Runtime,
    request: WorkControlRequest,
) -> tokio::task::JoinHandle<Result<WorkControlResult>> {
    let runtime = runtime.clone();
    tokio::spawn(async move { runtime.work_control(&actor(), request).await })
}
async fn complete(
    task: tokio::task::JoinHandle<Result<WorkControlResult>>,
) -> Result<WorkControlResult> {
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn verifier_wait_holds_neither_core_commit_gate_nor_native_transaction() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let paused = Paused::new();
        let runtime = fixture.runtime(paused.registration(), SendMode::Unknown);
        hold(&fixture, &runtime).await;
        let task = begin(&runtime, request(&fixture, "waiting"));
        paused.entered().await;
        let unrelated = tokio::time::timeout(
            Duration::from_secs(1),
            runtime.execute(
                &actor(),
                Command::create("unrelated", Notice { count: 7 }).idempotency("unrelated"),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(unrelated.revision, 1);
        assert_eq!(
            fixture.snapshot().records[0].state,
            WorkState::AwaitingReconciliation
        );
        paused.finish();
        assert_eq!(
            complete(task).await.unwrap().outcome,
            WorkControlOutcome::Completed
        );
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn stale_work_version_after_verification_cannot_publish_a_second_receipt() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let paused = Paused::new();
        let runtime = fixture.runtime(paused.registration(), SendMode::Unknown);
        hold(&fixture, &runtime).await;
        let first = begin(&runtime, request(&fixture, "first"));
        paused.entered().await;
        let winner = runtime
            .work_control(&actor(), request(&fixture, "second"))
            .await
            .unwrap();
        assert_eq!(winner.outcome, WorkControlOutcome::Completed);
        paused.finish();
        assert_eq!(complete(first).await, Err(Error::Conflict));
        assert_eq!(fixture.snapshot().operator.receipts.len(), 1);
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn concurrent_exact_request_replays_the_receipt_before_rechecking_stale_cas() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let paused = Paused::new();
        let runtime = fixture.runtime(paused.registration(), SendMode::Unknown);
        hold(&fixture, &runtime).await;
        let original = request(&fixture, "same-key");
        let before = fixture.snapshot();
        let first = begin(&runtime, original.clone());
        paused.entered().await;
        let mut winner = runtime.work_control(&actor(), original).await.unwrap();
        assert_eq!(winner.outcome, WorkControlOutcome::Completed);
        assert!(!winner.replayed);
        let committed = fixture.snapshot();
        assert_eq!(committed.operator.receipts.len(), 1);
        assert_eq!(
            committed.records[0].revision,
            before.records[0].revision + 1
        );
        paused.finish();
        winner.replayed = true;
        assert_eq!(complete(first).await.unwrap(), winner);
        assert_eq!(fixture.snapshot(), committed);
        assert_eq!(paused.calls.load(Ordering::SeqCst), 2);
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}

#[tokio::test]
async fn current_operator_actor_and_service_revocation_are_rechecked_after_verification() {
    for redb in [false, true] {
        for revoked in ["policy", "actor", "service", "source"] {
            let fixture = Fixture::new(redb);
            let paused = Paused::new();
            let runtime = fixture.runtime(paused.registration(), SendMode::Unknown);
            hold(&fixture, &runtime).await;
            let before = fixture.snapshot();
            let task = begin(&runtime, request(&fixture, "revoked"));
            paused.entered().await;
            match revoked {
                "policy" => fixture.revoked.store(true, Ordering::SeqCst),
                "actor" => runtime.revoke(&actor()),
                "source" => {
                    runtime
                        .execute(
                            &actor(),
                            Command::replace("one", Notice { count: 99 })
                                .at_revision(2)
                                .idempotency("revoke-source-fields"),
                        )
                        .await
                        .unwrap();
                }
                _ => runtime.revoke(&service()),
            };
            paused.finish();
            assert_eq!(complete(task).await, Err(Error::Denied), "{revoked}");
            assert_eq!(fixture.snapshot(), before);
            assert_eq!(fixture.send_count(), 1);
            shutdown(runtime).await;
        }
    }
}

#[tokio::test]
async fn caller_cancellation_retains_verifier_capacity_until_commit_and_replay() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let paused = Paused::new();
        let runtime = fixture
            .configure(
                fixture.builder().limits(Limits {
                    io_jobs: 1,
                    ..Default::default()
                }),
                paused.registration(),
                SendMode::Unknown,
            )
            .build(fixture.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        hold(&fixture, &runtime).await;
        let original = request(&fixture, "cancelled");
        let task = begin(&runtime, original.clone());
        paused.entered().await;
        task.abort();
        let _ = task.await;
        assert_eq!(runtime.available_io_capacity(), 0);
        assert_eq!(runtime.status().unwrap().owned_work, 1);
        assert_eq!(
            runtime.work_read(&actor(), original.handle.clone()).await,
            Err(Error::Overloaded)
        );
        assert_eq!(fixture.snapshot().operator.receipts.len(), 0);
        paused.finish();
        shutdown(runtime).await;
        let runtime = fixture.runtime(paused.registration(), SendMode::Unknown);
        let replay = runtime.work_control(&actor(), original).await.unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.outcome, WorkControlOutcome::Completed);
        assert_eq!(paused.calls.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.send_count(), 1);
        shutdown(runtime).await;
    }
}
