//! Positive-cost native tool observations settle before application I/O, across restart.
use super::*;
use rom_ai::{
    UsdNanos,
    flow::{AiBudget, ReservationStatus},
};

struct PaidClock(AtomicU64);
impl AiClock for PaidClock {
    fn now_unix_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl rom::Clock for PaidClock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst) / 1000
    }
}
struct PaidAdapter {
    calls: AtomicU64,
}
struct PaidAuthority(Arc<AtomicBool>);
impl FlowAuthority for PaidAuthority {
    fn submit(&self, actor: &Actor, input: &Submission) -> AiResult<OwnerIdentity> {
        Authority.submit(actor, input)
    }
    fn inspect(&self, actor: &Actor, owner: &OwnerIdentity) -> AiResult<()> {
        Authority.inspect(actor, owner)
    }
    fn cancel(&self, actor: &Actor, owner: &OwnerIdentity) -> AiResult<()> {
        Authority.cancel(actor, owner)
    }
    fn resolve(
        &self,
        owner: &OwnerIdentity,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        Authority.resolve(owner, reads)
    }
    fn attempt(
        &self,
        actor: &Actor,
        owner: &OwnerIdentity,
        prepared: &PreparedAttempt,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        if !self.0.load(Ordering::SeqCst) {
            return Err(rom::Error::Denied);
        }
        Authority.attempt(actor, owner, prepared, reads)
    }
    fn tool_actor(
        &self,
        actor: &Actor,
        owner: &OwnerIdentity,
        call: &ToolCall,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        Authority.tool_actor(actor, owner, call, reads)
    }
}
impl Provider for PaidAdapter {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async {
            CatalogSnapshot::new(
                "paid-tools",
                vec![CatalogModel::text(
                    "paid",
                    262144,
                    true,
                    true,
                    ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10)),
                )],
            )
        })
    }
    fn complete<'a>(&'a self, prepared: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            let evidence =
                AttemptEvidence::new(prepared.identity(), None, Some(format!("paid-gen-{call}")))?;
            let usage = Usage {
                cost: Some(UsdNanos(3)),
                ..Default::default()
            };
            if call == 1 {
                Completion::tool_calls(
                    vec![ToolCall::new(
                        "paid-write",
                        "classify_task",
                        serde_json::json!({"target":"task","expected_revision":1,"input":"reviewed"}),
                    )?],
                    usage,
                    evidence,
                )
            } else {
                Completion::output(serde_json::json!({"done":true}), usage, evidence)
            }
        })
    }
    fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
        Box::pin(async { panic!("a durable known cost must not trigger a provider lookup") })
    }
}
fn host(
    storage: Arc<dyn Storage>,
    clock: Arc<PaidClock>,
    adapter: Arc<PaidAdapter>,
    grant: Arc<AtomicBool>,
) -> (Runtime, rom_ai::flow::FlowClient) {
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .action(
            "classify_task",
            "Apply the validated classification",
            CLASSIFY,
        )
        .unwrap();
    FlowHost::new(
        service(),
        adapter,
        Arc::new(PaidAuthority(grant)),
        Arc::new(Validator),
        clock.clone(),
    )
    .unwrap()
    .tools(registry)
    .unwrap()
    .install(
        Runtime::builder().clock(clock).resource(
            Task::definition()
                .action(CLASSIFY)
                .policy(|actor, _, _| actor.subject == "tool-owner")
                .field_policy(|actor, _, _, _| actor.subject == "tool-owner"),
        ),
    )
    .unwrap()
    .build(storage, Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}
pub(super) async fn journey(redb: bool, after_commit: bool, revoke: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-paid-tools-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let open = || -> Arc<dyn Storage> {
        if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        }
    };
    let raw = open();
    let fault = Arc::new(crate::settlement_fault::SettlementFault {
        inner: raw.clone(),
        armed: AtomicBool::new(true),
        after_commit,
    });
    let clock = Arc::new(PaidClock(AtomicU64::new(1000)));
    let adapter = Arc::new(PaidAdapter {
        calls: AtomicU64::new(0),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let (runtime, client) = host(fault.clone(), clock.clone(), adapter.clone(), grant.clone());
    runtime
        .execute(
            &owner(),
            Command::create(
                "task",
                Task {
                    label: "original".into(),
                    read_allowed: true,
                },
            )
            .idempotency("paid-task"),
        )
        .await
        .unwrap();
    runtime
        .execute(
            &service(),
            Command::create(
                "paid-account",
                AiBudget::new(&service(), "paid-account", UsdNanos(20)).unwrap(),
            )
            .idempotency("paid-grant"),
        )
        .await
        .unwrap();
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .action(
            "classify_task",
            "Apply the validated classification",
            CLASSIFY,
        )
        .unwrap();
    let request = CompletionRequest::new(vec![Message::user("Classify the authorized task")], 64)
        .unwrap()
        .with_tools(registry.descriptors())
        .unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["paid".into()],
        Some(ModelPrice::new(UsdNanos(0), UsdNanos(0), UsdNanos(10))),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("paid-account")
    .unwrap();
    let run = client
        .submit(
            &owner(),
            Submission {
                id: "paid-tools".into(),
                idempotency: "submit-paid-tools".into(),
                request,
                policy,
            },
        )
        .await
        .unwrap();
    for _ in 0..8 {
        runtime.process_work(1).await.unwrap();
        if !fault.armed.load(Ordering::SeqCst) {
            break;
        }
    }
    assert!(
        !fault.armed.load(Ordering::SeqCst),
        "the intended known-cost settlement fault must fire"
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime
            .read::<Task>(&owner(), "task")
            .await
            .unwrap()
            .revision,
        1,
        "no tool action before acknowledged original settlement"
    );
    let record = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.state(), &RunState::ToolsPending);
    assert_eq!(
        record.attempt_evidence()[0].generation_id(),
        Some("paid-gen-1")
    );
    assert_eq!(record.attempt_usage()[0].cost, Some(UsdNanos(3)));
    let original = runtime
        .read::<AiBudget>(&service(), "paid-account")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(original.entries().len(), 1);
    assert_eq!(
        original.reserved(),
        UsdNanos(if after_commit { 0 } else { 10 })
    );
    assert_eq!(
        original.settled(),
        UsdNanos(if after_commit { 3 } else { 0 })
    );
    let original_entry = original.entries()[0].clone();
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    drop(fault);
    drop(raw);
    let storage = open();
    if revoke {
        grant.store(false, Ordering::SeqCst);
    }
    let (runtime, client) = host(
        storage.clone(),
        clock.clone(),
        adapter.clone(),
        grant.clone(),
    );
    clock.0.store(3000, Ordering::SeqCst);
    if revoke {
        let original_head = storage.journal_head("ai_budgets").unwrap();
        for _ in 0..4 {
            runtime.process_work(1).await.unwrap();
        }
        let account = runtime
            .read::<AiBudget>(&service(), "paid-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            account, original,
            "revocation retains exact known money and conservative reservation"
        );
        assert_eq!(storage.journal_head("ai_budgets").unwrap(), original_head);
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            client.view(&owner(), &run).await.unwrap().state(),
            &RunState::ToolsPending
        );
        runtime.shutdown().await.unwrap();
        return;
    }
    for _ in 0..16 {
        runtime.process_work(1).await.unwrap();
        if client.view(&owner(), &run).await.unwrap().state() == &RunState::Completed {
            break;
        }
    }
    assert_eq!(
        client.view(&owner(), &run).await.unwrap().state(),
        &RunState::Completed
    );
    assert_eq!(
        runtime
            .read::<Task>(&owner(), "task")
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        adapter.calls.load(Ordering::SeqCst),
        2,
        "only the explicit successor generation follows the committed tool result"
    );
    let account = runtime
        .read::<AiBudget>(&service(), "paid-account")
        .await
        .unwrap();
    let final_record = account.value.as_ref().unwrap().record().unwrap();
    assert_eq!(final_record.entries().len(), 2);
    assert_eq!(final_record.entries()[0].key(), original_entry.key());
    assert_eq!(
        final_record.entries()[0].prepared(),
        original_entry.prepared()
    );
    assert_eq!(
        final_record.entries()[0].status(),
        &ReservationStatus::Settled {
            actual_cost: UsdNanos(3)
        }
    );
    assert_eq!(final_record.reserved(), UsdNanos(0));
    assert_eq!(final_record.settled(), UsdNanos(6));
    let head = storage.journal_head("ai_budgets").unwrap();
    runtime.process_work(8).await.unwrap();
    assert_eq!(
        runtime
            .read::<AiBudget>(&service(), "paid-account")
            .await
            .unwrap()
            .revision,
        account.revision
    );
    assert_eq!(storage.journal_head("ai_budgets").unwrap(), head);
    assert_eq!(
        runtime
            .read::<Task>(&owner(), "task")
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 2);
    runtime.shutdown().await.unwrap();
}
