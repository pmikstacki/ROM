//! Proposed real-adapter regressions; no execution evidence yet.
use super::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_prepared_expiry_in_preflight_fails_and_releases_after_reopen() {
    prepared_failure(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_prepared_expiry_in_preflight_fails_and_releases_after_reopen() {
    prepared_failure(true, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_prepared_attempt_only_denial_is_public_terminal() {
    prepared_failure(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_prepared_attempt_only_denial_is_public_terminal() {
    prepared_failure(true, true).await;
}
async fn prepared_failure(redb: bool, denial: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-v2-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let audit = Arc::new(fault::FaultAudit::at(fault::Point::Prepared));
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        Some(audit.clone()),
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("account-v2"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    clock.0.store(3000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    assert!(audit.fired.load(Ordering::SeqCst));
    let prepared = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(prepared.state(), &RunState::Prepared);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    if denial {
        provider.controls.deny_attempt.store(true, Ordering::SeqCst);
    } else {
        *provider.controls.preflight_clock.lock().unwrap() =
            Some((clock.clone(), prepared.expires_at_unix_ms()));
    }
    let cleanup = Arc::new(fault::FaultAudit::at(fault::Point::Tombstone));
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        Some(cleanup.clone()),
    );
    clock.0.store(5000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    let view = client.view(&owner(), &handle).await.unwrap();
    assert_eq!(view.state(), &RunState::Failed);
    assert_eq!(
        view.failure(),
        Some(&if denial {
            AiError::Denied
        } else {
            AiError::DeadlineExceeded
        })
    );
    let failed = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(failed.expires_at_unix_ms(), prepared.expires_at_unix_ms());
    assert_eq!(failed.active_attempt(), prepared.active_attempt());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 2);
    assert_eq!(account.reserved(), UsdNanos(0));
    assert_eq!(account.settled(), UsdNanos(0));
    assert!(account.entries().iter().all(|e| e.status()
        == &ReservationStatus::Settled {
            actual_cost: UsdNanos(0)
        }));
    assert!(
        cleanup.fired.load(Ordering::SeqCst),
        "tombstone committed before losing its acknowledgement"
    );
    let cleanup_receipt = cleanup.captured.lock().unwrap().clone().unwrap().receipt;
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    clock.0.fetch_add(2000, Ordering::SeqCst);
    let (runtime, client) = build(
        &path,
        redb,
        clock,
        provider.clone(),
        grant,
        Some(cleanup.clone()),
    );
    assert_eq!(
        *cleanup.reopened_receipt.lock().unwrap(),
        Some(cleanup_receipt)
    );
    assert_eq!(
        client.view(&owner(), &handle).await.unwrap().state(),
        &RunState::Failed
    );
    runtime.process_work(16).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
}

#[derive(Clone, rom::Resource)]
#[resource(name = "failover_test_pulses")]
struct Pulse {
    count: u64,
}
const TWO_TICKS: rom::Action<Pulse, rom_ai::flow::FlowTick> =
    rom::Action::new("two_ticks", |pulse, tick| {
        pulse.count += 1;
        Ok(vec![
            rom_ai::flow::FLOW_TICKS.intent(tick.clone()),
            rom_ai::flow::FLOW_TICKS.intent(tick),
        ])
    });
pub(super) fn register(builder: rom::Builder) -> rom::Builder {
    use rom::Resource;
    builder.resource(
        Pulse::definition()
            .policy(|actor, _, _| {
                actor.authority == service().authority && actor.subject == service().subject
            })
            .allow_all_fields()
            .action(TWO_TICKS),
    )
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_overlapping_prepared_callbacks_dispatch_alternate_once() {
    concurrent_prepared(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn redb_overlapping_prepared_callbacks_dispatch_alternate_once() {
    concurrent_prepared(true).await;
}
async fn concurrent_prepared(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-race-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let audit = Arc::new(fault::FaultAudit::at(fault::Point::Prepared));
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        Some(audit.clone()),
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("race-account"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    clock.0.store(3000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    let before = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(before.state(), &RunState::Prepared);
    assert!(audit.fired.load(Ordering::SeqCst));
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant,
        Some(audit.clone()),
    );
    runtime
        .execute(
            &service(),
            rom::Command::create("pulse", Pulse { count: 0 }).idempotency("race-pulse"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &service(),
            rom::Command::action(
                "pulse",
                TWO_TICKS,
                rom_ai::flow::FlowTick::new(&handle.0, u64::from(before.checkpoint())).unwrap(),
            )
            .at_revision(1)
            .idempotency("race-two"),
        )
        .await
        .unwrap();
    let prior_ids: Vec<_> = audit
        .work
        .lock()
        .unwrap()
        .iter()
        .map(|w| w.pending.id.clone())
        .collect();
    *provider.controls.barrier.lock().unwrap() = Some(Arc::new(tokio::sync::Barrier::new(2)));
    // Distinct original intents, one shared Prepared revision. The bounded barrier
    // fails if the runtime serializes callbacks instead of silently missing the race.
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        let (a, b) = tokio::join!(runtime.process_work(16), runtime.process_work(16));
        a.unwrap();
        b.unwrap();
    })
    .await
    .expect("two callbacks must reach the preflight barrier");
    *provider.controls.barrier.lock().unwrap() = None;
    assert_eq!(
        audit.successor_starts.load(Ordering::SeqCst),
        1,
        "count inner START commits, not receipt replay"
    );
    {
        let attempts = provider.attempts.lock().unwrap();
        assert_eq!(
            attempts
                .iter()
                .filter(|a| a.route().model() == "synthetic/alternate")
                .count(),
            1
        );
        drop(attempts);
    }
    let after = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.state(), &RunState::Completed);
    assert_eq!(after.expires_at_unix_ms(), before.expires_at_unix_ms());
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 2);
    assert_eq!(account.reserved(), UsdNanos(0));
    assert_eq!(account.settled(), UsdNanos(3));
    assert_eq!(
        account.entries()[1].prepared(),
        before.active_attempt().unwrap().prepared()
    );
    {
        let records = audit.work.lock().unwrap();
        assert!(
            records
                .iter()
                .any(|w| w.delivery == Some(rom::DeliveryOutcome::Accepted))
        );
        // The losing callback cannot report Accepted by replaying the winning START.
        assert!(records.iter().any(|w| !prior_ids.contains(&w.pending.id)
            && matches!(
                w.delivery,
                Some(rom::DeliveryOutcome::Permanent | rom::DeliveryOutcome::Retryable)
            )));
        drop(records);
    }
    runtime.shutdown().await.unwrap();
    drop(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_terminal_tombstone_fences_late_uncommitted_reserve() {
    late_reserve(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_terminal_tombstone_fences_late_uncommitted_reserve() {
    late_reserve(true).await;
}
async fn late_reserve(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-late-reserve-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let audit = Arc::new(fault::FaultAudit::at(fault::Point::ReserveBefore));
    let inner = open(&path, redb);
    let injected: Arc<dyn Storage> = Arc::new(fault::WaitingFault {
        inner: inner.clone(),
        audit: audit.clone(),
    });
    let (runtime, client) = build_storage(
        injected.clone(),
        clock.clone(),
        provider.clone(),
        grant.clone(),
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("late-account"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    clock.0.store(3000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    assert!(audit.fired.load(Ordering::SeqCst));
    let delayed = audit.captured.lock().unwrap().clone().unwrap();
    assert!(inner.receipt(&delayed.receipt.identity).unwrap().is_none());
    let before = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(
        before.entries().len(),
        1,
        "reservation absent but may still commit"
    );
    let view = client.view(&owner(), &handle).await.unwrap();
    let cancelled = client
        .cancel(&owner(), &handle, view.revision(), "cancel-late-reserve")
        .await
        .unwrap();
    assert_eq!(cancelled.state(), &RunState::Cancelled);
    // The exact delayed CAS is submitted only after the terminal proof and tombstone commit.
    assert!(
        inner.commit(&delayed).is_err(),
        "older account revision cannot resurrect a cancelled attempt"
    );
    let after = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.entries().len(), 2);
    assert_eq!(after.reserved(), UsdNanos(0));
    assert_eq!(after.settled(), UsdNanos(0));
    let tombstone = after
        .entries()
        .iter()
        .find(|e| e.key().attempt_ordinal() == 2)
        .unwrap();
    assert_eq!(
        tombstone.status(),
        &ReservationStatus::Settled {
            actual_cost: UsdNanos(0)
        }
    );
    let snapshot = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap();
    let replay = runtime
        .execute(
            &service(),
            rom::Command::action(
                "test-account",
                rom_ai::flow::RESERVE_BUDGET,
                rom_ai::flow::ReserveBudget::new(
                    tombstone.key().clone(),
                    tombstone.prepared().clone(),
                )
                .unwrap(),
            )
            .at_revision(snapshot.revision)
            .idempotency("reserve-after-tombstone"),
        )
        .await;
    // Existing reserve semantics may acknowledge an identical key; current ledger must stay settled.
    drop(replay);
    let retained = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(retained.entries(), after.entries());
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    drop(injected);
    drop(inner);
    let (runtime, client) = build(&path, redb, clock, provider.clone(), grant, None);
    assert_eq!(
        client.view(&owner(), &handle).await.unwrap().state(),
        &RunState::Cancelled
    );
    runtime.process_work(16).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
}
