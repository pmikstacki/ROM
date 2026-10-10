//! Public flow journeys over actual file-backed adapters; no live provider requests.
#[path = "work_flows/fault.rs"]
mod fault;
#[path = "work_flows/reconciliation.rs"]
mod reconciliation;
use rom::{Actor, PrincipalKind, Runtime, Storage};
use rom_ai::flow::{FlowAuthority, FlowHost, OwnerIdentity, RunState, Submission};
use rom_ai::{
    AiClock, AiFuture, AiResult, AttemptEvidence, CatalogModel, CatalogSnapshot, Completion,
    CompletionRequest, Deadline, Message, ModelPrice, OutputValidator, PreparedAttempt, Provider,
    Reconciliation, RoutingPolicy, RunLimits, Usage,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

struct ObservedAdapter {
    inner: Adapter,
    invalid: u8,
    reconciled: Mutex<Vec<AttemptEvidence>>,
}
impl Provider for ObservedAdapter {
    fn catalog<'a>(&'a self, deadline: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        self.inner.catalog(deadline)
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        self.inner.complete(attempt)
    }
    fn complete_observed<'a>(
        &'a self,
        attempt: &'a PreparedAttempt,
    ) -> AiFuture<'a, rom_ai::DispatchOutcome> {
        Box::pin(async move {
            let _ = self.inner.complete(attempt).await;
            Ok(rom_ai::DispatchOutcome::Uncertain {
                evidence: AttemptEvidence::new(
                    if self.invalid == 1 {
                        "foreign-attempt"
                    } else {
                        attempt.identity()
                    },
                    None,
                    Some("gen-original-header".into()),
                )?,
                usage: Usage {
                    input_tokens: Some(4),
                    cost: (self.invalid == 2).then_some(rom_ai::UsdNanos(11)),
                    ..Usage::default()
                },
                cause: rom_ai::AiError::UnknownOutcome,
            })
        })
    }
    fn reconcile<'a>(&'a self, evidence: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async move {
            self.reconciled.lock().unwrap().push(evidence.clone());
            Ok(Reconciliation::Unresolved)
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_unknown_retains_header_identity_on_restart_without_generation_retry() {
    observed_unknown(0, false, &[false, true]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_unknown_foreign_identity_or_amplified_usage_is_not_trusted() {
    observed_unknown(1, false, &[false, true]).await;
    observed_unknown(2, false, &[false, true]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_unknown_after_manual_hold_sqlite_retains_late_header_on_restart() {
    observed_unknown(0, true, &[false]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observed_unknown_after_manual_hold_redb_retains_late_header_on_restart() {
    observed_unknown(0, true, &[true]).await;
}

async fn observed_unknown(invalid: u8, race: bool, adapters: &[bool]) {
    use rom_ai::flow::{AiBudget, AiRun, ReservationStatus};
    for &redb in adapters {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let provider = Arc::new(ObservedAdapter {
            inner: if race {
                Adapter::new(5).completion_gate(release.clone())
            } else {
                Adapter::new(5)
            },
            invalid,
            reconciled: Mutex::new(Vec::new()),
        });
        let build = |db: &Db| {
            FlowHost::new(
                service(),
                provider.clone(),
                Arc::new(Authority),
                Arc::new(Validator),
                clock.clone(),
            )
            .unwrap()
            .install(Runtime::builder().clock(clock.clone()))
            .unwrap()
            .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap()
        };
        let (runtime, client) = build(&db);
        runtime
            .execute(
                &service(),
                rom::Command::create(
                    "observed-window",
                    AiBudget::new(&service(), "observed-window", rom_ai::UsdNanos(20)).unwrap(),
                )
                .idempotency("grant-observed-window"),
            )
            .await
            .unwrap();
        let mut input = submission("observed-unknown");
        input.policy = RoutingPolicy::new(
            1,
            Vec::new(),
            vec!["paid".into()],
            Some(ModelPrice::new(
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(10),
            )),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("observed-window")
        .unwrap();
        let run = client.submit(&owner(), input).await.unwrap();
        let worker = if race {
            let worker = runtime.start_work().unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                while provider.inner.calls.load(Ordering::SeqCst) != 1 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            let view = client.view(&owner(), &run).await.unwrap();
            let held = client
                .resume(&owner(), &run, view.revision(), "preobserved-resume")
                .await
                .unwrap();
            assert_eq!(held.state(), &RunState::AwaitingReconciliation);
            let before = runtime
                .read::<AiRun>(&service(), &run.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(before.attempt_evidence()[0].generation_id(), None);
            release.add_permits(1);
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                while db.storage.reaction_records().unwrap().iter().any(|work| {
                    matches!(
                        work.state,
                        rom::WorkState::Leased { .. } | rom::WorkState::Pending
                    )
                }) {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
            Some(worker)
        } else {
            runtime.process_work(8).await.unwrap();
            None
        };
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(record.state(), &RunState::AwaitingReconciliation);
        assert_eq!(
            record.attempt_evidence()[0].generation_id(),
            (invalid == 0).then_some("gen-original-header")
        );
        assert_eq!(
            record.attempt_evidence()[0].attempt_id(),
            record.active_attempt().unwrap().prepared().identity()
        );
        assert_eq!(
            record.attempt_usage()[0].input_tokens,
            (invalid == 0).then_some(4)
        );
        let account = runtime
            .read::<AiBudget>(&service(), "observed-window")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(account.reserved(), rom_ai::UsdNanos(10));
        assert_eq!(account.settled(), rom_ai::UsdNanos(0));
        assert_eq!(account.entries()[0].status(), &ReservationStatus::Unknown);
        runtime.shutdown().await.unwrap();
        if let Some(worker) = worker {
            worker.join().await.unwrap();
        }
        drop(client);
        drop(runtime);
        let db = db.reopen();
        let (runtime, client) = build(&db);
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &run).await.unwrap();
        client
            .resume(&owner(), &run, view.revision(), "observed-resume")
            .await
            .unwrap();
        assert_eq!(provider.inner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            provider.reconciled.lock().unwrap().len(),
            if race { 2 } else { 1 }
        );
        assert_eq!(
            provider
                .reconciled
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .generation_id(),
            (invalid == 0).then_some("gen-original-header")
        );
        assert!(
            client
                .view(&owner(), &run)
                .await
                .unwrap()
                .output()
                .is_none()
        );
        runtime.shutdown().await.unwrap();
    }
}

struct Clock(AtomicU64);
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl rom::Clock for Clock {
    fn now(&self) -> u64 {
        self.now_unix_ms() / 1000
    }
}
fn owner() -> Actor {
    Actor::trusted("flow-tests", "owner")
}
fn service() -> Actor {
    Actor::trusted("flow-tests", "service").with_kind(PrincipalKind::Service)
}
struct Authority;
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        self.inspect(actor, &OwnerIdentity::from_actor(&owner())?)?;
        OwnerIdentity::from_actor(actor)
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor) && identity.matches(&owner()) {
            Ok(())
        } else {
            Err(rom_ai::AiError::Denied)
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
        if identity.matches(&owner()) {
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
        if identity.matches(actor) {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
struct GrantAuthority(Arc<std::sync::atomic::AtomicBool>);
impl FlowAuthority for GrantAuthority {
    fn submit(&self, actor: &Actor, input: &Submission) -> AiResult<OwnerIdentity> {
        Authority.submit(actor, input)
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.inspect(actor, identity)
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.cancel(actor, identity)
    }
    fn resolve(
        &self,
        identity: &OwnerIdentity,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        Authority.resolve(identity, reads)
    }
    fn attempt(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        prepared: &PreparedAttempt,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        if !self.0.load(Ordering::SeqCst) {
            return Err(rom::Error::Denied);
        }
        Authority.attempt(actor, identity, prepared, reads)
    }
}
struct SlowAuthority(Arc<std::sync::atomic::AtomicBool>);
impl FlowAuthority for SlowAuthority {
    fn submit(&self, actor: &Actor, input: &Submission) -> AiResult<OwnerIdentity> {
        Authority.submit(actor, input)
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.inspect(actor, identity)
    }
    fn cancel(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        Authority.cancel(actor, identity)
    }
    fn resolve(
        &self,
        identity: &OwnerIdentity,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if self.0.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
        Authority.resolve(identity, reads)
    }
    fn attempt(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        prepared: &PreparedAttempt,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        Authority.attempt(actor, identity, prepared, reads)
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        Ok(value.clone())
    }
}
struct RejectOutput;
impl OutputValidator for RejectOutput {
    fn validate(&self, _: &serde_json::Value) -> AiResult<serde_json::Value> {
        Err(rom_ai::AiError::InvalidOutput)
    }
}
struct Observer(Mutex<Vec<rom_ai::ObservationKind>>);
impl rom_ai::FlowObserver for Observer {
    fn observe(&self, event: &rom_ai::FlowObservation) {
        self.0.lock().unwrap().push(event.category());
        if event.category() == rom_ai::ObservationKind::Prepared {
            panic!("bounded observer fixture");
        }
    }
}
struct Adapter {
    calls: AtomicU64,
    lookups: AtomicU64,
    mode: u8,
    attempts: Mutex<Vec<PreparedAttempt>>,
    lookup_result: u8,
    lookup_release: Option<Arc<tokio::sync::Semaphore>>,
    completion_release: Option<Arc<tokio::sync::Semaphore>>,
    jump: Option<(Arc<Clock>, u64)>,
    invalid: u8,
}
impl Adapter {
    fn new(mode: u8) -> Self {
        Self {
            calls: AtomicU64::new(0),
            lookups: AtomicU64::new(0),
            mode,
            attempts: Mutex::new(vec![]),
            lookup_result: 0,
            lookup_release: None,
            completion_release: None,
            jump: None,
            invalid: 0,
        }
    }
    fn reconciliation(mut self, result: u8) -> Self {
        self.lookup_result = result;
        self
    }
    fn lookup_gate(mut self, gate: Arc<tokio::sync::Semaphore>) -> Self {
        self.lookup_release = Some(gate);
        self
    }
    fn completion_gate(mut self, gate: Arc<tokio::sync::Semaphore>) -> Self {
        self.completion_release = Some(gate);
        self
    }
    fn clock_jump(mut self, clock: Arc<Clock>, now: u64) -> Self {
        self.jump = Some((clock, now));
        self
    }
    fn invalid_completion(mut self, invalid: u8) -> Self {
        self.invalid = invalid;
        self
    }
}
impl Provider for Adapter {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async move {
            if self.mode >= 4 {
                CatalogSnapshot::new(
                    "fixture-paid",
                    vec![CatalogModel::text(
                        "paid",
                        262144,
                        true,
                        true,
                        ModelPrice::new(
                            rom_ai::UsdNanos(0),
                            rom_ai::UsdNanos(0),
                            rom_ai::UsdNanos(10),
                        ),
                    )],
                )
            } else {
                CatalogSnapshot::new(
                    "fixture",
                    vec![CatalogModel::text(
                        "free",
                        262144,
                        true,
                        true,
                        ModelPrice::free(),
                    )],
                )
            }
        })
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            let previous = self.calls.fetch_add(1, Ordering::SeqCst);
            self.attempts.lock().unwrap().push(attempt.clone());
            if self.invalid > 0 {
                let usage = Usage {
                    input_tokens: Some(999),
                    cost: Some(rom_ai::UsdNanos(999)),
                    ..Default::default()
                };
                let evidence = AttemptEvidence::new(
                    if self.invalid == 2 {
                        "foreign-attempt"
                    } else {
                        attempt.identity()
                    },
                    Some("untrusted-provider-reference".into()),
                    None,
                )?;
                if self.invalid == 3 {
                    return Ok(Completion::ToolCalls {
                        calls: vec![rom_ai::ToolCall::new(
                            "call",
                            "private-tool",
                            serde_json::json!({}),
                        )?],
                        usage,
                        evidence,
                    });
                }
                return Ok(Completion::Output {
                    value: if self.invalid == 1 {
                        serde_json::json!({"answer":"x".repeat(65537)})
                    } else {
                        serde_json::json!({"answer":"wrong"})
                    },
                    usage,
                    evidence,
                });
            }
            if let Some(gate) = &self.completion_release {
                gate.acquire().await.unwrap().forget();
            }
            if let Some((clock, now)) = &self.jump {
                clock.0.store(*now, Ordering::SeqCst);
            }
            if self.mode == 6 {
                return Ok(Completion::ToolCalls {
                    calls: vec![rom_ai::ToolCall::new(
                        "call-original",
                        "requested-tool",
                        serde_json::json!({}),
                    )?],
                    usage: Usage {
                        input_tokens: Some(4),
                        output_tokens: Some(1),
                        cost: Some(rom_ai::UsdNanos(3)),
                        ..Default::default()
                    },
                    evidence: AttemptEvidence::new(
                        attempt.identity(),
                        None,
                        Some("gen-requested-tool".into()),
                    )?,
                });
            }
            if self.mode == 1 || self.mode == 5 || (self.mode == 3 && previous == 0) {
                return Err(rom_ai::AiError::UnknownOutcome);
            }
            if self.mode == 2 && previous == 0 {
                return Err(rom_ai::AiError::RateLimited {
                    retry_after_ms: 1001,
                });
            }
            Completion::output(
                serde_json::json!({"answer":"complete"}),
                Usage {
                    input_tokens: Some(4),
                    output_tokens: Some(1),
                    cost: Some(rom_ai::UsdNanos(if self.mode == 4 { 3 } else { 0 })),
                    ..Default::default()
                },
                AttemptEvidence::new(attempt.identity(), None, None)?,
            )
        })
    }
    fn reconcile<'a>(&'a self, evidence: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async move {
            self.lookups.fetch_add(1, Ordering::SeqCst);
            if let Some(gate) = &self.lookup_release {
                gate.acquire().await.unwrap().forget();
            }
            match self.lookup_result {
                1 => Ok(Reconciliation::Accepted {
                    completion: Completion::output(
                        serde_json::json!({"answer":"reconciled"}),
                        Usage::default(),
                        evidence.clone(),
                    )?,
                }),
                2 => Ok(Reconciliation::NotAccepted),
                _ => Ok(Reconciliation::Unresolved),
            }
        })
    }
}
fn submission(id: &str) -> Submission {
    Submission {
        id: id.into(),
        idempotency: format!("submit-{id}"),
        request: CompletionRequest::new(vec![Message::user("private question")], 128).unwrap(),
        policy: RoutingPolicy::new(1, vec!["free".into()], vec![], None, RunLimits::default())
            .unwrap(),
    }
}

struct Db {
    storage: Arc<dyn Storage>,
    path: std::path::PathBuf,
    redb: bool,
}
impl Db {
    fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-ai-flows-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        Self::open(redb, path)
    }
    fn open(redb: bool, path: std::path::PathBuf) -> Self {
        let file = path.join("database");
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&file).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&file).unwrap())
        };
        Self {
            storage,
            path,
            redb,
        }
    }
    fn reopen(mut self) -> Self {
        let path = std::mem::take(&mut self.path);
        let redb = self.redb;
        drop(self);
        Self::open(redb, path)
    }
}
impl Drop for Db {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn build(db: &Db, clock: Arc<Clock>, adapter: Arc<Adapter>) -> (Runtime, rom_ai::flow::FlowClient) {
    FlowHost::new(
        service(),
        adapter,
        Arc::new(Authority),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .install(Runtime::builder().clock(clock))
    .unwrap()
    .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_discovery_failure_is_visible_and_fresh_retry_preserves_original_history() {
    preparation_failure(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_discovery_failure_is_visible_and_fresh_retry_preserves_original_history() {
    preparation_failure(true, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_current_budget_denial_is_visible_without_charging_or_hidden_retry() {
    preparation_failure(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_current_budget_denial_is_visible_without_charging_or_hidden_retry() {
    preparation_failure(true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preparation_failure_action_retains_exact_orphan_reservation_and_receipt_identity() {
    use rom_ai::flow::{
        AiBudget, AiRun, FAIL_PREPARATION, FailPreparation, RESERVE_BUDGET, ReservationKey,
        ReserveBudget,
    };
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let provider = Arc::new(Adapter::new(4));
        let (runtime, client) = build(&db, clock, provider.clone());
        runtime
            .execute(
                &service(),
                rom::Command::create(
                    "orphan-account",
                    AiBudget::new(&service(), "orphan-account", rom_ai::UsdNanos(20)).unwrap(),
                )
                .idempotency("orphan-account-create"),
            )
            .await
            .unwrap();
        let mut input = submission("preparation-orphan");
        input.policy = RoutingPolicy::new(
            1,
            Vec::new(),
            vec!["paid".into()],
            Some(ModelPrice::new(
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(10),
            )),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("orphan-account")
        .unwrap();
        let request = input.request.clone();
        let policy = input.policy.clone();
        let handle = client.submit(&owner(), input).await.unwrap();
        // Materialize creation and commit admission without running the generation channel callback.
        runtime.process_work(2).await.unwrap();
        let admitted = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(admitted.state(), &RunState::Queued);
        assert_eq!(
            serde_json::to_value(&admitted).unwrap()["queue_admitted"],
            true
        );
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        let catalog = provider
            .catalog(Deadline::remaining(1000, 1000, 19_000).unwrap())
            .await
            .unwrap();
        let route = rom_ai::choose(
            &policy,
            &catalog,
            &rom_ai::RouteCursor::new(1, catalog.identity()).unwrap(),
            &request,
        )
        .unwrap();
        let key = ReservationKey::new("orphan-account", &handle.0, 1, 1, 1).unwrap();
        let prepared = PreparedAttempt::new(
            key.attempt_identity(),
            request,
            policy,
            route,
            Deadline::remaining(1000, 1000, 19_000).unwrap(),
        )
        .unwrap();
        let reservation = ReserveBudget::new(key, prepared).unwrap();
        let account = runtime
            .read::<AiBudget>(&service(), "orphan-account")
            .await
            .unwrap();
        runtime
            .execute(
                &service(),
                rom::Command::action("orphan-account", RESERVE_BUDGET, reservation)
                    .at_revision(account.revision)
                    .idempotency("orphan-reserve"),
            )
            .await
            .unwrap();
        let original_account = runtime
            .read::<AiBudget>(&service(), "orphan-account")
            .await
            .unwrap();
        let original_run = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        assert_eq!(
            original_run.value.as_ref().unwrap().record().unwrap(),
            admitted
        );
        let head = db.storage.journal_head("ai_runs").unwrap();
        let failure = FailPreparation::new(1000, rom_ai::AiError::ProviderUnavailable).unwrap();
        assert!(matches!(
            runtime
                .execute(
                    &owner(),
                    rom::Command::action(&handle.0, FAIL_PREPARATION, failure.clone())
                        .at_revision(original_run.revision)
                        .idempotency("owner-forged-preparation-failure")
                )
                .await,
            Err(rom::Error::Denied)
        ));
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), head);
        assert_eq!(
            runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .revision,
            original_run.revision
        );
        let execute = |input| {
            rom::Command::action(&handle.0, FAIL_PREPARATION, input)
                .at_revision(original_run.revision)
                .idempotency("orphan-preparation-failure")
        };
        runtime
            .execute(&service(), execute(failure.clone()))
            .await
            .unwrap();
        let committed_head = db.storage.journal_head("ai_runs").unwrap();
        runtime.execute(&service(), execute(failure)).await.unwrap();
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), committed_head);
        assert!(matches!(
            runtime
                .execute(
                    &service(),
                    execute(FailPreparation::new(1000, rom_ai::AiError::Denied).unwrap())
                )
                .await,
            Err(rom::Error::IdentityMismatch)
        ));
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), committed_head);
        runtime.process_work(8).await.unwrap();
        let account = runtime
            .read::<AiBudget>(&service(), "orphan-account")
            .await
            .unwrap();
        assert_eq!(account.revision, original_account.revision);
        assert_eq!(
            account.value.unwrap().record().unwrap(),
            original_account.value.unwrap().record().unwrap()
        );
        let view = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(view.state(), &RunState::Failed);
        assert!(view.output().is_none());
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        let record = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert!(record.active_attempt().is_none());
        assert!(record.attempt_evidence().is_empty());
        assert_eq!(record.counters().generation_attempts(), 0);
        runtime.shutdown().await.unwrap();
    }
}
async fn preparation_failure(redb: bool, paid: bool) {
    use rom_ai::flow::{AiBudget, AiRun};
    let db = Db::new(redb);
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let grant = Arc::new(std::sync::atomic::AtomicBool::new(!paid));
    let build = |db: &Db, adapter: Arc<Adapter>| {
        FlowHost::new(
            service(),
            adapter,
            Arc::new(GrantAuthority(grant.clone())),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock.clone()))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
    };
    let submission = |id| {
        let mut input = submission(id);
        if paid {
            input.policy = RoutingPolicy::new(
                1,
                Vec::new(),
                vec!["paid".into()],
                Some(ModelPrice::new(
                    rom_ai::UsdNanos(0),
                    rom_ai::UsdNanos(0),
                    rom_ai::UsdNanos(10),
                )),
                RunLimits::default(),
            )
            .unwrap()
            .with_budget_reference("preparation-account")
            .unwrap();
        }
        input
    };
    let provider = Arc::new(Adapter::new(if paid { 4 } else { 7 }));
    let (runtime, client) = build(&db, provider.clone());
    let account = if paid {
        "preparation-account"
    } else {
        "rom-ai-free-v1"
    };
    if paid {
        runtime
            .execute(
                &service(),
                rom::Command::create(
                    account,
                    AiBudget::new(&service(), account, rom_ai::UsdNanos(20)).unwrap(),
                )
                .idempotency("preparation-account-create"),
            )
            .await
            .unwrap();
    }
    let run = client
        .submit(&owner(), submission("preparation-old"))
        .await
        .unwrap();
    runtime.process_work(8).await.unwrap();
    let failed = client.view(&owner(), &run).await.unwrap();
    assert_eq!(
        failed.state(),
        &RunState::Failed,
        "permanent preparation failure must not strand an owner-visible Queued run"
    );
    assert_eq!(
        failed.failure(),
        Some(&if paid {
            rom_ai::AiError::Denied
        } else {
            rom_ai::AiError::ProviderUnavailable
        })
    );
    assert_eq!(failed.counters().generation_attempts(), 0);
    assert!(failed.output().is_none());
    let original = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert!(original.active_attempt().is_none());
    let budget = runtime
        .read::<AiBudget>(&service(), account)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert!(budget.entries().is_empty());
    assert_eq!(budget.reserved(), rom_ai::UsdNanos(0));
    assert_eq!(budget.settled(), rom_ai::UsdNanos(0));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let db = db.reopen();
    grant.store(true, Ordering::SeqCst);
    let recovered_provider = Arc::new(Adapter::new(if paid { 4 } else { 0 }));
    let (runtime, client) = build(&db, recovered_provider.clone());
    let replay = client
        .submit(&owner(), submission("preparation-old"))
        .await
        .unwrap();
    assert_eq!(replay, run);
    runtime.process_work(8).await.unwrap();
    assert_eq!(recovered_provider.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        client.view(&owner(), &run).await.unwrap().state(),
        &RunState::Failed
    );
    let fresh = client
        .submit(&owner(), submission("preparation-fresh"))
        .await
        .unwrap();
    assert_ne!(fresh, run);
    runtime.process_work(8).await.unwrap();
    assert_eq!(recovered_provider.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        client.view(&owner(), &fresh).await.unwrap().state(),
        &RunState::Completed
    );
    let retained = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(retained, original);
    runtime.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observations_follow_real_committed_stages_and_panics_do_not_break_delivery() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let observer = Arc::new(Observer(Mutex::new(vec![])));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .observer(observer.clone())
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let run = client
            .submit(&owner(), submission("observed"))
            .await
            .unwrap();
        assert!(observer.0.lock().unwrap().is_empty());
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::Completed
        );
        assert_eq!(
            *observer.0.lock().unwrap(),
            vec![
                rom_ai::ObservationKind::Prepared,
                rom_ai::ObservationKind::Completed
            ]
        );
        runtime.process_work(8).await.unwrap();
        assert_eq!(observer.0.lock().unwrap().len(), 2);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_io_capacity_reports_finite_overload_without_provider_work() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .limits(rom::Limits {
            actions: 1,
            io_jobs: 1,
            command_bytes: 1024 * 1024,
            ..Default::default()
        })
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let run = client
            .submit(&owner(), submission("capacity-one"))
            .await
            .unwrap();
        let entered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let marker = entered.clone();
        let other = runtime.clone();
        let task = tokio::spawn(async move {
            other
                .establish_actor(move |_| {
                    marker.store(true, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    Ok(owner())
                })
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !entered.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            tokio::time::timeout(
                std::time::Duration::from_millis(100),
                client.view(&owner(), &run)
            )
            .await
            .unwrap()
            .unwrap_err(),
            rom_ai::AiError::BudgetExhausted
        );
        task.await.unwrap().unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_output_retains_original_call_evidence_without_publication_or_retry() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(RejectOutput),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let run = client
            .submit(&owner(), submission("bad-output"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert!(view.output().is_none());
        let record = runtime
            .read::<rom_ai::flow::AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            record.attempt_evidence()[0].attempt_id(),
            record.active_attempt().unwrap().prepared().identity()
        );
        assert_eq!(record.attempt_usage()[0].cost, Some(rom_ai::UsdNanos(0)));
        runtime.process_work(8).await.unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restored_executing_attempt_is_held_and_reconciled_without_generation() {
    for redb in [false, true] {
        use rom_ai::flow::{
            AiBudget, AiRun, PREPARE_RUN, PrepareRun, RESERVE_BUDGET, ReservationKey,
            ReserveBudget, START_RUN, StartRun,
        };
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let input = submission("restored-started");
        let run = client
            .submit(&owner(), submission("restored-started"))
            .await
            .unwrap();
        runtime.process_work(2).await.unwrap();
        let deadline = Deadline::remaining(1000, 1000, 301000).unwrap();
        let catalog = adapter.catalog(deadline).await.unwrap();
        let route = rom_ai::choose(
            &input.policy,
            &catalog,
            &rom_ai::RouteCursor::new(1, catalog.identity()).unwrap(),
            &input.request,
        )
        .unwrap();
        let key = ReservationKey::new("rom-ai-free-v1", &run.0, 1, 1, 1).unwrap();
        let attempt = PreparedAttempt::new(
            key.attempt_identity(),
            input.request,
            input.policy,
            route,
            deadline,
        )
        .unwrap();
        let reserve = ReserveBudget::new(key, attempt).unwrap();
        let account = runtime
            .read::<AiBudget>(&service(), "rom-ai-free-v1")
            .await
            .unwrap();
        runtime
            .execute(
                &service(),
                rom::Command::action("rom-ai-free-v1", RESERVE_BUDGET, reserve.clone())
                    .at_revision(account.revision)
                    .idempotency("fixture-reserve"),
            )
            .await
            .unwrap();
        let queued = runtime.read::<AiRun>(&service(), &run.0).await.unwrap();
        let prepared = runtime
            .execute(
                &service(),
                rom::Command::action(
                    &run.0,
                    PREPARE_RUN,
                    PrepareRun::new(reserve.entry().clone(), 1000).unwrap(),
                )
                .at_revision(queued.revision)
                .idempotency("fixture-prepare"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &service(),
                rom::Command::action(&run.0, START_RUN, StartRun::new(1, 1000).unwrap())
                    .at_revision(prepared.revision)
                    .idempotency("fixture-start"),
            )
            .await
            .unwrap();
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        let db = db.reopen();
        let (runtime, client) = build(&db, clock, adapter.clone());
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::Executing);
        let view = client
            .resume(&owner(), &run, view.revision(), "recover-executing")
            .await
            .unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        runtime.process_work(8).await.unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_owner_is_checked_after_waiting_for_shared_provider_capacity() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(1).lookup_gate(gate.clone()));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .limits(rom::Limits {
            io_jobs: 2,
            command_bytes: 1024 * 1024,
            ..Default::default()
        })
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let client = Arc::new(client);
        let run = client
            .submit(&owner(), submission("queued-current-owner"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let mut tasks = vec![];
        let mut queued_revision = 0;
        for index in 0..3_u64 {
            let revision = client.view(&owner(), &run).await.unwrap().revision();
            if index == 2 {
                queued_revision = revision;
            }
            let client = client.clone();
            let lookup_run = run.clone();
            tasks.push(tokio::spawn(async move {
                client
                    .resume(&owner(), &lookup_run, revision, &format!("queued-{index}"))
                    .await
            }));
            tokio::time::timeout(std::time::Duration::from_secs(1), async {
                loop {
                    let record = runtime
                        .read::<rom_ai::flow::AiRun>(&service(), &run.0)
                        .await
                        .unwrap()
                        .value
                        .unwrap()
                        .record()
                        .unwrap();
                    if record.counters().ticks() == index as u32 + 2 {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            if index < 2 {
                tokio::time::timeout(std::time::Duration::from_secs(1), async {
                    while adapter.lookups.load(Ordering::SeqCst) != index + 1 {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
            }
        }
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 2);
        runtime.revoke(&owner());
        gate.add_permits(2);
        let queued = tasks.pop().unwrap().await.unwrap();
        assert!(matches!(
            queued.unwrap_err(),
            rom_ai::AiError::Denied | rom_ai::AiError::BudgetExhausted
        ));
        for task in tasks {
            let _ = task.await.unwrap();
        }
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 2);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            client
                .resume(&owner(), &run, queued_revision, "queued-2")
                .await
                .unwrap_err(),
            rom_ai::AiError::Denied
        );
        assert_eq!(
            client.view(&owner(), &run).await.unwrap_err(),
            rom_ai::AiError::Denied
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn competing_operation_nonces_do_not_repeat_provider_lookup() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(1).lookup_gate(gate.clone()));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let client = Arc::new(client);
        let run = client
            .submit(&owner(), submission("same-operation"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let revision = client.view(&owner(), &run).await.unwrap().revision();
        let mut tasks = vec![];
        for _ in 0..2 {
            let client = client.clone();
            let run = run.clone();
            tasks.push(tokio::spawn(async move {
                client.resume(&owner(), &run, revision, "same-key").await
            }));
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while adapter.lookups.load(Ordering::SeqCst) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        gate.add_permits(1);
        for task in tasks {
            if let Err(error) = task.await.unwrap() {
                assert_eq!(error, rom_ai::AiError::Conflict);
            }
        }
        let view = client
            .resume(&owner(), &run, revision, "same-key")
            .await
            .unwrap();
        assert_eq!(view.counters().ticks(), 2);
        assert_eq!(view.counters().generation_attempts(), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paid_known_cost_settles_exact_account_and_unknown_retains_ceiling_after_expiry() {
    for redb in [false, true] {
        for mode in [4, 5] {
            use rom_ai::flow::{AiBudget, ReservationStatus};
            let db = Db::new(redb);
            let clock = Arc::new(Clock(AtomicU64::new(1000)));
            let adapter = Arc::new(Adapter::new(mode));
            let (runtime, client) = build(&db, clock.clone(), adapter.clone());
            runtime
                .execute(
                    &service(),
                    rom::Command::create(
                        "paid-window",
                        AiBudget::new(&service(), "paid-window", rom_ai::UsdNanos(20)).unwrap(),
                    )
                    .idempotency("grant-paid-window"),
                )
                .await
                .unwrap();
            let mut input = submission("paid-run");
            input.policy = RoutingPolicy::new(
                1,
                vec![],
                vec!["paid".into()],
                Some(ModelPrice::new(
                    rom_ai::UsdNanos(0),
                    rom_ai::UsdNanos(0),
                    rom_ai::UsdNanos(10),
                )),
                RunLimits::default(),
            )
            .unwrap()
            .with_budget_reference("paid-window")
            .unwrap();
            let run = client.submit(&owner(), input).await.unwrap();
            runtime.process_work(8).await.unwrap();
            let account = runtime
                .read::<AiBudget>(&service(), "paid-window")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(account.entries().len(), 1);
            assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
            if mode == 4 {
                assert_eq!(
                    client.view(&owner(), &run).await.unwrap().state(),
                    &RunState::Completed
                );
                assert_eq!(account.reserved(), rom_ai::UsdNanos(0));
                assert_eq!(account.settled(), rom_ai::UsdNanos(3));
                assert_eq!(
                    account.entries()[0].status(),
                    &ReservationStatus::Settled {
                        actual_cost: rom_ai::UsdNanos(3)
                    }
                );
            } else {
                assert_eq!(account.reserved(), rom_ai::UsdNanos(10));
                assert_eq!(account.entries()[0].status(), &ReservationStatus::Unknown);
                clock.0.store(302000, Ordering::SeqCst);
                runtime.process_work(8).await.unwrap();
                let after = runtime
                    .read::<AiBudget>(&service(), "paid-window")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .record()
                    .unwrap();
                assert_eq!(after, account);
                assert_eq!(
                    client.view(&owner(), &run).await.unwrap().state(),
                    &RunState::AwaitingReconciliation
                );
            }
            runtime.shutdown().await.unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupted_known_settlement_recovers_from_durable_run_without_new_generation() {
    settlement_recovery(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lost_settlement_ack_recovers_without_extra_cost_events_or_generation() {
    settlement_recovery(true).await;
}
async fn settlement_recovery(after_commit: bool) {
    for redb in [false, true] {
        use rom_ai::flow::AiBudget;
        let mut db = Db::new(redb);
        let native = db.storage.clone();
        let fault = Arc::new(fault::SettlementFault {
            inner: native,
            armed: std::sync::atomic::AtomicBool::new(true),
            after_commit,
        });
        db.storage = fault;
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(4));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        runtime
            .execute(
                &service(),
                rom::Command::create(
                    "paid-window",
                    AiBudget::new(&service(), "paid-window", rom_ai::UsdNanos(20)).unwrap(),
                )
                .idempotency("grant-paid-window"),
            )
            .await
            .unwrap();
        let mut input = submission("paid-interrupted");
        input.policy = RoutingPolicy::new(
            1,
            vec![],
            vec!["paid".into()],
            Some(ModelPrice::new(
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(10),
            )),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("paid-window")
        .unwrap();
        let run = client.submit(&owner(), input).await.unwrap();
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::Completed
        );
        assert_eq!(
            runtime
                .read::<AiBudget>(&service(), "paid-window")
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap()
                .reserved(),
            rom_ai::UsdNanos(if after_commit { 0 } else { 10 })
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        let original_head = db.storage.journal_head("ai_budgets").unwrap();
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        let db = db.reopen();
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        clock.0.store(3000, Ordering::SeqCst);
        runtime.process_work(8).await.unwrap();
        let account = runtime
            .read::<AiBudget>(&service(), "paid-window")
            .await
            .unwrap();
        assert_eq!(
            account.value.as_ref().unwrap().record().unwrap().settled(),
            rom_ai::UsdNanos(3)
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::Completed
        );
        if after_commit {
            assert_eq!(
                db.storage.journal_head("ai_budgets").unwrap(),
                original_head
            );
        }
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            runtime
                .read::<AiBudget>(&service(), "paid-window")
                .await
                .unwrap()
                .revision,
            account.revision
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lookup_admission_is_finite_and_dropped_pending_operations_never_replay_http() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(1).lookup_gate(gate.clone()));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let client = Arc::new(client);
        let handle = client
            .submit(&owner(), submission("bounded-lookups"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let mut tasks = vec![];
        let mut originals = vec![];
        for index in 0..8_u64 {
            let revision = client.view(&owner(), &handle).await.unwrap().revision();
            let key = format!("lookup-{index}");
            originals.push((revision, key.clone()));
            let client = client.clone();
            let run = handle.clone();
            tasks.push(tokio::spawn(async move {
                client.resume(&owner(), &run, revision, &key).await
            }));
            tokio::time::timeout(std::time::Duration::from_secs(1), async {
                while adapter.lookups.load(Ordering::SeqCst) != index + 1 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        let before = client.view(&owner(), &handle).await.unwrap();
        let head = db.storage.journal_head("ai_runs").unwrap();
        assert_eq!(
            client
                .resume(&owner(), &handle, before.revision(), "overflow")
                .await
                .unwrap_err(),
            rom_ai::AiError::BudgetExhausted
        );
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), head);
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().revision(),
            before.revision()
        );
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 8);
        for task in tasks {
            task.abort();
            let _ = task.await;
        }
        let (revision, key) = &originals[0];
        client
            .resume(&owner(), &handle, *revision, key)
            .await
            .unwrap();
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 8);
        gate.add_permits(1);
        let revision = client.view(&owner(), &handle).await.unwrap().revision();
        client
            .resume(&owner(), &handle, revision, "fresh-after-drop")
            .await
            .unwrap();
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 9);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoked_attempt_grant_denies_generation_and_lookup_without_new_reservations() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let grant = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(GrantAuthority(grant.clone())),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let held = client
            .submit(&owner(), submission("grant-held"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let before = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), "rom-ai-free-v1")
            .await
            .unwrap();
        grant.store(false, Ordering::SeqCst);
        let view = client.view(&owner(), &held).await.unwrap();
        assert_eq!(
            client
                .resume(&owner(), &held, view.revision(), "revoked-grant")
                .await
                .unwrap_err(),
            rom_ai::AiError::Denied
        );
        let queued = client
            .submit(&owner(), submission("grant-queued"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let denied = client.view(&owner(), &queued).await.unwrap();
        assert_eq!(denied.state(), &RunState::Failed);
        assert_eq!(denied.failure(), Some(&rom_ai::AiError::Denied));
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        let after = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), "rom-ai-free-v1")
            .await
            .unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(
            after.value.unwrap().record().unwrap(),
            before.value.unwrap().record().unwrap()
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconciliation_deadline_includes_current_authority_setup_before_http() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let slow = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(SlowAuthority(slow.clone())),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let run = client
            .submit(&owner(), submission("slow-lookup-auth"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &run).await.unwrap();
        slow.store(true, Ordering::SeqCst);
        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(2500),
            client.resume(&owner(), &run, view.revision(), "slow-auth"),
        )
        .await
        .expect("host lookup deadline includes trusted authorization setup");
        assert_eq!(outcome.unwrap_err(), rom_ai::AiError::UnknownOutcome);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        slow.store(false, Ordering::SeqCst);
        client
            .resume(&owner(), &run, view.revision(), "slow-auth")
            .await
            .unwrap();
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_started_attempt_holds_and_unresolved_resume_never_retransmits() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let handle = client
            .submit(&owner(), submission("unknown"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert_eq!(view.counters().generation_attempts(), 1);
        let before = db.storage.reaction_records().unwrap();
        assert!(
            before
                .iter()
                .any(|work| work.state == rom::WorkState::AwaitingReconciliation)
        );
        clock.0.store(302000, Ordering::SeqCst);
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            client
                .resume(&owner(), &handle, view.revision(), "resume-unknown")
                .await
                .unwrap()
                .state(),
            &RunState::AwaitingReconciliation
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            client
                .view(&owner(), &handle)
                .await
                .unwrap()
                .counters()
                .generation_attempts(),
            1
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rate_limit_resumes_automatically_at_durable_due() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(2));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let handle = client
            .submit(&owner(), submission("rate-limit"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Waiting {
                retry_at_unix_ms: 3000
            }
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        clock.0.store(2999, Ordering::SeqCst);
        runtime.process_work(8).await.unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        clock.0.store(3000, Ordering::SeqCst);
        let worker = runtime.start_work().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 2);
        {
            let attempts = adapter.attempts.lock().unwrap();
            assert_ne!(attempts[0].identity(), attempts[1].identity());
            assert_eq!(attempts[0].route(), attempts[1].route());
        }
        assert_eq!(
            client
                .view(&owner(), &handle)
                .await
                .unwrap()
                .counters()
                .generation_attempts(),
            2
        );
        runtime.shutdown().await.unwrap();
        worker.join().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn principal_revocation_blocks_view_replay_and_dispatch() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let handle = client
            .submit(&owner(), submission("revoked"))
            .await
            .unwrap();
        runtime.revoke(&owner());
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap_err(),
            rom_ai::AiError::Denied
        );
        assert_eq!(
            client
                .submit(&owner(), submission("revoked"))
                .await
                .unwrap_err(),
            rom_ai::AiError::Denied
        );
        runtime.process_work(8).await.unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restored_due_work_attaches_before_start_and_marker_cannot_be_rewritten() {
    use rom::{Patch, Resource};
    use rom_ai::flow::AiRun;
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let handle = client
            .submit(&owner(), submission("restored"))
            .await
            .unwrap();
        runtime.process_work(2).await.unwrap(); // materialize and commit admission, before channel delivery
        let snapshot = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        let value = snapshot.value.unwrap();
        let original = value.encode();
        let record: serde_json::Value =
            serde_json::from_str(original["encoded"].as_str().unwrap()).unwrap();
        assert_eq!(record["queue_admitted"], true);
        let before = db.storage.reaction_records().unwrap();
        let events = db.storage.journal_head(AiRun::KIND).unwrap();
        for mutate in [0, 1] {
            let mut forged = record.clone();
            if mutate == 0 {
                forged["queue_admitted"] = serde_json::json!(false);
            } else {
                forged["policy"]["version"] = serde_json::json!(2);
            }
            let patch = Patch::new().set(
                AiRun::encoded_field(),
                serde_json::to_string(&forged).unwrap(),
            );
            assert!(
                runtime
                    .execute(
                        &service(),
                        rom::Command::patch(&handle.0, patch)
                            .at_revision(snapshot.revision)
                            .idempotency(&format!("forge-{mutate}"))
                    )
                    .await
                    .is_err()
            );
        }
        assert_eq!(db.storage.reaction_records().unwrap(), before);
        assert_eq!(db.storage.journal_head(AiRun::KIND).unwrap(), events);
        client
            .submit(&owner(), submission("restored"))
            .await
            .unwrap();
        assert_eq!(db.storage.reaction_records().unwrap(), before);
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        let db = db.reopen();
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        assert_eq!(
            adapter.calls.load(Ordering::SeqCst),
            0,
            "build must not run restored due work"
        );
        let worker = runtime.start_work().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
        worker.join().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_replay_keeps_original_identity_after_clock_changes() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let (runtime, client) = build(&db, clock.clone(), adapter);
        let handle = client
            .submit(&owner(), submission("cancel-replay"))
            .await
            .unwrap();
        let initial = client.view(&owner(), &handle).await.unwrap();
        client
            .cancel(&owner(), &handle, initial.revision(), "cancel-original")
            .await
            .unwrap();
        clock.0.store(5000, Ordering::SeqCst);
        let replay = client
            .cancel(&owner(), &handle, initial.revision(), "cancel-original")
            .await
            .unwrap();
        assert_eq!(replay.state(), &RunState::Cancelled);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unresolved_resume_replay_does_not_repeat_lookup_and_charges_tick() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let handle = client
            .submit(&owner(), submission("lookup-replay"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let held = client.view(&owner(), &handle).await.unwrap();
        client
            .resume(&owner(), &handle, held.revision(), "lookup-original")
            .await
            .unwrap();
        clock.0.store(2000, Ordering::SeqCst);
        let replay = client
            .resume(&owner(), &handle, held.revision(), "lookup-original")
            .await
            .unwrap();
        assert_eq!(
            adapter.lookups.load(Ordering::SeqCst),
            1,
            "same caller operation must not repeat completed lookup"
        );
        assert_eq!(
            replay.counters().ticks(),
            2,
            "lookup admission is a durable bounded tick"
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_reconciliation_completes_original_attempt_without_generation() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1).reconciliation(1));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let handle = client
            .submit(&owner(), submission("accepted-reconcile"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let held = client.view(&owner(), &handle).await.unwrap();
        let completed = client
            .resume(&owner(), &handle, held.revision(), "reconcile-original")
            .await
            .unwrap();
        assert_eq!(completed.state(), &RunState::Completed);
        assert_eq!(
            completed.output(),
            Some(&serde_json::json!({"answer":"reconciled"}))
        );
        assert_eq!(completed.counters().generation_attempts(), 1);
        assert_eq!(completed.counters().ticks(), 2);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        client
            .resume(&owner(), &handle, held.revision(), "reconcile-original")
            .await
            .unwrap();
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirmed_nonacceptance_schedules_new_bounded_attempt_automatically() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(3).reconciliation(2));
        let (runtime, client) = build(&db, clock.clone(), adapter.clone());
        let handle = client
            .submit(&owner(), submission("not-accepted"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let held = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(
            client
                .resume(&owner(), &handle, held.revision(), "confirmed-original")
                .await
                .unwrap()
                .state(),
            &RunState::Waiting {
                retry_at_unix_ms: 2000
            }
        );
        clock.0.store(2000, Ordering::SeqCst);
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 2);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        assert_eq!(
            client
                .view(&owner(), &handle)
                .await
                .unwrap()
                .counters()
                .ticks(),
            3
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operation_identity_and_tick_exhaustion_reject_without_lookup_or_fact() {
    use rom_ai::flow::AiRun;
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(1));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let mut input = submission("bounded-lookup");
        input.policy = RoutingPolicy::new(
            1,
            vec!["free".into()],
            vec![],
            None,
            RunLimits {
                ticks: 2,
                ..Default::default()
            },
        )
        .unwrap();
        let handle = client.submit(&owner(), input).await.unwrap();
        runtime.process_work(8).await.unwrap();
        let held = client.view(&owner(), &handle).await.unwrap();
        client
            .resume(&owner(), &handle, held.revision(), "same-key")
            .await
            .unwrap();
        let current = client.view(&owner(), &handle).await.unwrap();
        let head = db.storage.journal_head("ai_runs").unwrap();
        let work = db.storage.reaction_records().unwrap();
        assert_eq!(
            client
                .cancel(&owner(), &handle, held.revision(), "same-key")
                .await
                .unwrap_err(),
            rom_ai::AiError::Conflict
        );
        assert_eq!(
            client
                .resume(&owner(), &handle, held.revision() + 1, "same-key")
                .await
                .unwrap_err(),
            rom_ai::AiError::Conflict
        );
        assert_eq!(
            client
                .resume(&owner(), &handle, current.revision(), "fresh-key")
                .await
                .unwrap_err(),
            rom_ai::AiError::BudgetExhausted
        );
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), head);
        assert_eq!(db.storage.reaction_records().unwrap(), work);
        let snapshot = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        assert_eq!(snapshot.revision, current.revision());
        runtime.revoke(&owner());
        assert_eq!(
            client
                .resume(&owner(), &handle, held.revision(), "same-key")
                .await
                .unwrap_err(),
            rom_ai::AiError::Denied
        );
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_during_provider_call_retains_late_evidence_without_output() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(0).completion_gate(release.clone()));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let handle = client
            .submit(&owner(), submission("late-cancel"))
            .await
            .unwrap();
        let worker = runtime.start_work().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while adapter.calls.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let executing = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(executing.state(), &RunState::Executing);
        assert_eq!(
            client
                .cancel(&owner(), &handle, executing.revision(), "cancel-inflight")
                .await
                .unwrap()
                .state(),
            &RunState::CancelRequested
        );
        release.add_permits(1);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let view = client.view(&owner(), &handle).await.unwrap();
                if view.state() == &RunState::Cancelled {
                    assert_eq!(view.output(), None);
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
        worker.join().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn body_clock_expiry_holds_original_effect_without_publishing_completed() {
    use rom_ai::flow::{AiRun, CHECKPOINT_RUN, CheckpointRun};
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0).clock_jump(clock.clone(), 2000));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let mut input = submission("body-expiry");
        input.policy = RoutingPolicy::new(
            1,
            vec!["free".into()],
            vec![],
            None,
            RunLimits {
                age_seconds: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let handle = client.submit(&owner(), input).await.unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert_eq!(view.output(), None);
        assert_eq!(view.counters().generation_attempts(), 1);
        let snapshot = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        let record = snapshot.value.unwrap().record().unwrap();
        assert_eq!(record.attempt_usage()[0].input_tokens, Some(4));
        assert_eq!(record.attempt_usage()[0].cost, Some(rom_ai::UsdNanos(0)));
        let head = db.storage.journal_head("ai_runs").unwrap();
        assert!(
            runtime
                .execute(
                    &service(),
                    rom::Command::action(
                        &handle.0,
                        CHECKPOINT_RUN,
                        CheckpointRun::completed(
                            1,
                            2000,
                            record.attempt_evidence()[0].clone(),
                            Usage {
                                input_tokens: Some(4),
                                output_tokens: Some(1),
                                cost: Some(rom_ai::UsdNanos(1)),
                                ..Default::default()
                            },
                            serde_json::json!({"answer":"conflict"})
                        )
                        .unwrap()
                    )
                    .at_revision(snapshot.revision)
                    .idempotency("conflicting-late-usage")
                )
                .await
                .is_err()
        );
        assert_eq!(db.storage.journal_head("ai_runs").unwrap(), head);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_provider_outcome_survives_concurrent_cancel_stamp() {
    cancellation_outcome_race(1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirmed_rate_limit_survives_concurrent_cancel_stamp_without_successor() {
    cancellation_outcome_race(2).await;
}
async fn cancellation_outcome_race(mode: u8) {
    for redb in [false, true] {
        use rom_ai::flow::{AiBudget, AiRun, ReservationStatus};
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(mode).completion_gate(release.clone()));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let run = client
            .submit(&owner(), submission("cancel-outcome-race"))
            .await
            .unwrap();
        let worker = runtime.start_work().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while adapter.calls.load(Ordering::SeqCst) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let original = client.view(&owner(), &run).await.unwrap();
        let cancelled = client
            .cancel(&owner(), &run, original.revision(), "cancel-racing-outcome")
            .await
            .unwrap();
        assert_eq!(cancelled.state(), &RunState::CancelRequested);
        release.add_permits(1);
        let expected = if mode == 1 {
            RunState::AwaitingReconciliation
        } else {
            RunState::Cancelled
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if client.view(&owner(), &run).await.unwrap().state() == &expected
                    && db.storage.reaction_records().unwrap().iter().all(|work| {
                        !matches!(
                            work.state,
                            rom::WorkState::Leased { .. } | rom::WorkState::Pending
                        )
                    })
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert!(record.cancel_requested());
        assert_eq!(
            record.attempt_evidence()[0].attempt_id(),
            record.active_attempt().unwrap().prepared().identity()
        );
        assert_eq!(record.counters().generation_attempts(), 1);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert!(
            client
                .view(&owner(), &run)
                .await
                .unwrap()
                .output()
                .is_none()
        );
        let account = runtime
            .read::<AiBudget>(&service(), "rom-ai-free-v1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(account.entries().len(), 1);
        assert_eq!(
            account.entries()[0].status(),
            &if mode == 1 {
                ReservationStatus::Unknown
            } else {
                ReservationStatus::Settled {
                    actual_cost: rom_ai::UsdNanos(0),
                }
            }
        );
        client
            .cancel(&owner(), &run, original.revision(), "cancel-racing-outcome")
            .await
            .unwrap();
        runtime.shutdown().await.unwrap();
        worker.join().await.unwrap();
        if mode == 1 {
            assert!(
                db.storage
                    .reaction_records()
                    .unwrap()
                    .iter()
                    .any(|work| work.state == rom::WorkState::AwaitingReconciliation)
            );
        } else {
            assert!(
                db.storage
                    .reaction_records()
                    .unwrap()
                    .iter()
                    .all(|work| work.state == rom::WorkState::Done)
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_provider_outcome_survives_concurrent_reconciliation_stamp() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let adapter = Arc::new(Adapter::new(1).completion_gate(release.clone()));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let run = client
            .submit(&owner(), submission("resume-outcome-race"))
            .await
            .unwrap();
        let worker = runtime.start_work().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while adapter.calls.load(Ordering::SeqCst) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let original = client.view(&owner(), &run).await.unwrap();
        let resumed = client
            .resume(&owner(), &run, original.revision(), "resume-racing-outcome")
            .await
            .unwrap();
        assert_eq!(resumed.state(), &RunState::AwaitingReconciliation);
        release.add_permits(1);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let work = db.storage.reaction_records().unwrap();
                if work.iter().all(|work| {
                    !matches!(
                        work.state,
                        rom::WorkState::Leased { .. } | rom::WorkState::Pending
                    )
                }) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(
            db.storage
                .reaction_records()
                .unwrap()
                .iter()
                .any(|work| work.state == rom::WorkState::AwaitingReconciliation)
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 1);
        let current = client.view(&owner(), &run).await.unwrap();
        assert_eq!(current.state(), &RunState::AwaitingReconciliation);
        assert_eq!(current.counters().ticks(), 2);
        runtime.shutdown().await.unwrap();
        worker.join().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_completion_is_held_without_trusting_supplied_usage() {
    unsafe_completion(1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_attempt_completion_is_held_without_trusting_supplied_evidence() {
    unsafe_completion(2).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_tool_completion_is_held_without_executing_or_trusting_usage() {
    unsafe_completion(3).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_valid_requested_tool_completion_retains_knowledge_without_tool_execution() {
    valid_tool_knowledge(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_valid_requested_tool_completion_retains_knowledge_without_tool_execution() {
    valid_tool_knowledge(true).await;
}
async fn valid_tool_knowledge(redb: bool) {
    use rom_ai::flow::{AiBudget, AiRun};
    let db = Db::new(redb);
    let clock = Arc::new(Clock(AtomicU64::new(1000)));
    let adapter = Arc::new(Adapter::new(6));
    let grant = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let build = |db: &Db| {
        FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(GrantAuthority(grant.clone())),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap()
        .install(Runtime::builder().clock(clock.clone()))
        .unwrap()
        .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap()
    };
    let (runtime, client) = build(&db);
    runtime
        .execute(
            &service(),
            rom::Command::create(
                "tool-account",
                AiBudget::new(&service(), "tool-account", rom_ai::UsdNanos(20)).unwrap(),
            )
            .idempotency("tool-account-create"),
        )
        .await
        .unwrap();
    let mut input = submission("valid-tool-held");
    input.policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["paid".into()],
        Some(ModelPrice::new(
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(10),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("tool-account")
    .unwrap();
    input.request = input
        .request
        .with_tools(vec![
            rom_ai::ToolDescriptor::new(
                "requested-tool",
                "Public fixture tool",
                serde_json::json!({"type":"object"}),
            )
            .unwrap(),
        ])
        .unwrap();
    let run = client.submit(&owner(), input).await.unwrap();
    runtime.process_work(8).await.unwrap();
    let view = client.view(&owner(), &run).await.unwrap();
    assert_eq!(view.state(), &RunState::AwaitingReconciliation);
    assert!(view.output().is_none());
    let original = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(
        original.attempt_evidence()[0].generation_id(),
        Some("gen-requested-tool")
    );
    assert_eq!(original.attempt_usage()[0].cost, Some(rom_ai::UsdNanos(3)));
    assert_eq!(original.counters().tool_calls(), 0);
    let account = runtime
        .read::<AiBudget>(&service(), "tool-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(account.entries().len(), 1);
    assert_eq!(account.settled(), rom_ai::UsdNanos(3));
    assert_eq!(account.reserved(), rom_ai::UsdNanos(0));
    runtime.process_work(8).await.unwrap();
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    let db = db.reopen();
    let (runtime, client) = build(&db);
    let current = client.view(&owner(), &run).await.unwrap();
    grant.store(false, Ordering::SeqCst);
    assert!(matches!(
        client
            .resume(&owner(), &run, current.revision(), "tool-revoked-lookup")
            .await,
        Err(rom_ai::AiError::Denied)
    ));
    assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
    let after = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(after.active_attempt(), original.active_attempt());
    assert_eq!(after.attempt_evidence(), original.attempt_evidence());
    assert_eq!(after.attempt_usage(), original.attempt_usage());
    assert_eq!(after.counters().tool_calls(), 0);
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    assert!(
        client
            .view(&owner(), &run)
            .await
            .unwrap()
            .output()
            .is_none()
    );
    runtime.shutdown().await.unwrap();
}
async fn unsafe_completion(invalid: u8) {
    for redb in [false, true] {
        use rom_ai::flow::{AiBudget, AiRun, ReservationStatus};
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0).invalid_completion(invalid));
        let (runtime, client) = build(&db, clock, adapter.clone());
        let run = client
            .submit(&owner(), submission("unsafe-completion"))
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert!(view.output().is_none());
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let evidence = record.attempt_evidence();
        assert_eq!(evidence.len(), 1);
        assert_eq!(
            evidence[0].attempt_id(),
            record.active_attempt().unwrap().prepared().identity()
        );
        assert_eq!(evidence[0].provider_request_id(), None);
        assert_eq!(record.attempt_usage(), &[Usage::default()]);
        let account = runtime
            .read::<AiBudget>(&service(), "rom-ai-free-v1")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(account.entries()[0].status(), &ReservationStatus::Unknown);
        assert!(
            db.storage
                .reaction_records()
                .unwrap()
                .iter()
                .any(|work| work.state == rom::WorkState::AwaitingReconciliation)
        );
        runtime.process_work(8).await.unwrap();
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn binding_is_local_without_autostart_or_runtime_cycle() {
    for redb in [false, true] {
        let db = Db::new(redb);
        let clock = Arc::new(Clock(AtomicU64::new(1000)));
        let adapter = Arc::new(Adapter::new(0));
        let host = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            clock.clone(),
        )
        .unwrap();
        let (runtime, client) = host
            .install(Runtime::builder().clock(clock))
            .unwrap()
            .build(db.storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let run = client.submit(&owner(), submission("first")).await.unwrap();
        assert_eq!(
            adapter.calls.load(Ordering::SeqCst),
            0,
            "build/submit must not start workers"
        );
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::Queued
        );
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            adapter.calls.load(Ordering::SeqCst),
            1,
            "work states: {:?}",
            db.storage
                .reaction_records()
                .unwrap()
                .iter()
                .map(|work| (&work.pending.definition, &work.state))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::Completed
        );
        client
            .submit(&owner(), submission("after-drop"))
            .await
            .unwrap();
        drop(client);
        runtime.process_work(8).await.unwrap();
        assert_eq!(
            adapter.calls.load(Ordering::SeqCst),
            1,
            "weak attachment cannot keep a dropped client bound"
        );
        runtime.shutdown().await.unwrap();
    }
}
