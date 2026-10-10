//! Proposed alternate-model regression bodies. Uncompiled; only public APIs and real adapters.
use rom::{Actor, Runtime, Storage};
use rom_ai::flow::{
    AiBudget, AiRun, FlowAuthority, FlowClient, FlowHost, OwnerIdentity, ReservationStatus,
    RunState, Submission,
};
use rom_ai::{
    AiClock, AiError, AiFuture, AiResult, AttemptEvidence, CatalogModel, CatalogSnapshot,
    Completion, CompletionRequest, Deadline, DispatchOutcome, Message, ModelPrice, OutputValidator,
    PreparedAttempt, Provider, Reconciliation, RoutingPolicy, RunLimits, Usage, UsdNanos,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[path = "failover/cancellation.rs"]
mod cancellation;
#[path = "failover/fault.rs"]
mod fault;

fn owner() -> Actor {
    Actor::trusted("nonacceptance-consumer", "author")
}
fn service() -> Actor {
    Actor::trusted("nonacceptance-consumer", "service").with_kind(rom::PrincipalKind::Service)
}
struct Clock(AtomicU64);
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl rom::Clock for Clock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst) / 1000
    }
}
struct Authority(Arc<AtomicBool>, Arc<AtomicBool>);
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        if actor.authority == owner().authority && actor.subject == owner().subject {
            OwnerIdentity::from_actor(actor)
        } else {
            Err(AiError::Denied)
        }
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if self.0.load(Ordering::SeqCst) && identity.matches(actor) && identity.matches(&owner()) {
            Ok(())
        } else {
            Err(AiError::Denied)
        }
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        self.inspect(actor, identity)
    }
    fn resolve(
        &self,
        identity: &OwnerIdentity,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if self.0.load(Ordering::SeqCst) && identity.matches(&owner()) {
            Ok(owner())
        } else {
            Err(rom::Error::Denied)
        }
    }
    fn attempt(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        prepared: &PreparedAttempt,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        prepared.validate().map_err(|_| rom::Error::Denied)?;
        if self.0.load(Ordering::SeqCst)
            && !self.1.load(Ordering::SeqCst)
            && identity.matches(actor)
            && identity.matches(&owner())
            && prepared.route().maximum_cost() == UsdNanos(10)
        {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        Ok(value.clone())
    }
}
#[derive(Clone, Copy)]
enum Mode {
    RequestReference,
    ContradictoryGeneration,
    KnownCost,
    UntrustedCost,
}
#[derive(Default)]
struct Controls {
    deny_attempt: Arc<AtomicBool>,
    preflight_clock: Mutex<Option<(Arc<Clock>, u64)>>,
    barrier: Mutex<Option<Arc<tokio::sync::Barrier>>>,
    pending_dispatch: Mutex<Option<Arc<cancellation::PendingDispatch>>>,
}
struct Fixture {
    controls: Controls,
    mode: Mode,
    calls: AtomicU64,
    attempts: Mutex<Vec<PreparedAttempt>>,
    retry_after_ms: u64,
    resolve_nonaccepted: AtomicBool,
    exclude_alternate: AtomicBool,
}
impl Provider for Fixture {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async move {
            let excluded = self.exclude_alternate.load(Ordering::SeqCst);
            CatalogSnapshot::new(
                if excluded {
                    "fresh-overpriced"
                } else {
                    "synthetic-paid"
                },
                ["synthetic/model", "synthetic/alternate"]
                    .into_iter()
                    .map(|id| {
                        CatalogModel::text(
                            id,
                            4096,
                            true,
                            true,
                            if excluded && id == "synthetic/alternate" {
                                ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(11))
                            } else {
                                price()
                            },
                        )
                    })
                    .collect(),
            )
        })
    }
    fn preflight<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, ()> {
        Box::pin(async move {
            if attempt.route().model() == "synthetic/alternate" {
                let barrier = self.controls.barrier.lock().unwrap().clone();
                if let Some(barrier) = barrier {
                    tokio::time::timeout(std::time::Duration::from_secs(3), barrier.wait())
                        .await
                        .map_err(|_| AiError::ProviderUnavailable)?;
                }
                if let Some((clock, value)) = self.controls.preflight_clock.lock().unwrap().as_ref()
                {
                    clock.0.store(*value, Ordering::SeqCst);
                }
            }
            Ok(())
        })
    }
    fn complete<'a>(&'a self, _: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async { Err(AiError::UnknownOutcome) })
    }
    fn complete_observed<'a>(
        &'a self,
        prepared: &'a PreparedAttempt,
    ) -> AiFuture<'a, DispatchOutcome> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.attempts.lock().unwrap().push(prepared.clone());
            if prepared.route().model() == "synthetic/alternate" {
                let pending_dispatch = self.controls.pending_dispatch.lock().unwrap().clone();
                if let Some(pending_dispatch) = pending_dispatch {
                    return pending_dispatch.wait().await;
                }
                return Ok(DispatchOutcome::Completed(Completion::output(
                    serde_json::json!({"answer":"done"}),
                    Usage {
                        cost: Some(UsdNanos(3)),
                        ..Usage::default()
                    },
                    AttemptEvidence::new(
                        prepared.identity(),
                        Some("request-successor".into()),
                        Some("gen-successor".into()),
                    )?,
                )?));
            }
            let evidence = AttemptEvidence::new(
                prepared.identity(),
                Some("request-original".into()),
                matches!(self.mode, Mode::ContradictoryGeneration).then(|| "gen-original".into()),
            )?;
            Ok(match self.mode {
                Mode::RequestReference | Mode::ContradictoryGeneration => {
                    DispatchOutcome::NotAccepted {
                        evidence,
                        retry_after_ms: self.retry_after_ms,
                    }
                }
                // Normalized adapter contract after invalid/over-ceiling wire usage.
                Mode::UntrustedCost => DispatchOutcome::Uncertain {
                    evidence,
                    usage: Usage::default(),
                    cause: AiError::UnknownOutcome,
                },
                Mode::KnownCost => DispatchOutcome::Uncertain {
                    evidence,
                    usage: Usage {
                        cost: Some(UsdNanos(3)),
                        ..Usage::default()
                    },
                    cause: AiError::UnknownOutcome,
                },
            })
        })
    }
    fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async move {
            Ok(if self.resolve_nonaccepted.load(Ordering::SeqCst) {
                Reconciliation::NotAccepted
            } else {
                Reconciliation::Unresolved
            })
        })
    }
}
fn price() -> ModelPrice {
    ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10))
}
fn submission() -> Submission {
    Submission {
        id: "run-original".into(),
        idempotency: "submission-original".into(),
        request: CompletionRequest::new(vec![Message::user("synthetic request")], 32).unwrap(),
        policy: RoutingPolicy::new(
            1,
            vec![],
            vec!["synthetic/model".into(), "synthetic/alternate".into()],
            Some(price()),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("test-account")
        .unwrap()
        .with_failover(rom_ai::FailoverPolicy::AdvanceOnConfirmedNonacceptance)
        .unwrap(),
    }
}
fn open(path: &std::path::Path, redb: bool) -> Arc<dyn Storage> {
    if redb {
        Arc::new(rom_redb::Redb::open(path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path).unwrap())
    }
}
fn build(
    path: &std::path::Path,
    redb: bool,
    clock: Arc<Clock>,
    provider: Arc<Fixture>,
    grant: Arc<AtomicBool>,
    lost_ack: Option<Arc<fault::FaultAudit>>,
) -> (Runtime, FlowClient) {
    let inner = open(path, redb);
    let storage: Arc<dyn Storage> = if let Some(audit) = lost_ack {
        Arc::new(fault::WaitingFault { inner, audit })
    } else {
        inner
    };
    build_storage(storage, clock, provider, grant)
}
fn build_storage(
    storage: Arc<dyn Storage>,
    clock: Arc<Clock>,
    provider: Arc<Fixture>,
    grant: Arc<AtomicBool>,
) -> (Runtime, FlowClient) {
    FlowHost::new(
        service(),
        provider.clone(),
        Arc::new(Authority(grant, provider.controls.deny_attempt.clone())),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .install(regressions::register(Runtime::builder().clock(clock)))
    .unwrap()
    .limits(rom::Limits {
        io_jobs: 4,
        ..Default::default()
    })
    .unwrap()
    .build(storage, Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_exact_reference_survives_due_and_reopen() {
    journey(false, false, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_exact_reference_survives_due_and_reopen() {
    journey(true, false, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_generation_is_held_without_zero_settlement() {
    journey(false, false, Mode::ContradictoryGeneration).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_generation_is_held_without_zero_settlement() {
    journey(true, false, Mode::ContradictoryGeneration).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_uncertain_known_cost_stays_charged_without_retry() {
    journey(false, false, Mode::KnownCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_uncertain_known_cost_stays_charged_without_retry() {
    journey(true, false, Mode::KnownCost).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_lost_waiting_ack_preserves_receipt_and_single_successor() {
    journey(false, true, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_lost_waiting_ack_preserves_receipt_and_single_successor() {
    journey(true, true, Mode::RequestReference).await;
}

async fn journey(redb: bool, lost_ack: bool, mode: Mode) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-nonacceptance-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let armed = lost_ack.then(|| Arc::new(fault::FaultAudit::new()));
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        armed.clone(),
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("account-create"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    if let Some(armed) = &armed {
        assert!(
            armed.fired.load(Ordering::SeqCst),
            "Waiting commit must succeed before acknowledgement loss"
        );
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let record = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let original_expiry = record.expires_at_unix_ms();
    let original_attempt = record.active_attempt().unwrap().prepared().clone();
    assert_eq!(record.attempt_evidence().len(), 1);
    assert_eq!(
        record.attempt_evidence()[0].attempt_id(),
        original_attempt.identity()
    );
    assert_eq!(
        record.attempt_evidence()[0].provider_request_id(),
        Some("request-original")
    );
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 1);
    match mode {
        Mode::RequestReference => {
            assert_eq!(
                record.state(),
                &RunState::Waiting {
                    retry_at_unix_ms: 3000
                }
            );
            if lost_ack {
                assert!(matches!(
                    account.entries()[0].status(),
                    ReservationStatus::Reserved | ReservationStatus::Unknown
                ));
                assert_eq!(account.reserved(), UsdNanos(10));
                assert_eq!(account.settled(), UsdNanos(0));
                let captured = armed
                    .as_ref()
                    .unwrap()
                    .captured
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap();
                assert!(!captured.receipt.identity.is_empty());
                assert!(!captured.receipt.fingerprint.is_empty());
                assert_eq!(captured.receipt.row.key.id, handle.0);
                let committed =
                    <AiRun as rom::Resource>::decode(captured.receipt.row.value.clone().unwrap())
                        .unwrap()
                        .record()
                        .unwrap();
                assert_eq!(committed.attempt_evidence(), record.attempt_evidence());
                let successors: Vec<_> = captured
                    .effects
                    .iter()
                    .filter(|intent| intent.channel == "ai_flow_ticks")
                    .collect();
                assert_eq!(successors.len(), 1);
                assert_eq!(successors[0].not_before, Some(3));
                assert_eq!(successors[0].payload["run_id"], handle.0);
            } else {
                assert_eq!(
                    account.entries()[0].status(),
                    &ReservationStatus::Settled {
                        actual_cost: UsdNanos(0)
                    }
                );
            }
        }
        Mode::ContradictoryGeneration => {
            assert_eq!(record.state(), &RunState::AwaitingReconciliation);
            assert_eq!(
                record.attempt_evidence()[0].generation_id(),
                Some("gen-original")
            );
            assert_eq!(account.entries()[0].status(), &ReservationStatus::Unknown);
            assert_eq!(account.reserved(), UsdNanos(10));
        }
        Mode::UntrustedCost => {
            assert_eq!(record.state(), &RunState::AwaitingReconciliation);
            assert_eq!(account.entries()[0].status(), &ReservationStatus::Unknown);
            assert_eq!(account.reserved(), UsdNanos(10));
            assert_eq!(account.settled(), UsdNanos(0));
        }
        Mode::KnownCost => {
            assert_eq!(record.state(), &RunState::AwaitingReconciliation);
            assert_eq!(
                account.entries()[0].status(),
                &ReservationStatus::Settled {
                    actual_cost: UsdNanos(3)
                }
            );
            assert_eq!(account.settled(), UsdNanos(3));
        }
    }
    assert!(
        client
            .view(&owner(), &handle)
            .await
            .unwrap()
            .output()
            .is_none()
    );
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        armed.clone(),
    );
    assert_eq!(client.submit(&owner(), submission()).await.unwrap(), handle);
    let reopened = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(reopened.expires_at_unix_ms(), original_expiry);
    assert_eq!(
        reopened.active_attempt().unwrap().prepared(),
        &original_attempt
    );
    assert_eq!(reopened.attempt_evidence(), record.attempt_evidence());
    let reopened_budget = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert!(
        reopened_budget == account,
        "exact reservation/known cost survives reopen"
    );
    if let Some(audit) = &armed {
        let captured = audit.captured.lock().unwrap().clone().unwrap();
        assert_eq!(
            *audit.reopened_receipt.lock().unwrap(),
            Some(captured.receipt)
        );
    }
    clock.0.store(2999, Ordering::SeqCst);
    runtime.process_work(8).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    if lost_ack {
        let held = runtime
            .read::<AiBudget>(&service(), "test-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(held.entries().len(), 1);
        assert_eq!(held.entries()[0].prepared(), &original_attempt);
        assert_eq!(
            held.entries()[0].status(),
            &ReservationStatus::Settled {
                actual_cost: UsdNanos(0)
            }
        );
        assert_eq!(held.reserved(), UsdNanos(0));
        // The stale original delivery may settle the persisted uncharged proof before
        // the alternate becomes due. It still cannot dispatch before the retry floor.
        assert_eq!(held.settled(), UsdNanos(0));
    }
    clock.0.store(3000, Ordering::SeqCst);
    if matches!(mode, Mode::RequestReference) {
        runtime.process_work(8).await.unwrap();
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        let after = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(after.attempt_evidence()[0], record.attempt_evidence()[0]);
        assert_eq!(after.counters().generation_attempts(), 2);
        assert_eq!(
            provider
                .attempts
                .lock()
                .unwrap()
                .iter()
                .map(|a| a.route().model())
                .collect::<Vec<_>>(),
            vec!["synthetic/model", "synthetic/alternate"]
        );
        assert_eq!(after.expires_at_unix_ms(), original_expiry);
        let settled = runtime
            .read::<AiBudget>(&service(), "test-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(settled.entries().len(), 2);
        assert_eq!(settled.entries()[0].prepared(), &original_attempt);
        assert_eq!(
            settled.entries()[0].status(),
            &ReservationStatus::Settled {
                actual_cost: UsdNanos(0)
            }
        );
        assert_eq!(
            settled.entries()[1].status(),
            &ReservationStatus::Settled {
                actual_cost: UsdNanos(3)
            }
        );
        assert_ne!(
            after.active_attempt().unwrap().prepared().identity(),
            original_attempt.identity()
        );
    } else {
        let current = client.view(&owner(), &handle).await.unwrap();
        let resumed = client
            .resume(&owner(), &handle, current.revision(), "lookup-original")
            .await
            .unwrap();
        assert_eq!(resumed.state(), &RunState::AwaitingReconciliation);
        let resumed_budget = runtime
            .read::<AiBudget>(&service(), "test-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert!(
            resumed_budget == account,
            "unresolved reconciliation preserves held ceiling or exact known charge"
        );
        assert!(resumed.output().is_none());
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        grant.store(false, Ordering::SeqCst);
        assert!(matches!(
            client
                .resume(&owner(), &handle, resumed.revision(), "lookup-revoked")
                .await,
            Err(AiError::Denied)
        ));
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    }
    runtime.shutdown().await.unwrap();
    // Keep the database for the future execution owner's evidence, including failures.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_untrusted_usage_retains_reserved_ceiling_after_reopen() {
    journey(false, false, Mode::UntrustedCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_untrusted_usage_retains_reserved_ceiling_after_reopen() {
    journey(true, false, Mode::UntrustedCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_original_deadline_before_rounded_due_never_retransmits() {
    boundary(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_original_deadline_before_rounded_due_never_retransmits() {
    boundary(true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_current_grant_revoked_before_retry_never_retransmits() {
    boundary(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_current_grant_revoked_before_retry_never_retransmits() {
    boundary(true, false).await;
}

async fn boundary(redb: bool, expiry: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-nonacceptance-boundary-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: if expiry { 3001 } else { 1001 },
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
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
            .idempotency("account-create"),
        )
        .await
        .unwrap();
    let make_input = || {
        let mut input = submission();
        if expiry {
            input.policy = RoutingPolicy::new(
                1,
                vec![],
                vec!["synthetic/model".into(), "synthetic/alternate".into()],
                Some(price()),
                RunLimits {
                    age_seconds: 3,
                    ..RunLimits::default()
                },
            )
            .unwrap()
            .with_budget_reference("test-account")
            .unwrap()
            .with_failover(rom_ai::FailoverPolicy::AdvanceOnConfirmedNonacceptance)
            .unwrap();
        }
        input
    };
    let handle = client.submit(&owner(), make_input()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let before = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let frozen = before.active_attempt().unwrap().prepared().clone();
    let original_expiry = before.expires_at_unix_ms();
    if expiry {
        assert_eq!(original_expiry, 4000);
        assert!(original_expiry < 5000);
        assert_eq!(before.state(), &RunState::Failed);
        assert_eq!(before.failure(), Some(&AiError::DeadlineExceeded));
    } else {
        assert_eq!(
            before.state(),
            &RunState::Waiting {
                retry_at_unix_ms: 3000
            }
        );
    }
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(
        &path,
        redb,
        clock.clone(),
        provider.clone(),
        grant.clone(),
        None,
    );
    assert_eq!(client.submit(&owner(), make_input()).await.unwrap(), handle);
    if !expiry {
        grant.store(false, Ordering::SeqCst);
    }
    clock
        .0
        .store(if expiry { 5000 } else { 3000 }, Ordering::SeqCst);
    runtime.process_work(8).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let after = runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.expires_at_unix_ms(), original_expiry);
    assert_eq!(after.active_attempt().unwrap().prepared(), &frozen);
    assert_eq!(after.counters().generation_attempts(), 1);
    assert_ne!(after.state(), &RunState::Completed);
    if !expiry {
        assert!(matches!(
            client.view(&owner(), &handle).await,
            Err(AiError::Denied)
        ));
    }
    runtime.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_two_unrelated_runs_have_independent_durable_cursors() {
    two_runs(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_two_unrelated_runs_have_independent_durable_cursors() {
    two_runs(true).await;
}
async fn two_runs(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-independent-{}-{}",
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
    let make = |id: &str, prompt: &str| {
        let mut s = submission();
        s.id = id.into();
        s.idempotency = format!("submit-{id}");
        s.request = CompletionRequest::new(vec![Message::user(prompt)], 32).unwrap();
        s
    };
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
            .idempotency("account-two"),
        )
        .await
        .unwrap();
    let first = client
        .submit(
            &owner(),
            make("publication-run", "Publish this prepared edition"),
        )
        .await
        .unwrap();
    runtime.process_work(8).await.unwrap();
    let frozen = runtime
        .read::<AiRun>(&service(), &first.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert!(matches!(frozen.state(), RunState::Waiting { .. }));
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(&path, redb, clock.clone(), provider.clone(), grant, None);
    // An unrelated request starts at A although the reopened publication has rejected A.
    let second = client
        .submit(&owner(), make("triage-run", "Classify this ticket"))
        .await
        .unwrap();
    runtime.process_work(8).await.unwrap();
    let triage = runtime
        .read::<AiRun>(&service(), &second.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(
        triage.active_attempt().unwrap().prepared().route().model(),
        "synthetic/model"
    );
    clock.0.store(3000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    for handle in [&first, &second] {
        let r = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(r.state(), &RunState::Completed);
        assert_eq!(r.counters().generation_attempts(), 2);
        assert_eq!(
            r.active_attempt().unwrap().prepared().route().model(),
            "synthetic/alternate"
        );
        assert_eq!(r.expires_at_unix_ms(), frozen.expires_at_unix_ms());
        let attempts = provider.attempts.lock().unwrap();
        let own: Vec<_> = attempts
            .iter()
            .filter(|a| a.identity().starts_with(&format!("{}:", handle.0)))
            .collect();
        assert_eq!(own.len(), 2);
        assert_eq!(own[0].route().model(), "synthetic/model");
        assert_eq!(own[1].route().model(), "synthetic/alternate");
        assert_eq!(own[0].request(), own[1].request());
        assert_ne!(own[0].identity(), own[1].identity());
    }
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 4);
    assert_eq!(account.settled(), UsdNanos(6));
    assert_eq!(account.reserved(), UsdNanos(0));
    runtime.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_exact_commits_recover_lost_acknowledgements() {
    ack_boundaries(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_exact_commits_recover_lost_acknowledgements() {
    ack_boundaries(true).await;
}
async fn ack_boundaries(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    for point in [
        fault::Point::Waiting,
        fault::Point::Settlement,
        fault::Point::Reserve,
        fault::Point::Prepared,
        fault::Point::Start,
    ] {
        let dir = std::env::temp_dir().join(format!(
            "rom-failover-ack-{}-{}",
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
        let audit = Arc::new(fault::FaultAudit::at(point));
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
                .idempotency("account-ack"),
            )
            .await
            .unwrap();
        let handle = client.submit(&owner(), submission()).await.unwrap();
        runtime.process_work(8).await.unwrap();
        clock.0.store(3000, Ordering::SeqCst);
        runtime.process_work(16).await.unwrap();
        assert!(audit.fired.load(Ordering::SeqCst));
        let captured = audit.captured.lock().unwrap().clone().unwrap();
        assert!(!captured.receipt.identity.is_empty());
        assert!(!captured.receipt.fingerprint.is_empty());
        if matches!(point, fault::Point::Waiting) {
            let intents: Vec<_> = captured
                .effects
                .iter()
                .filter(|i| i.channel == "ai_flow_ticks")
                .collect();
            assert_eq!(intents.len(), 1);
            assert_eq!(intents[0].not_before, Some(3));
        }
        let before = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let expiry = before.expires_at_unix_ms();
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        let (runtime, client) = build(
            &path,
            redb,
            clock.clone(),
            provider.clone(),
            grant.clone(),
            Some(audit.clone()),
        );
        assert_eq!(
            *audit.reopened_receipt.lock().unwrap(),
            Some(captured.receipt.clone())
        );
        clock.0.store(5000, Ordering::SeqCst);
        runtime.process_work(16).await.unwrap();
        let after = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(after.expires_at_unix_ms(), expiry);
        {
            let sent = provider.attempts.lock().unwrap();
            assert_eq!(
                sent.iter()
                    .filter(|a| a.route().model() == "synthetic/model")
                    .count(),
                1
            );
            assert!(
                sent.iter()
                    .filter(|a| a.route().model() == "synthetic/alternate")
                    .count()
                    <= 1
            );
            if matches!(point, fault::Point::Start) {
                // Unknown START acknowledgement is not permission to POST. Exact replay may
                // prove the original start inside its callback; otherwise recovery holds it.
                assert_eq!(after.state(), &RunState::AwaitingReconciliation);
                assert_eq!(
                    sent.len(),
                    1,
                    "unresolved START acknowledgement must not POST the alternate"
                );
            } else {
                assert_eq!(after.state(), &RunState::Completed);
                assert_eq!(sent.len(), 2);
            }
            drop(sent);
        }
        let account = runtime
            .read::<AiBudget>(&service(), "test-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(account.entries().len(), 2);
        assert_eq!(
            account.entries()[0].status(),
            &ReservationStatus::Settled {
                actual_cost: UsdNanos(0)
            }
        );
        let successor = account
            .entries()
            .iter()
            .find(|e| e.key().attempt_ordinal() == 2)
            .unwrap();
        assert_eq!(
            successor.prepared(),
            after.active_attempt().unwrap().prepared()
        );
        if matches!(point, fault::Point::Start) {
            assert_uncertain_start_account(&account, after.active_attempt().unwrap());
        }
        if captured.receipt.row.key.kind == "ai_runs"
            && matches!(point, fault::Point::Prepared | fault::Point::Start)
        {
            let saved =
                <AiRun as rom::Resource>::decode(captured.receipt.row.value.clone().unwrap())
                    .unwrap()
                    .record()
                    .unwrap();
            assert_eq!(
                saved.active_attempt().unwrap().prepared(),
                successor.prepared()
            );
        }
        let frozen_successor = successor.clone();
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        if matches!(point, fault::Point::Start) {
            // Reopen the held state itself, not merely the original uncertain START.
            let (runtime, client) = build(
                &path,
                redb,
                clock.clone(),
                provider.clone(),
                grant,
                Some(audit.clone()),
            );
            assert_eq!(
                *audit.reopened_receipt.lock().unwrap(),
                Some(captured.receipt.clone())
            );
            let reopened = client.view(&owner(), &handle).await.unwrap();
            assert_eq!(reopened.state(), &RunState::AwaitingReconciliation);
            let money = runtime
                .read::<AiBudget>(&service(), "test-account")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_uncertain_start_account(&money, &frozen_successor);
            assert_eq!(
                provider.calls.load(Ordering::SeqCst),
                1,
                "reopen grants no permission for another POST"
            );
            // No reconciliation or nonacceptance input is supplied. Bounded Work
            // processing must retain the same uncertain attempt and full ceiling.
            runtime.process_work(16).await.unwrap();
            let retained = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(retained.state(), &RunState::AwaitingReconciliation);
            assert_eq!(retained.expires_at_unix_ms(), expiry);
            assert_eq!(
                retained.active_attempt().unwrap().key(),
                frozen_successor.key()
            );
            assert_eq!(
                retained.active_attempt().unwrap().prepared(),
                frozen_successor.prepared()
            );
            let money = runtime
                .read::<AiBudget>(&service(), "test-account")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_uncertain_start_account(&money, &frozen_successor);
            {
                let sent = provider.attempts.lock().unwrap();
                assert_eq!(sent.len(), 1);
                assert_eq!(sent[0].route().model(), "synthetic/model");
                assert!(
                    !sent
                        .iter()
                        .any(|a| a.route().model() == "synthetic/alternate")
                );
                drop(sent);
            }
            runtime.shutdown().await.unwrap();
            drop(client);
        }
    }
}

fn assert_uncertain_start_account(
    account: &rom_ai::flow::BudgetRecord,
    expected: &rom_ai::flow::ReservationEntry,
) {
    assert_eq!(account.entries().len(), 2);
    let predecessor = account
        .entries()
        .iter()
        .find(|e| e.key().attempt_ordinal() == 1)
        .unwrap();
    assert_eq!(predecessor.key().run_id(), expected.key().run_id());
    assert_eq!(
        predecessor.key().account_window(),
        expected.key().account_window()
    );
    assert_eq!(
        predecessor.status(),
        &ReservationStatus::Settled {
            actual_cost: UsdNanos(0)
        }
    );
    let successor = account
        .entries()
        .iter()
        .find(|e| e.key() == expected.key())
        .unwrap();
    assert_eq!(successor.prepared(), expected.prepared());
    assert_eq!(successor.key().attempt_ordinal(), 2);
    assert_eq!(successor.maximum_cost(), UsdNanos(10));
    assert!(
        matches!(
            successor.status(),
            ReservationStatus::Reserved | ReservationStatus::Unknown
        ),
        "uncertain START cannot be zero-settled"
    );
    assert_eq!(account.reserved(), UsdNanos(10));
    assert_eq!(account.settled(), UsdNanos(0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_rejects_forged_or_rewritten_persisted_decisions() {
    reject_mutations(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_rejects_forged_or_rewritten_persisted_decisions() {
    reject_mutations(true).await;
}
async fn reject_mutations(redb: bool) {
    use rom::Resource;
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-forgery-{}-{}",
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
    let (runtime, client) = build(
        &path,
        redb,
        clock,
        provider.clone(),
        Arc::new(AtomicBool::new(true)),
        None,
    );
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "test-account",
                AiBudget::new(&service(), "test-account", UsdNanos(100)).unwrap(),
            )
            .idempotency("account-negative"),
        )
        .await
        .unwrap();
    let handle = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let before = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
    let value = before.value.unwrap();
    let wire = value.encode();
    let inner: serde_json::Value = serde_json::from_str(wire["encoded"].as_str().unwrap()).unwrap();
    assert_eq!(inner["version"], 1);
    assert_eq!(inner["route_continuations"].as_array().unwrap().len(), 1);
    assert_eq!(
        inner["cursor"],
        inner["active_attempt"]["prepared"]["route"]["next_cursor"]
    );
    for case in 0..6 {
        let mut forged = inner.clone();
        match case {
            0 => forged["route_continuations"][0]["version"] = 2.into(),
            1 => {
                forged["route_continuations"][0]["forward_cursor"]["catalog_identity"] =
                    "forged-current".into()
            }
            2 => {
                let duplicate = forged["route_continuations"][0].clone();
                forged["route_continuations"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            3 => {
                forged["route_continuations"][0]["proof"]["evidence"]["attempt_id"] =
                    "other-run:1:1".into()
            }
            4 => forged["route_continuations"][0]["confirmed_cost"] = 11.into(),
            _ => {
                forged["route_continuations"][0]["staged"] = serde_json::Value::Null;
                forged["request"]["max_output_tokens"] = 1.into();
            }
        }
        let mut encoded = wire.clone();
        encoded["encoded"] = serde_json::to_string(&forged).unwrap().into();
        let attempt = AiRun::decode(encoded).unwrap();
        assert!(
            runtime
                .execute(
                    &service(),
                    rom::Command::replace(&handle.0, attempt)
                        .at_revision(before.revision)
                        .idempotency(&format!("forged-{case}"))
                )
                .await
                .is_err()
        );
        let unchanged = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        assert_eq!(unchanged.revision, before.revision);
        assert_eq!(unchanged.value.unwrap().encode(), wire);
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_positive_reconciliation_preserves_known_charge() {
    reconciled_charge(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_positive_reconciliation_preserves_known_charge() {
    reconciled_charge(true, false).await;
}
async fn reconciled_charge(redb: bool, limited: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-reconciled-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("db");
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let provider = Arc::new(Fixture {
        controls: Controls::default(),
        mode: Mode::KnownCost,
        calls: AtomicU64::new(0),
        attempts: Mutex::new(vec![]),
        retry_after_ms: 1001,
        resolve_nonaccepted: AtomicBool::new(false),
        exclude_alternate: AtomicBool::new(false),
    });
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
                AiBudget::new(
                    &service(),
                    "test-account",
                    UsdNanos(if limited { 10 } else { 100 }),
                )
                .unwrap(),
            )
            .idempotency("account-reconcile"),
        )
        .await
        .unwrap();
    let run = client.submit(&owner(), submission()).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let uncertain = client.view(&owner(), &run).await.unwrap();
    assert_eq!(uncertain.state(), &RunState::AwaitingReconciliation);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    provider.resolve_nonaccepted.store(true, Ordering::SeqCst);
    let waiting = client
        .resume(
            &owner(),
            &run,
            uncertain.revision(),
            "positive-nonacceptance",
        )
        .await
        .unwrap();
    assert!(matches!(waiting.state(), RunState::Waiting { .. }));
    let before = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    let money = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(money.settled(), UsdNanos(3));
    assert_eq!(money.entries().len(), 1);
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(&path, redb, clock.clone(), provider.clone(), grant, None);
    clock.0.store(2000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    let after = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.expires_at_unix_ms(), before.expires_at_unix_ms());
    if limited {
        assert_eq!(after.state(), &RunState::Failed);
        assert_eq!(after.failure(), Some(&AiError::BudgetExhausted));
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        let money = runtime
            .read::<AiBudget>(&service(), "test-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(money.entries().len(), 2);
        assert_eq!(money.settled(), UsdNanos(3));
        assert_eq!(money.reserved(), UsdNanos(0));
        assert_eq!(
            money.entries()[1].status(),
            &ReservationStatus::Settled {
                actual_cost: UsdNanos(0)
            }
        );
        runtime.shutdown().await.unwrap();
        return;
    }
    assert_eq!(after.state(), &RunState::Completed);
    assert_eq!(after.attempt_usage()[0].cost, Some(UsdNanos(3)));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        provider.attempts.lock().unwrap()[1].route().model(),
        "synthetic/alternate"
    );
    let money = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(money.entries().len(), 2);
    assert_eq!(money.settled(), UsdNanos(6));
    assert_eq!(money.reserved(), UsdNanos(0));
    runtime.shutdown().await.unwrap();
    drop(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_exhaustion_is_durable_and_keeps_original_evidence() {
    exhausted(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_exhaustion_is_durable_and_keeps_original_evidence() {
    exhausted(true, false).await;
}
async fn exhausted(redb: bool, stale: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "rom-failover-exhausted-{}-{}",
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
            .idempotency("account-exhausted"),
        )
        .await
        .unwrap();
    let mut input = submission();
    input.policy = RoutingPolicy::new(
        1,
        vec![],
        if stale {
            vec!["synthetic/model".into(), "synthetic/alternate".into()]
        } else {
            vec!["synthetic/model".into()]
        },
        Some(price()),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("test-account")
    .unwrap()
    .with_failover(rom_ai::FailoverPolicy::AdvanceOnConfirmedNonacceptance)
    .unwrap();
    let run = client.submit(&owner(), input).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let before = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    if stale {
        provider.exclude_alternate.store(true, Ordering::SeqCst);
    }
    clock.0.store(3000, Ordering::SeqCst);
    runtime.process_work(16).await.unwrap();
    let failed = client.view(&owner(), &run).await.unwrap();
    assert_eq!(failed.state(), &RunState::Failed);
    assert_eq!(failed.failure(), Some(&AiError::ProviderUnavailable));
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let (runtime, client) = build(&path, redb, clock.clone(), provider.clone(), grant, None);
    let after = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.state(), &RunState::Failed);
    assert_eq!(after.expires_at_unix_ms(), before.expires_at_unix_ms());
    assert_eq!(after.attempt_evidence(), before.attempt_evidence());
    assert_eq!(after.counters().generation_attempts(), 1);
    runtime.process_work(8).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let account = runtime
        .read::<AiBudget>(&service(), "test-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 1);
    assert_eq!(account.settled(), UsdNanos(0));
    assert_eq!(account.reserved(), UsdNanos(0));
    runtime.shutdown().await.unwrap();
    drop(client);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_known_cost_counts_before_successor_reservation() {
    reconciled_charge(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_known_cost_counts_before_successor_reservation() {
    reconciled_charge(true, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_failover_fresh_catalog_rejects_now_overpriced_alternate() {
    exhausted(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_failover_fresh_catalog_rejects_now_overpriced_alternate() {
    exhausted(true, true).await;
}

#[path = "failover/regressions.rs"]
mod regressions;
