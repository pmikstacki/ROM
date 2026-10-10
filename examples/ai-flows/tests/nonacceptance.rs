//! Unexecuted public-consumer regression draft. Real adapters; synthetic trusted Provider.
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
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[path = "nonacceptance/fault.rs"]
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
struct Authority(Arc<AtomicBool>);
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
struct Fixture {
    mode: Mode,
    calls: AtomicU64,
    retry_after_ms: u64,
}
impl Provider for Fixture {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async {
            CatalogSnapshot::new(
                "synthetic-paid",
                vec![CatalogModel::text(
                    "synthetic/model",
                    4096,
                    true,
                    true,
                    price(),
                )],
            )
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
            let ordinal = self.calls.fetch_add(1, Ordering::SeqCst);
            if ordinal > 0 {
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
        Box::pin(async { Ok(Reconciliation::Unresolved) })
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
            vec!["synthetic/model".into()],
            Some(price()),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("test-account")
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
    FlowHost::new(
        service(),
        provider,
        Arc::new(Authority(grant)),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .install(Runtime::builder().clock(clock))
    .unwrap()
    .build(storage, Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_exact_reference_survives_due_and_reopen() {
    journey(false, false, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_exact_reference_survives_due_and_reopen() {
    journey(true, false, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_generation_is_held_without_zero_settlement() {
    journey(false, false, Mode::ContradictoryGeneration).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_generation_is_held_without_zero_settlement() {
    journey(true, false, Mode::ContradictoryGeneration).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_uncertain_known_cost_stays_charged_without_retry() {
    journey(false, false, Mode::KnownCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_uncertain_known_cost_stays_charged_without_retry() {
    journey(true, false, Mode::KnownCost).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_lost_waiting_ack_preserves_receipt_and_single_successor() {
    journey(false, true, Mode::RequestReference).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_lost_waiting_ack_preserves_receipt_and_single_successor() {
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
        mode,
        calls: AtomicU64::new(0),
        retry_after_ms: 1001,
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
        assert!(matches!(
            held.entries()[0].status(),
            ReservationStatus::Reserved | ReservationStatus::Unknown
        ));
        assert_eq!(held.reserved(), UsdNanos(10));
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
async fn sqlite_nonacceptance_untrusted_usage_retains_reserved_ceiling_after_reopen() {
    journey(false, false, Mode::UntrustedCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_untrusted_usage_retains_reserved_ceiling_after_reopen() {
    journey(true, false, Mode::UntrustedCost).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_original_deadline_before_rounded_due_never_retransmits() {
    boundary(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_original_deadline_before_rounded_due_never_retransmits() {
    boundary(true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_nonacceptance_current_grant_revoked_before_retry_never_retransmits() {
    boundary(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_nonacceptance_current_grant_revoked_before_retry_never_retransmits() {
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
        mode: Mode::RequestReference,
        calls: AtomicU64::new(0),
        retry_after_ms: if expiry { 3001 } else { 1001 },
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
                vec!["synthetic/model".into()],
                Some(price()),
                RunLimits {
                    age_seconds: 3,
                    ..RunLimits::default()
                },
            )
            .unwrap()
            .with_budget_reference("test-account")
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
