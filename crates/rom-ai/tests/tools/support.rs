//! Public actual-database fixture; application declarations remain outside rom-ai.
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_ai::flow::{AiRun, FlowAuthority, FlowHost, OwnerIdentity, RunState, Submission};
use rom_ai::{
    AiClock, AiFuture, AiResult, AttemptEvidence, CatalogModel, CatalogSnapshot, Completion,
    CompletionRequest, Deadline, Message, ModelPrice, OutputValidator, PreparedAttempt, Provider,
    Reconciliation, RoutingPolicy, RunLimits, ToolCall, ToolRegistry, Usage,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[path = "paid.rs"]
mod paid;
#[path = "read_recovery.rs"]
mod read_recovery;
#[path = "resume_race.rs"]
mod resume_race;
pub async fn paid_settlement(redb: bool, after_commit: bool) {
    paid::journey(redb, after_commit, false).await;
}
pub async fn paid_revoked_grant(redb: bool) {
    for after_commit in [false, true] {
        paid::journey(redb, after_commit, true).await;
    }
}

#[derive(Clone, rom::Resource)]
#[resource(name = "typed_tool_task", version = 1)]
struct Task {
    label: String,
    read_allowed: bool,
}
const CLASSIFY: rom::Action<Task, String> = rom::Action::new("classify", |task, label| {
    if label != "reviewed" {
        return Err(rom::Error::Denied);
    }
    task.label = label;
    Ok(vec![])
});
fn owner() -> Actor {
    Actor::trusted("tool-tests", "tool-owner")
}
fn service() -> Actor {
    Actor::trusted("tool-tests", "tool-service").with_kind(rom::PrincipalKind::Service)
}
struct Clock;
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        1000
    }
}
struct RecoveryClock(Arc<AtomicU64>);
impl AiClock for RecoveryClock {
    fn now_unix_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
impl rom::Clock for Clock {
    fn now(&self) -> u64 {
        1
    }
}
struct Authority;
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        let identity = OwnerIdentity::from_actor(&owner())?;
        self.inspect(actor, &identity)?;
        Ok(identity)
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor) && actor.subject == owner().subject {
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
        if identity.matches(actor) && prepared.validate().is_ok() {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
    fn tool_actor(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        _: &ToolCall,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if identity.matches(actor) {
            Ok(actor.clone())
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
struct Adapter {
    attempts: Mutex<Vec<PreparedAttempt>>,
    lookups: AtomicU64,
    repeat_call: bool,
    reads: AtomicU64,
    blocked_read: Option<Arc<read_recovery::ReadBlock>>,
    now: Arc<AtomicU64>,
    read_admission: Option<Arc<super::fault::ReadAdmissionFault>>,
    slow_authority: Arc<AtomicBool>,
}
impl Provider for Adapter {
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async {
            CatalogSnapshot::new(
                "tools-fixture",
                vec![CatalogModel::text(
                    "free",
                    262144,
                    true,
                    true,
                    ModelPrice::free(),
                )],
            )
        })
    }
    fn complete<'a>(&'a self, attempt: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
        Box::pin(async move {
            let index = {
                let mut attempts = self.attempts.lock().unwrap();
                attempts.push(attempt.clone());
                attempts.len()
            };
            let evidence =
                AttemptEvidence::new(attempt.identity(), None, Some(format!("gen-tool-{index}")))?;
            let usage = Usage {
                cost: Some(rom_ai::UsdNanos(0)),
                ..Default::default()
            };
            if index == 1 {
                Completion::tool_calls(
                    vec![
                        ToolCall::new("read-1", "read_task", serde_json::json!({"input":"task"}))?,
                        ToolCall::new(
                            "write-1",
                            "classify_task",
                            serde_json::json!({"target":"task","expected_revision":1,"input":"reviewed"}),
                        )?,
                    ],
                    usage,
                    evidence,
                )
            } else if self.repeat_call {
                Completion::tool_calls(
                    vec![ToolCall::new(
                        "read-1",
                        "read_task",
                        serde_json::json!({"input":"task"}),
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
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Reconciliation::Unresolved) })
    }
}
pub async fn journey(redb: bool, forgeries: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-tools-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    let adapter = Arc::new(Adapter {
        attempts: Mutex::new(vec![]),
        lookups: AtomicU64::new(0),
        repeat_call: false,
        reads: AtomicU64::new(0),
        blocked_read: None,
        now: Arc::new(AtomicU64::new(1000)),
        read_admission: None,
        slow_authority: Arc::new(AtomicBool::new(false)),
    });
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .read::<String, String, _>(
            "read_task",
            "Read the authorized task label",
            |context, input| {
                Box::pin(async move {
                    let snapshot = context
                        .read::<Task>(&input)
                        .await
                        .map_err(|_| rom_ai::AiError::Denied)?;
                    Ok(snapshot.value.ok_or(rom_ai::AiError::Denied)?.label)
                })
            },
        )
        .unwrap();
    registry
        .action(
            "classify_task",
            "Apply the validated classification",
            CLASSIFY,
        )
        .unwrap();
    let request = CompletionRequest::new(vec![Message::user("Read and classify the task")], 64)
        .unwrap()
        .with_tools(registry.descriptors())
        .unwrap();
    let (runtime, client) = FlowHost::new(
        service(),
        adapter.clone(),
        Arc::new(Authority),
        Arc::new(Validator),
        Arc::new(Clock),
    )
    .unwrap()
    .tools(registry)
    .unwrap()
    .install(
        Runtime::builder().clock(Arc::new(Clock)).resource(
            Task::definition()
                .action(CLASSIFY)
                .policy(|actor, _, _| actor.subject == "tool-owner")
                .field_policy(|actor, _, _, _| actor.subject == "tool-owner"),
        ),
    )
    .unwrap()
    .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
    .unwrap();
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
            .idempotency("create-task"),
        )
        .await
        .unwrap();
    let handle = client
        .submit(
            &owner(),
            Submission {
                id: "tool-run".into(),
                idempotency: "submit-tools".into(),
                request: request.clone(),
                policy: RoutingPolicy::new(
                    1,
                    vec!["free".into()],
                    vec![],
                    None,
                    RunLimits::default(),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();
    for _ in 0..8 {
        runtime.process_work(8).await.unwrap();
    }
    let view = client.view(&owner(), &handle).await.unwrap();
    assert_eq!(view.state(), &RunState::Completed);
    let record = runtime
        .read::<AiRun>(&service(), "tool-run")
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.request(), &request);
    assert_eq!(record.counters().tool_calls(), 2);
    let task = runtime.read::<Task>(&owner(), "task").await.unwrap();
    assert_eq!(task.revision, 2);
    assert_eq!(task.value.unwrap().label, "reviewed");
    let attempts = adapter.attempts.lock().unwrap().clone();
    assert_eq!(attempts.len(), 2);
    let messages = attempts[1].request().messages();
    assert_eq!(messages.len(), 4);
    assert_eq!(
        messages[1]
            .tool_calls
            .iter()
            .map(|call| call.id())
            .collect::<Vec<_>>(),
        vec!["read-1", "write-1"]
    );
    assert_eq!(messages[2].tool_call_id.as_deref(), Some("read-1"));
    assert_eq!(messages[3].tool_call_id.as_deref(), Some("write-1"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&messages[2].content).unwrap(),
        "original"
    );
    drop(attempts);
    for _ in 0..2 {
        runtime.process_work(8).await.unwrap();
    }
    assert_eq!(
        runtime
            .read::<Task>(&owner(), "task")
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
    if forgeries {
        let snapshot = runtime.read::<AiRun>(&service(), "tool-run").await.unwrap();
        let original = snapshot.value.unwrap();
        let encoded = serde_json::to_value(original.record().unwrap()).unwrap();
        let events = storage.journal_head(AiRun::KIND).unwrap();
        let work = storage.reaction_records().unwrap();
        let money = storage.journal_head(rom_ai::flow::AiBudget::KIND).unwrap();
        for variant in 0..4 {
            let mut forged = encoded.clone();
            match variant {
                0 => {
                    forged["tool_turns"][0]["steps"][1]["call"]["id"] = serde_json::json!("read-1")
                }
                1 => forged["tool_turns"][0]["source"]["step"] = serde_json::json!(2),
                2 => forged["tool_turns"][0]["registry_version"] = serde_json::json!(2),
                _ => {
                    forged["tool_turns"][0]["steps"][0]["actor"]["subject"] =
                        serde_json::json!("another-principal")
                }
            }
            let raw = serde_json::to_string(&forged).unwrap();
            let replacement = AiRun::decode(serde_json::json!({"encoded":raw})).unwrap();
            if variant == 3 {
                assert!(
                    replacement.record().is_ok(),
                    "different valid actor identity is structurally well-formed but cannot replace historical authority facts"
                );
            } else {
                assert!(
                    replacement.record().is_err(),
                    "decoded structural forgery must fail closed"
                );
            }
            assert!(
                runtime
                    .execute(
                        &service(),
                        Command::patch(
                            "tool-run",
                            rom::Patch::new().set(AiRun::encoded_field(), raw)
                        )
                        .at_revision(snapshot.revision)
                        .idempotency(&format!("forge-tools-{variant}"))
                    )
                    .await
                    .is_err()
            );
            assert_eq!(storage.journal_head(AiRun::KIND).unwrap(), events);
            assert_eq!(storage.reaction_records().unwrap(), work);
            assert_eq!(
                storage.journal_head(rom_ai::flow::AiBudget::KIND).unwrap(),
                money
            );
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), "tool-run")
                    .await
                    .unwrap()
                    .value
                    .unwrap()
                    .encode(),
                original.encode()
            );
        }
        assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
    }
    drop(client);
    drop(runtime);
    std::fs::remove_dir_all(directory).unwrap();
}

pub async fn registry_freeze(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-tool-registry-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let adapter = Arc::new(Adapter {
        attempts: Mutex::new(vec![]),
        lookups: AtomicU64::new(0),
        repeat_call: false,
        reads: AtomicU64::new(0),
        blocked_read: None,
        now: Arc::new(AtomicU64::new(1000)),
        read_admission: None,
        slow_authority: Arc::new(AtomicBool::new(false)),
    });
    for version in [1, 2] {
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let mut registry = ToolRegistry::new(version).unwrap();
        registry
            .read::<String, String, _>("read_task", "Read the authorized task label", |_, input| {
                Box::pin(async move { Ok(input) })
            })
            .unwrap();
        let request =
            CompletionRequest::new(vec![Message::user("Retain this queued identity")], 64)
                .unwrap()
                .with_tools(registry.descriptors())
                .unwrap();
        let (runtime, client) = FlowHost::new(
            service(),
            adapter.clone(),
            Arc::new(Authority),
            Arc::new(Validator),
            Arc::new(Clock),
        )
        .unwrap()
        .tools(registry)
        .unwrap()
        .install(Runtime::builder().clock(Arc::new(Clock)))
        .unwrap()
        .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        client
            .submit(
                &owner(),
                Submission {
                    id: "frozen-registry".into(),
                    idempotency: "submit-frozen-registry".into(),
                    request: request.clone(),
                    policy: RoutingPolicy::new(
                        1,
                        vec!["free".into()],
                        vec![],
                        None,
                        RunLimits::default(),
                    )
                    .unwrap(),
                },
            )
            .await
            .unwrap();
        let snapshot = runtime
            .read::<AiRun>(&service(), "frozen-registry")
            .await
            .unwrap();
        let encoded = serde_json::to_value(snapshot.value.unwrap().record().unwrap()).unwrap();
        assert_eq!(
            encoded["tool_registry_version"], 1,
            "original submission registry remains frozen after host replacement"
        );
        assert_eq!(snapshot.revision, 1);
        let head = storage.journal_head(AiRun::KIND).unwrap();
        let work = storage.reaction_records().unwrap();
        let mut forged = encoded.clone();
        forged["tool_registry_version"] = serde_json::json!(version + 1);
        let replacement =
            AiRun::decode(serde_json::json!({"encoded":serde_json::to_string(&forged).unwrap()}))
                .unwrap();
        assert!(
            replacement.record().is_ok(),
            "changed nonzero registry version is structurally valid"
        );
        assert!(matches!(
            runtime
                .execute(
                    &service(),
                    Command::replace("frozen-registry", replacement)
                        .at_revision(1)
                        .idempotency(&format!("forge-version-{version}"))
                )
                .await,
            Err(rom::Error::Conflict)
        ));
        assert_eq!(storage.journal_head(AiRun::KIND).unwrap(), head);
        assert_eq!(storage.reaction_records().unwrap(), work);
        if version == 2 {
            client
                .submit(
                    &owner(),
                    Submission {
                        id: "explicit-fresh-registry".into(),
                        idempotency: "submit-explicit-fresh-registry".into(),
                        request,
                        policy: RoutingPolicy::new(
                            1,
                            vec!["free".into()],
                            vec![],
                            None,
                            RunLimits::default(),
                        )
                        .unwrap(),
                    },
                )
                .await
                .unwrap();
            let fresh = runtime
                .read::<AiRun>(&service(), "explicit-fresh-registry")
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(fresh.value.unwrap().record().unwrap()).unwrap()["tool_registry_version"],
                2
            );
        }
        assert_eq!(adapter.attempts.lock().unwrap().len(), 0);
        drop(client);
        drop(runtime);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

struct GateAuthority {
    resume_gate: Option<Arc<resume_race::Gate>>,
    tool_grant: Arc<AtomicBool>,
    slow_authority: Arc<AtomicBool>,
    trace: bool,
    trace_count: AtomicU64,
}
impl GateAuthority {
    fn trace(&self, stage: &str) {
        if self.trace && self.trace_count.fetch_add(1, Ordering::SeqCst) < 128 {
            eprintln!("fixture authority stage {stage}");
        }
    }
}
impl FlowAuthority for GateAuthority {
    fn submit(&self, a: &Actor, s: &Submission) -> AiResult<OwnerIdentity> {
        Authority.submit(a, s)
    }
    fn inspect(&self, a: &Actor, o: &OwnerIdentity) -> AiResult<()> {
        self.trace("inspect");
        Authority.inspect(a, o)
    }
    fn cancel(&self, a: &Actor, o: &OwnerIdentity) -> AiResult<()> {
        Authority.cancel(a, o)
    }
    fn resolve(&self, o: &OwnerIdentity, r: &mut dyn rom::AuthorizationRead) -> rom::Result<Actor> {
        self.trace("resolve");
        Authority.resolve(o, r)
    }
    fn attempt(
        &self,
        a: &Actor,
        o: &OwnerIdentity,
        p: &PreparedAttempt,
        r: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        self.trace("attempt");
        Authority.attempt(a, o, p, r)
    }
    fn tool_actor(
        &self,
        a: &Actor,
        o: &OwnerIdentity,
        c: &ToolCall,
        r: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        self.trace("tool_actor");
        if c.name() == "classify_task"
            && let Some(gate) = &self.resume_gate
        {
            gate.pause_once();
        }
        if self.slow_authority.swap(false, Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(2_300));
        }
        if !self.tool_grant.load(Ordering::SeqCst) {
            return Err(rom::Error::Denied);
        }
        let target = if c.name() == "read_task" {
            c.arguments().get("input")
        } else {
            c.arguments().get("target")
        }
        .and_then(serde_json::Value::as_str)
        .ok_or(rom::Error::Denied)?;
        let row = r
            .load(&rom::Key {
                kind: Task::KIND.into(),
                id: target.into(),
            })?
            .ok_or(rom::Error::Denied)?;
        if row
            .value
            .as_ref()
            .and_then(|value| value.get("read_allowed"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            return Err(rom::Error::Denied);
        }
        Authority.tool_actor(a, o, c, r)
    }
}
fn recovery_host(
    adapter: Arc<Adapter>,
    grant: Arc<AtomicBool>,
    storage: Arc<dyn Storage>,
    nullable: bool,
    cpu_revoke: bool,
    read_failure: u8,
) -> (Runtime, rom_ai::flow::FlowClient) {
    recovery_host_with_resume_gate(
        adapter,
        grant,
        storage,
        nullable,
        cpu_revoke,
        read_failure,
        None,
    )
}
fn recovery_host_with_resume_gate(
    adapter: Arc<Adapter>,
    grant: Arc<AtomicBool>,
    storage: Arc<dyn Storage>,
    nullable: bool,
    cpu_revoke: bool,
    read_failure: u8,
    resume_gate: Option<Arc<resume_race::Gate>>,
) -> (Runtime, rom_ai::flow::FlowClient) {
    let mut registry = ToolRegistry::new(1).unwrap();
    if nullable {
        registry
            .read::<String, (), _>(
                "read_task",
                "Read the authorized task label",
                |context, input| {
                    Box::pin(async move {
                        context
                            .read::<Task>(&input)
                            .await
                            .map_err(|_| rom_ai::AiError::Denied)?;
                        Ok(())
                    })
                },
            )
            .unwrap();
    } else {
        let calculation_grant = grant.clone();
        let read_adapter = adapter.clone();
        registry
            .read::<String, String, _>(
                "read_task",
                "Read the authorized task label",
                move |context, input| {
                    let calculation_grant = calculation_grant.clone();
                    let read_adapter = read_adapter.clone();
                    Box::pin(async move {
                        let read_ordinal = read_adapter.reads.fetch_add(1, Ordering::SeqCst) + 1;
                        if read_failure >= 17 {
                            eprintln!("fixture enters read callback ordinal {read_ordinal}");
                        }
                        if read_failure >= 11
                            && read_failure != 18
                            && read_failure != 23
                            && (!matches!(
                                read_failure,
                                17 | 18 | 19 | 20 | 21 | 22 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32
                            ) || read_ordinal == 1)
                        {
                            return Err(match read_failure {
                                11 => rom_ai::AiError::InvalidOutput,
                                12 => rom_ai::AiError::ProviderUnavailable,
                                13 => rom_ai::AiError::UnknownOutcome,
                                14 => rom_ai::AiError::Storage,
                                15 => rom_ai::AiError::Closed,
                                16 => rom_ai::AiError::DeadlineExceeded,
                                _ => rom_ai::AiError::UnknownOutcome,
                            });
                        }
                        let label = context
                            .read::<Task>(&input)
                            .await
                            .map_err(|_| rom_ai::AiError::Denied)?
                            .value
                            .ok_or(rom_ai::AiError::Denied)?
                            .label;
                        if read_failure == 18 && read_ordinal == 1 {
                            let block = read_adapter.blocked_read.as_ref().unwrap().clone();
                            return context
                                .calculate(move || {
                                    block.wait();
                                    Ok(label)
                                })
                                .await
                                .map_err(|_| rom_ai::AiError::UnknownOutcome);
                        }
                        if cpu_revoke {
                            let started = Arc::new(tokio::sync::Notify::new());
                            let observed = started.clone();
                            let revocation = tokio::spawn(async move {
                                observed.notified().await;
                                calculation_grant.store(false, Ordering::SeqCst);
                            });
                            let calculated = context
                                .calculate(move || {
                                    started.notify_one();
                                    std::thread::sleep(std::time::Duration::from_millis(20));
                                    Ok(label)
                                })
                                .await
                                .map_err(|_| rom_ai::AiError::Denied)?;
                            revocation.await.unwrap();
                            Ok(calculated)
                        } else {
                            Ok(label)
                        }
                    })
                },
            )
            .unwrap();
    }
    registry
        .action(
            "classify_task",
            "Apply the validated classification",
            CLASSIFY,
        )
        .unwrap();
    let clock = Arc::new(RecoveryClock(adapter.now.clone()));
    let slow_authority = adapter.slow_authority.clone();
    FlowHost::new(
        service(),
        adapter,
        Arc::new(GateAuthority {
            resume_gate,
            tool_grant: grant,
            slow_authority,
            trace: read_failure >= 17,
            trace_count: AtomicU64::new(0),
        }),
        Arc::new(Validator),
        clock,
    )
    .unwrap()
    .tools(registry)
    .unwrap()
    .install(
        Runtime::builder().clock(Arc::new(Clock)).resource(
            Task::definition()
                .action(CLASSIFY)
                .policy(|a, access, task| {
                    a.subject == "tool-owner"
                        && (!matches!(access, rom::Access::Read) || task.read_allowed)
                })
                .field_policy(|a, _, _, _| a.subject == "tool-owner"),
        ),
    )
    .unwrap()
    .build(storage, Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}
pub async fn deterministic_resume_race(redb: bool) {
    Box::pin(recovery_with_resume_gate(
        redb,
        4,
        Some(Arc::new(resume_race::Gate::default())),
    ))
    .await;
}
pub async fn recovery(redb: bool, mode: u8) {
    Box::pin(recovery_with_resume_gate(redb, mode, None)).await;
}
async fn recovery_with_resume_gate(
    redb: bool,
    mode: u8,
    resume_gate: Option<Arc<resume_race::Gate>>,
) {
    if mode >= 17 {
        eprintln!("read recovery entered mode {mode}");
    }
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-tools-recovery-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let raw: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    let read_admission = matches!(mode, 25..=29).then(|| {
        Arc::new(super::fault::ReadAdmissionFault {
            after_commit: mode == 26 || mode == 29,
            delivery_started: mode == 27,
            result_commit: mode == 28 || mode == 29,
            armed: AtomicBool::new(true),
        })
    });
    let fault = Arc::new(super::fault::ActionFault {
        inner: raw.clone(),
        armed: AtomicBool::new(mode < 3 || mode == 4),
        fail_hold: AtomicBool::new(mode == 2),
        trace_read: mode >= 17,
        read_admission: read_admission.clone(),
    });
    let adapter = Arc::new(Adapter {
        attempts: Mutex::new(vec![]),
        lookups: AtomicU64::new(0),
        repeat_call: mode == 9,
        reads: AtomicU64::new(0),
        blocked_read: (mode == 18).then(|| Arc::new(read_recovery::ReadBlock::new())),
        now: Arc::new(AtomicU64::new(1000)),
        read_admission,
        slow_authority: Arc::new(AtomicBool::new(false)),
    });
    let grant = Arc::new(AtomicBool::new(true));
    let (runtime, client) = recovery_host(
        adapter.clone(),
        grant.clone(),
        fault.clone(),
        mode == 8,
        mode == 10,
        mode,
    );
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
            .idempotency("create-task"),
        )
        .await
        .unwrap();
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .read::<String, String, _>("read_task", "Read the authorized task label", |_, input| {
            Box::pin(async move { Ok(input) })
        })
        .unwrap();
    registry
        .action(
            "classify_task",
            "Apply the validated classification",
            CLASSIFY,
        )
        .unwrap();
    let request = CompletionRequest::new(vec![Message::user("Read and classify the task")], 64)
        .unwrap()
        .with_tools(registry.descriptors())
        .unwrap();
    let handle = client
        .submit(
            &owner(),
            Submission {
                id: "tool-recovery".into(),
                idempotency: "submit-tool-recovery".into(),
                request: request.clone(),
                policy: RoutingPolicy::new(
                    1,
                    vec!["free".into()],
                    vec![],
                    None,
                    if mode == 24 {
                        RunLimits {
                            ticks: 4,
                            ..RunLimits::default()
                        }
                    } else {
                        RunLimits::default()
                    },
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();
    if mode >= 17 {
        eprintln!("read recovery submitted mode {mode}");
    }
    if mode == 18 {
        read_recovery::active_cpu(runtime, client, adapter, raw, handle).await;
        return;
    }
    for _ in 0..32 {
        runtime.process_work(1).await.unwrap();
        let view = client.view(&owner(), &handle).await.unwrap();
        if (mode == 3 && view.state() == &RunState::ToolsPending)
            || (mode < 3 && !fault.armed.load(Ordering::SeqCst))
            || (mode == 4 && !fault.armed.load(Ordering::SeqCst))
            || (mode == 5 && view.counters().tool_calls() == 2)
            || ((mode == 6 || mode == 7 || mode == 8) && view.state() == &RunState::Completed)
            || (mode == 9 && adapter.attempts.lock().unwrap().len() == 2)
            || (mode == 10 && !grant.load(Ordering::SeqCst))
            || (mode == 11 && view.state() == &RunState::Failed)
            || (mode == 23 && view.state() == &RunState::ToolsPending)
        {
            break;
        }
    }
    assert_eq!(
        adapter.attempts.lock().unwrap().len(),
        if (6..=9).contains(&mode) { 2 } else { 1 }
    );
    let task_revision = runtime
        .read::<Task>(&owner(), "task")
        .await
        .unwrap()
        .revision;
    assert_eq!(task_revision, if mode == 3 || mode >= 10 { 1 } else { 2 });
    if matches!(
        mode,
        17 | 19 | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 28 | 29 | 30 | 31 | 32
    ) {
        assert_eq!(
            adapter.reads.load(Ordering::SeqCst),
            if mode == 23 { 0 } else { 1 }
        );
        let before = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let original = before.active_attempt().unwrap().clone();
        assert_eq!(before.state(), &RunState::ToolsPending);
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(fault);
        drop(raw);
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let storage: Arc<dyn Storage> = if let Some(control) = &adapter.read_admission {
            Arc::new(super::fault::ActionFault {
                inner: storage,
                armed: AtomicBool::new(false),
                fail_hold: AtomicBool::new(false),
                trace_read: true,
                read_admission: Some(control.clone()),
            })
        } else {
            storage
        };
        let (runtime, client) =
            recovery_host(adapter.clone(), grant.clone(), storage, false, false, mode);
        let before = client.view(&owner(), &handle).await.unwrap();
        if mode == 31 || mode == 32 {
            use rom_ai::flow::{ReadStatus, RunView};
            let retained = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            let record = retained.value.as_ref().unwrap().record().unwrap();
            let pure = RunView::project(&record, retained.revision, &owner(), |a, o| {
                Authority.inspect(a, o)
            })
            .unwrap();
            assert_eq!(
                pure.read_progress().unwrap().status(),
                ReadStatus::ActivityUnknown
            );
            assert_eq!(pure.read_progress().unwrap().ordinal(), 1);
            if mode == 32 {
                let revoked = runtime
                    .execute(
                        &owner(),
                        Command::patch(
                            "task",
                            rom::Patch::new().set(Task::read_allowed_field(), false),
                        )
                        .at_revision(1)
                        .idempotency("revoke-row-read-progress"),
                    )
                    .await;
                assert!(
                    matches!(revoked, Err(rom::Error::Denied)),
                    "the new row grant denies committed outcome disclosure"
                );
                runtime
                    .establish_actor(|reads| {
                        let row = reads
                            .load(&rom::Key {
                                kind: Task::KIND.into(),
                                id: "task".into(),
                            })?
                            .ok_or(rom::Error::Missing)?;
                        assert_eq!(row.revision, 2);
                        assert_eq!(row.value.unwrap()["read_allowed"], false);
                        Ok(service())
                    })
                    .await
                    .unwrap();
                assert_eq!(
                    client.view(&owner(), &handle).await.unwrap_err(),
                    rom_ai::AiError::Denied
                );
                assert_eq!(
                    runtime
                        .read::<AiRun>(&service(), &handle.0)
                        .await
                        .unwrap()
                        .revision,
                    retained.revision
                );
                assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
                assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
                assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
                runtime.shutdown().await.unwrap();
                return;
            }
            assert_eq!(
                before.read_progress().unwrap().status(),
                ReadStatus::AwaitingRecovery
            );
            assert_eq!(before.read_progress().unwrap().ordinal(), 1);
            let encoded = serde_json::to_value(&before).unwrap();
            assert_eq!(
                encoded["read_progress"],
                serde_json::json!({"status":"AwaitingRecovery","ordinal":1})
            );
            assert_eq!(
                client
                    .view(&Actor::trusted("tool-tests", "different-owner"), &handle)
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Denied
            );
            grant.store(false, Ordering::SeqCst);
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap_err(),
                rom_ai::AiError::Denied
            );
            grant.store(true, Ordering::SeqCst);
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), &handle.0)
                    .await
                    .unwrap()
                    .revision,
                retained.revision
            );
            let queued = client
                .resume(&owner(), &handle, before.revision(), "read-progress-retry")
                .await
                .unwrap();
            assert_eq!(queued.read_progress().unwrap().status(), ReadStatus::Queued);
            assert_eq!(queued.read_progress().unwrap().ordinal(), 2);
            let woken = client
                .resume(&owner(), &handle, queued.revision(), "read-progress-wake")
                .await
                .unwrap();
            assert_eq!(woken.read_progress().unwrap().status(), ReadStatus::Queued);
            assert_eq!(woken.read_progress().unwrap().ordinal(), 2);
            assert_eq!(
                client
                    .resume(
                        &owner(),
                        &handle,
                        queued.revision(),
                        "stale-read-progress-snapshot"
                    )
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Conflict
            );
            for _ in 0..16 {
                runtime.process_work(1).await.unwrap();
                let current = client.view(&owner(), &handle).await.unwrap();
                if current.state() == &RunState::Completed {
                    assert!(current.read_progress().is_none());
                    break;
                }
            }
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap().state(),
                &RunState::Completed
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 30 {
            let retained = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            adapter.slow_authority.store(true, Ordering::SeqCst);
            let began = std::time::Instant::now();
            assert_eq!(
                client
                    .resume(
                        &owner(),
                        &handle,
                        before.revision(),
                        "read-retry-queued-authority-deadline"
                    )
                    .await
                    .unwrap_err(),
                rom_ai::AiError::UnknownOutcome
            );
            assert!(began.elapsed() < std::time::Duration::from_secs(4));
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            assert!(!adapter.slow_authority.load(Ordering::SeqCst));
            for _ in 0..100 {
                if runtime.status().unwrap().owned_work == 0 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            assert_eq!(runtime.status().unwrap().owned_work, 0);
            let unchanged = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            assert_eq!(unchanged.revision, retained.revision);
            assert_eq!(
                unchanged.value.unwrap().record().unwrap(),
                retained.value.unwrap().record().unwrap()
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_before.revision, account_after.revision);
            assert_eq!(
                account_before.value.unwrap().record().unwrap(),
                account_after.value.unwrap().record().unwrap()
            );
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                1
            );
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "read-retry-queued-authority-deadline",
                )
                .await
                .unwrap();
            for _ in 0..16 {
                runtime.process_work(1).await.unwrap();
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
            }
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap().state(),
                &RunState::Completed
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                2
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 28 || mode == 29 {
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "read-result-interrupted-retry",
                )
                .await
                .unwrap();
            for _ in 0..8 {
                let _ = runtime.process_work(1).await;
                if !adapter
                    .read_admission
                    .as_ref()
                    .unwrap()
                    .armed
                    .load(Ordering::SeqCst)
                {
                    break;
                }
            }
            assert!(
                !adapter
                    .read_admission
                    .as_ref()
                    .unwrap()
                    .armed
                    .load(Ordering::SeqCst)
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                1
            );
            let interrupted = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            let data = serde_json::to_value(&interrupted).unwrap();
            let step = &data["tool_turns"].as_array().unwrap().last().unwrap()["steps"][0];
            assert_eq!(step["read_attempt"]["ordinal"], 2);
            assert_eq!(
                step["read_attempt"]["phase"],
                if mode == 28 { "Started" } else { "Completed" }
            );
            assert_eq!(step["result"].is_null(), mode == 28);
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            runtime.shutdown().await.unwrap();
            drop(client);
            drop(runtime);
            let storage: Arc<dyn Storage> = if redb {
                Arc::new(rom_redb::Redb::open(&path).unwrap())
            } else {
                Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
            };
            let (runtime, client) =
                recovery_host(adapter.clone(), grant.clone(), storage, false, false, mode);
            let reopened = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(reopened, interrupted);
            if mode == 28 {
                let view = client.view(&owner(), &handle).await.unwrap();
                client
                    .resume(
                        &owner(),
                        &handle,
                        view.revision(),
                        "read-after-confirmed-result-noncommit",
                    )
                    .await
                    .unwrap();
            }
            for _ in 0..16 {
                runtime.process_work(1).await.unwrap();
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
            }
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap().state(),
                &RunState::Completed
            );
            assert_eq!(
                adapter.reads.load(Ordering::SeqCst),
                if mode == 28 { 3 } else { 2 }
            );
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                2
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            // One continuation adds its distinct reservation; the original settled entry is unchanged.
            let before_account = account_before.value.unwrap().record().unwrap();
            let after_account = account_after.value.unwrap().record().unwrap();
            assert_eq!(
                before_account
                    .entries()
                    .iter()
                    .find(|entry| entry.key() == original.key())
                    .unwrap(),
                after_account
                    .entries()
                    .iter()
                    .find(|entry| entry.key() == original.key())
                    .unwrap()
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 27 {
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "read-retry-claimed-before-callback",
                )
                .await
                .unwrap();
            let scheduled = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            for _ in 0..8 {
                let _ = runtime.process_work(1).await;
                if !adapter
                    .read_admission
                    .as_ref()
                    .unwrap()
                    .armed
                    .load(Ordering::SeqCst)
                {
                    break;
                }
            }
            assert!(
                !adapter
                    .read_admission
                    .as_ref()
                    .unwrap()
                    .armed
                    .load(Ordering::SeqCst)
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            let retained = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(retained, scheduled);
            runtime.shutdown().await.unwrap();
            drop(client);
            drop(runtime);
            let storage: Arc<dyn Storage> = if redb {
                Arc::new(rom_redb::Redb::open(&path).unwrap())
            } else {
                Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
            };
            let (runtime, client) =
                recovery_host(adapter.clone(), grant.clone(), storage, false, false, mode);
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            let view = client.view(&owner(), &handle).await.unwrap();
            client
                .resume(
                    &owner(),
                    &handle,
                    view.revision(),
                    "read-recover-original-scheduled-ordinal",
                )
                .await
                .unwrap();
            let woken = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            let woken_record = woken.value.unwrap().record().unwrap();
            let original_json = serde_json::to_value(&scheduled).unwrap();
            let woken_json = serde_json::to_value(&woken_record).unwrap();
            for field in [
                "tool_turns",
                "active_attempt",
                "request",
                "policy",
                "owner",
                "cursor",
                "checkpoint",
                "evidence_history",
                "usage_history",
                "validated_output",
                "cancel_requested",
            ] {
                assert_eq!(
                    original_json[field], woken_json[field],
                    "wake changed {field}"
                );
            }
            let old_ops = original_json["operations"].as_array().unwrap();
            let new_ops = woken_json["operations"].as_array().unwrap();
            assert_eq!(new_ops.len(), old_ops.len() + 1);
            assert_eq!(&new_ops[..old_ops.len()], old_ops);
            assert_eq!(
                woken_record.counters().ticks(),
                scheduled.counters().ticks() + 1
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_before.revision, account_after.revision);
            assert_eq!(
                account_before.value.unwrap().record().unwrap(),
                account_after.value.unwrap().record().unwrap()
            );
            client
                .resume(
                    &owner(),
                    &handle,
                    view.revision(),
                    "read-recover-original-scheduled-ordinal",
                )
                .await
                .unwrap();
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), &handle.0)
                    .await
                    .unwrap()
                    .revision,
                woken.revision
            );
            let (old, new) = tokio::join!(runtime.process_work(1), runtime.process_work(1));
            old.unwrap();
            new.unwrap();
            for _ in 0..16 {
                runtime.process_work(1).await.unwrap();
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
            }
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap().state(),
                &RunState::Completed
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                2
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 25 || mode == 26 {
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(
                client
                    .resume(
                        &owner(),
                        &handle,
                        before.revision(),
                        "read-retry-interrupted-admission"
                    )
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Storage
            );
            assert!(
                !adapter
                    .read_admission
                    .as_ref()
                    .unwrap()
                    .armed
                    .load(Ordering::SeqCst)
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_before.revision, account_after.revision);
            assert_eq!(
                account_before.value.unwrap().record().unwrap(),
                account_after.value.unwrap().record().unwrap()
            );
            runtime.shutdown().await.unwrap();
            drop(client);
            drop(runtime);
            let storage: Arc<dyn Storage> = if redb {
                Arc::new(rom_redb::Redb::open(&path).unwrap())
            } else {
                Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
            };
            let (runtime, client) =
                recovery_host(adapter.clone(), grant.clone(), storage, false, false, mode);
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "read-retry-interrupted-admission",
                )
                .await
                .unwrap();
            let admitted = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "read-retry-interrupted-admission",
                )
                .await
                .unwrap();
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), &handle.0)
                    .await
                    .unwrap()
                    .revision,
                admitted.revision
            );
            for _ in 0..16 {
                runtime.process_work(1).await.unwrap();
                if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                    break;
                }
            }
            assert_eq!(
                client.view(&owner(), &handle).await.unwrap().state(),
                &RunState::Completed
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                2
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 24 {
            for attempt in 0_u32..3 {
                let view = client.view(&owner(), &handle).await.unwrap();
                client
                    .resume(
                        &owner(),
                        &handle,
                        view.revision(),
                        &format!("bounded-read-retry-{attempt}"),
                    )
                    .await
                    .unwrap();
                for _ in 0..16 {
                    runtime.process_work(1).await.unwrap();
                    if adapter.reads.load(Ordering::SeqCst) == u64::from(attempt) + 2 {
                        break;
                    }
                }
            }
            let head = client.view(&owner(), &handle).await.unwrap();
            assert_eq!(head.counters().ticks(), 4);
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 4);
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            let outcome = client
                .resume(
                    &owner(),
                    &handle,
                    head.revision(),
                    "read-retry-beyond-frozen-budget",
                )
                .await;
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 4);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                1
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_after.revision, account_before.revision);
            assert_eq!(
                account_after.value.unwrap().record().unwrap(),
                account_before.value.unwrap().record().unwrap()
            );
            let refused = outcome.unwrap();
            assert_eq!(refused.state(), &RunState::Failed);
            assert_eq!(refused.failure(), Some(&rom_ai::AiError::BudgetExhausted));
            assert_eq!(refused.counters(), head.counters());
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 22 {
            let run_before = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            let other_owner = Actor::trusted("tool-tests", "different-owner");
            assert_eq!(
                client
                    .resume(
                        &other_owner,
                        &handle,
                        before.revision(),
                        "retry-as-different-owner"
                    )
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Denied
            );
            let run_after = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            assert_eq!(run_after.revision, run_before.revision);
            assert_eq!(
                run_after.value.unwrap().record().unwrap(),
                run_before.value.unwrap().record().unwrap()
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_after.revision, account_before.revision);
            assert_eq!(
                account_after.value.unwrap().record().unwrap(),
                account_before.value.unwrap().record().unwrap()
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 20 || mode == 21 {
            let refusal_before = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            if mode == 20 {
                grant.store(false, Ordering::SeqCst);
            } else {
                adapter.now.store(
                    before_record_expiry(&runtime, &handle).await,
                    Ordering::SeqCst,
                );
            }
            eprintln!("read-refusal fixture before public resume");
            let outcome = client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "retry-now-denied-read",
                )
                .await;
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            let before_unwrap = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(before_unwrap.revision, account_before.revision);
            assert_eq!(
                before_unwrap.value.as_ref().unwrap().record().unwrap(),
                account_before.value.as_ref().unwrap().record().unwrap()
            );
            let failure = outcome.unwrap();
            assert_eq!(failure.state(), &RunState::Failed);
            let refusal_after = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(refusal_after.checkpoint(), refusal_before.checkpoint());
            assert_eq!(refusal_after.counters(), refusal_before.counters());
            let before_json = serde_json::to_value(&refusal_before).unwrap();
            let after_json = serde_json::to_value(&refusal_after).unwrap();
            for field in [
                "operations",
                "tool_turns",
                "active_attempt",
                "evidence_history",
                "usage_history",
                "cursor",
                "owner",
                "policy",
                "request",
            ] {
                assert_eq!(
                    after_json[field], before_json[field],
                    "refusal preserves {field}"
                );
            }
            assert_eq!(
                failure.failure(),
                Some(&if mode == 20 {
                    rom_ai::AiError::Denied
                } else {
                    rom_ai::AiError::DeadlineExceeded
                })
            );
            assert!(failure.output().is_none());
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                1
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_after.revision, account_before.revision);
            assert_eq!(
                account_after.value.unwrap().record().unwrap(),
                account_before.value.unwrap().record().unwrap()
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 19 {
            let actor = owner();
            let first_future = client.resume(
                &actor,
                &handle,
                before.revision(),
                "retry-stopped-original-read",
            );
            let duplicate_future = client.resume(
                &actor,
                &handle,
                before.revision(),
                "retry-stopped-original-read",
            );
            eprintln!(
                "public resume future bytes: {} and {}",
                std::mem::size_of_val(&first_future),
                std::mem::size_of_val(&duplicate_future)
            );
            let (first, duplicate) =
                tokio::join!(Box::pin(first_future), Box::pin(duplicate_future));
            first.unwrap();
            duplicate.unwrap();
            let admitted = client.view(&actor, &handle).await.unwrap();
            let scheduled = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            let account_before = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(
                client
                    .resume(&actor, &handle, before.revision(), "stale-revision-wake")
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Conflict
            );
            assert_eq!(
                client
                    .resume(
                        &actor,
                        &handle,
                        admitted.revision(),
                        "retry-stopped-original-read"
                    )
                    .await
                    .unwrap_err(),
                rom_ai::AiError::Conflict,
                "same key cannot change original expected revision"
            );
            client
                .resume(
                    &actor,
                    &handle,
                    admitted.revision(),
                    "explicit-queued-read-wake",
                )
                .await
                .unwrap();
            let woken = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
            read_recovery::assert_wake(&scheduled, &woken.value.unwrap().record().unwrap());
            client
                .resume(
                    &actor,
                    &handle,
                    admitted.revision(),
                    "explicit-queued-read-wake",
                )
                .await
                .unwrap();
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), &handle.0)
                    .await
                    .unwrap()
                    .revision,
                woken.revision
            );
            let account_after = runtime
                .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
                .await
                .unwrap();
            assert_eq!(account_before.revision, account_after.revision);
            assert_eq!(
                account_before.value.unwrap().record().unwrap(),
                account_after.value.unwrap().record().unwrap()
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
        } else {
            client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "retry-stopped-original-read",
                )
                .await
                .unwrap();
        }
        if mode == 23 || mode == 19 {
            let (original, recovery) =
                tokio::join!(runtime.process_work(1), runtime.process_work(1));
            original.unwrap();
            recovery.unwrap();
        }
        for _ in 0..16 {
            runtime.process_work(1).await.unwrap();
            if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                break;
            }
        }
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        assert_eq!(
            adapter.reads.load(Ordering::SeqCst),
            if mode == 23 { 1 } else { 2 },
            "one first callback or exactly one explicitly authorized retry"
        );
        assert_eq!(
            adapter.attempts.lock().unwrap().len(),
            2,
            "one original generation and one continuation, no model restart"
        );
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            2
        );
        let after = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        if mode == 19 {
            let head = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .revision;
            let replay = client
                .resume(
                    &owner(),
                    &handle,
                    before.revision(),
                    "retry-stopped-original-read",
                )
                .await
                .unwrap();
            assert_eq!(replay.state(), &RunState::Completed);
            assert_eq!(
                runtime
                    .read::<AiRun>(&service(), &handle.0)
                    .await
                    .unwrap()
                    .revision,
                head
            );
            assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
        }
        assert_eq!(after.request(), &request);
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let original_entry = account
            .entries()
            .iter()
            .find(|entry| entry.key() == original.key())
            .unwrap();
        assert_eq!(original_entry.prepared(), original.prepared());
        assert_eq!(
            original_entry.status(),
            &rom_ai::flow::ReservationStatus::Settled {
                actual_cost: rom_ai::UsdNanos(0)
            }
        );
        runtime.shutdown().await.unwrap();
        return;
    } else if mode >= 12 {
        let view = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(view.state(), &RunState::ToolsPending);
        assert!(view.failure().is_none());
        assert!(view.output().is_none());
        assert_eq!(view.counters().tool_calls(), 0);
        let record = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let window = record.active_attempt().unwrap().key().account_window();
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), window)
            .await
            .unwrap();
        for _ in 0..4 {
            runtime.process_work(4).await.unwrap();
        }
        let after = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), window)
            .await
            .unwrap();
        assert_eq!(after.revision, account.revision);
        assert_eq!(
            after.value.unwrap().record().unwrap(),
            account.value.unwrap().record().unwrap()
        );
        assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            1
        );
    } else if mode == 11 {
        let view = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(view.state(), &RunState::Failed);
        assert_eq!(view.failure(), Some(&rom_ai::AiError::InvalidOutput));
        assert!(view.output().is_none());
        assert_eq!(view.counters().tool_calls(), 0);
        for _ in 0..4 {
            runtime.process_work(4).await.unwrap();
        }
        assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            1
        );
    } else if mode == 10 {
        let view = client.view(&owner(), &handle).await.unwrap();
        assert!(view.output().is_none());
        assert_eq!(
            view.counters().tool_calls(),
            0,
            "a revoked grant cannot publish a calculated private result"
        );
        assert_eq!(view.state(), &RunState::Failed);
        assert_eq!(view.failure(), Some(&rom_ai::AiError::Denied));
        for _ in 0..4 {
            runtime.process_work(4).await.unwrap();
        }
        assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            1
        );
    } else if mode == 9 {
        let held = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(
            held.state(),
            &RunState::AwaitingReconciliation,
            "valid provider knowledge must survive rejection of a globally repeated call ID"
        );
        assert!(held.output().is_none());
        let record = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let active = record.active_attempt().unwrap();
        let index = record
            .attempt_evidence()
            .iter()
            .position(|evidence| evidence.attempt_id() == active.prepared().identity())
            .unwrap();
        assert_eq!(
            record.attempt_evidence()[index].generation_id(),
            Some("gen-tool-2")
        );
        assert_eq!(
            record.attempt_usage()[index].cost,
            Some(rom_ai::UsdNanos(0))
        );
        assert_eq!(record.request(), &request);
        assert_eq!(record.counters().tool_calls(), 2);
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), active.key().account_window())
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let entry = account
            .entries()
            .iter()
            .find(|entry| entry.key() == active.key())
            .unwrap();
        assert_eq!(entry.prepared(), active.prepared());
        assert_eq!(
            entry.status(),
            &rom_ai::flow::ReservationStatus::Settled {
                actual_cost: rom_ai::UsdNanos(0)
            }
        );
        for _ in 0..4 {
            runtime.process_work(4).await.unwrap();
        }
        assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            2
        );
    } else if mode == 8 {
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        let attempts = adapter.attempts.lock().unwrap().clone();
        assert_eq!(attempts[1].request().messages()[2].content, "null");
        let record = runtime
            .read::<AiRun>(&service(), &handle.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(record.request(), &request);
        assert_eq!(record.counters().tool_calls(), 2);
    } else if mode == 5 {
        let before = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(before.state(), &RunState::ToolsPending);
        grant.store(false, Ordering::SeqCst);
        for _ in 0..8 {
            runtime.process_work(4).await.unwrap();
        }
        assert_eq!(
            adapter.attempts.lock().unwrap().len(),
            1,
            "retained tool output requires current grant before successor POST"
        );
        let failure = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(failure.state(), &RunState::Failed);
        assert_eq!(failure.failure(), Some(&rom_ai::AiError::Denied));
        assert!(failure.output().is_none());
    } else if mode == 6 || mode == 7 {
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        if mode == 7 {
            let revoked = runtime
                .execute(
                    &owner(),
                    Command::patch(
                        "task",
                        rom::Patch::new().set(Task::read_allowed_field(), false),
                    )
                    .at_revision(2)
                    .idempotency("revoke-source-read-grant"),
                )
                .await;
            assert!(
                matches!(revoked, Err(rom::Error::Denied)),
                "commit may succeed while outcome disclosure is denied by the new row grant"
            );
            let committed = raw
                .load(&rom::Key {
                    kind: Task::KIND.into(),
                    id: "task".into(),
                })
                .unwrap()
                .unwrap();
            assert_eq!(committed.revision, 3);
            assert_eq!(committed.value.unwrap()["read_allowed"], false);
            assert!(matches!(
                runtime.read::<Task>(&owner(), "task").await,
                Err(rom::Error::Denied)
            ));
            assert!(
                grant.load(Ordering::SeqCst),
                "actor/tool role remains valid while source row grant changes"
            );
        } else {
            grant.store(false, Ordering::SeqCst);
        }
        assert!(
            matches!(
                client.view(&owner(), &handle).await,
                Err(rom_ai::AiError::Denied)
            ),
            "owner identity does not authorize retained private tool output"
        );
        assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
    } else if mode == 2 || mode == 3 {
        let before = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(before.state(), &RunState::ToolsPending);
        let cancelled = client
            .cancel(&owner(), &handle, before.revision(), "cancel-tools")
            .await
            .unwrap();
        assert_eq!(
            cancelled.state(),
            if mode == 2 {
                &RunState::AwaitingReconciliation
            } else {
                &RunState::Cancelled
            }
        );
        assert!(cancelled.cancel_requested());
        assert!(cancelled.output().is_none());
        for _ in 0..4 {
            runtime.process_work(4).await.unwrap();
        }
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            task_revision
        );
        assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
    } else {
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::AwaitingReconciliation
        );
        drop(client);
        drop(runtime);
        drop(fault);
        drop(raw);
        let reopened: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        let (runtime, client) = recovery_host_with_resume_gate(
            adapter.clone(),
            grant.clone(),
            reopened.clone(),
            mode == 8,
            mode == 10,
            mode,
            resume_gate.clone(),
        );
        let before = client.view(&owner(), &handle).await.unwrap();
        if mode == 1 {
            grant.store(false, Ordering::SeqCst);
        }
        let before_money = reopened.journal_head(rom_ai::flow::AiBudget::KIND).unwrap();
        if let Some(gate) = resume_gate {
            let result = Box::pin(
                resume_race::Race {
                    runtime: &runtime,
                    client: &client,
                    storage: &reopened,
                    adapter: &adapter,
                    handle: &handle,
                    revision: before.revision(),
                    money: &before_money,
                    request: &request,
                    gate,
                }
                .run(),
            )
            .await;
            drop(client);
            drop(runtime);
            drop(reopened);
            if result.is_ok() {
                std::fs::remove_dir_all(&directory).unwrap();
            } else {
                eprintln!("resume race retained database: {}", directory.display());
            }
            result.unwrap();
            return;
        }
        let resumed = if mode == 4 {
            let actor = owner();
            let (first, second) = tokio::join!(
                client.resume(&actor, &handle, before.revision(), "resume-original-tool"),
                client.resume(&actor, &handle, before.revision(), "resume-original-tool")
            );
            assert!(first.is_ok() || second.is_ok());
            for result in [&first, &second] {
                assert!(result.is_ok() || matches!(result, Err(rom_ai::AiError::Conflict)));
            }
            if first.is_ok() { first } else { second }
        } else {
            client
                .resume(&owner(), &handle, before.revision(), "resume-original-tool")
                .await
        };
        if mode == 1 {
            assert!(
                matches!(resumed, Err(rom_ai::AiError::Denied)),
                "current tool grant must deny recovery despite valid owner"
            );
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        } else {
            resumed.unwrap();
            let head = reopened.journal_head(AiRun::KIND).unwrap();
            let work = reopened.reaction_records().unwrap();
            client
                .resume(&owner(), &handle, before.revision(), "resume-original-tool")
                .await
                .unwrap();
            assert_eq!(reopened.journal_head(AiRun::KIND).unwrap(), head);
            assert_eq!(reopened.reaction_records().unwrap(), work);
            assert!(
                reopened
                    .journal(
                        rom_ai::flow::AiBudget::KIND,
                        Some(&before_money),
                        64,
                        64 * 1024
                    )
                    .unwrap()
                    .events
                    .is_empty(),
                "recovery must not reserve or settle again; global cursor may advance for run-operation events"
            );
            for _ in 0..8 {
                runtime.process_work(4).await.unwrap();
            }
            let completed = client.view(&owner(), &handle).await.unwrap();
            assert_eq!(completed.state(), &RunState::Completed);
            assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
            assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
            assert_eq!(
                runtime
                    .read::<Task>(&owner(), "task")
                    .await
                    .unwrap()
                    .revision,
                2,
                "original receipt replay must not repeat mutation or check current revision first"
            );
            let record = runtime
                .read::<AiRun>(&service(), &handle.0)
                .await
                .unwrap()
                .value
                .unwrap()
                .record()
                .unwrap();
            assert_eq!(record.request(), &request);
            assert_eq!(record.counters().tool_calls(), 2);
        }
        drop(client);
        drop(runtime);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

pub async fn read_future_size() {
    read_recovery::future_size().await;
}

async fn before_record_expiry(runtime: &Runtime, handle: &rom_ai::flow::RunHandle) -> u64 {
    runtime
        .read::<AiRun>(&service(), &handle.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap()
        .expires_at_unix_ms()
}
