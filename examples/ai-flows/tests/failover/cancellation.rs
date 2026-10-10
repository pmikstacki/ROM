//! Real adapter recovery after the existing dispatch deadline drops pending dispatch.
use super::*;
use std::{future::pending, time::Duration};

#[derive(Default)]
pub(super) struct PendingDispatch {
    entered: tokio::sync::Notify,
    drops: AtomicU64,
}
struct DispatchDrop<'a>(&'a PendingDispatch);
impl Drop for DispatchDrop<'_> {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl PendingDispatch {
    pub(super) async fn wait(&self) -> AiResult<DispatchOutcome> {
        let _drop = DispatchDrop(self);
        self.entered.notify_one();
        pending().await
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_pending_successor_cancellation_preserves_hold_after_reopen() {
    pending_successor(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_pending_successor_cancellation_preserves_hold_after_reopen() {
    pending_successor(true).await;
}
async fn pending_successor(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-phase-cancel-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let gate = Arc::new(PendingDispatch::default());
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
    *provider.controls.pending_dispatch.lock().unwrap() = Some(gate.clone());
    let grant = Arc::new(AtomicBool::new(true));
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        None,
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("phase-cancel-account"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    clock.0.store(3000, Ordering::SeqCst);
    // Production retains its 16-second execution budget, 18-second callback deadline,
    // and default worker stacks. The 25-second test bound includes delivery drain.
    // The actual dispatch timeout cancels the pending provider future; no result is invented.
    let (drained, frozen) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(25), runtime.process_work(16)),
        async {
            tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
                .await
                .expect("successor must enter the real dispatch phase");
            let record = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(record.state(), &RunState::Executing);
            let active = record.active_attempt().unwrap().clone();
            assert_eq!(active.key().attempt_ordinal(), 2);
            assert_eq!(active.prepared().route().model(), "synthetic/alternate");
            let account = runtime
                .read::<AiBudget>(&service(), "test-account")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_uncertain_start_account(&account, &active);
            let view = client.view(&owner(), &handle).await.unwrap();
            let cancelled = client
                .cancel(
                    &owner(),
                    &handle,
                    view.revision(),
                    "cancel-pending-successor",
                )
                .await
                .unwrap();
            assert_eq!(cancelled.state(), &RunState::CancelRequested);
            assert!(cancelled.cancel_requested());
            assert_eq!(
                gate.drops.load(Ordering::SeqCst),
                0,
                "cancel request is not proof of provider nonacceptance"
            );
            (active, record.expires_at_unix_ms())
        }
    );
    drained
        .expect("existing dispatch deadline must drop and drain pending dispatch")
        .unwrap();
    assert_eq!(gate.drops.load(Ordering::SeqCst), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_held(&runtime, &client, &handle, &frozen.0, frozen.1).await;
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(&path, redb, clock.clone(), provider.clone(), grant, None);
    assert_held(&runtime, &client, &handle, &frozen.0, frozen.1).await;
    clock.0.store(5000, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(5), runtime.process_work(16))
        .await
        .expect("unresolved Work must not dispatch again")
        .unwrap();
    assert_held(&runtime, &client, &handle, &frozen.0, frozen.1).await;
    {
        let sent = provider.attempts.lock().unwrap();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            sent.iter()
                .filter(|a| a.route().model() == "synthetic/model")
                .count(),
            1
        );
        assert_eq!(
            sent.iter()
                .filter(|a| a.route().model() == "synthetic/alternate")
                .count(),
            1
        );
        assert_eq!(&sent[1], frozen.0.prepared());
        drop(sent);
    }
    assert_eq!(gate.drops.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
}
async fn assert_held(
    runtime: &Runtime,
    client: &FlowClient,
    handle: &rom_ai::flow::RunHandle,
    expected: &rom_ai::flow::ReservationEntry,
    expiry: u64,
) {
    let view = client.view(&owner(), handle).await.unwrap();
    // An unresolved delivery can retain CancelRequested or recover into AwaitingReconciliation.
    // Neither state proves nonacceptance or grants permission to zero-settle the successor.
    assert!(matches!(
        view.state(),
        RunState::CancelRequested | RunState::AwaitingReconciliation
    ));
    assert!(view.cancel_requested());
    let record = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.expires_at_unix_ms(), expiry);
    assert_eq!(record.active_attempt().unwrap().key(), expected.key());
    assert_eq!(
        record.active_attempt().unwrap().prepared(),
        expected.prepared()
    );
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_uncertain_start_account(&account, expected);
}
