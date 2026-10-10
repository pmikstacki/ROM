//! Two unrelated external consumers use only the installed public flow/tool facade.
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_ai::flow::{AiRun, FlowAuthority, FlowHost, OwnerIdentity, RunState, Submission};
use rom_ai::{
    AiClock, AiError, AiFuture, AiResult, AttemptEvidence, CatalogModel, CatalogSnapshot,
    Completion, CompletionRequest, Deadline, Message, ModelPrice, OutputValidator, PreparedAttempt,
    Provider, RoutingPolicy, RunLimits, ToolCall, Usage,
};
use rom_ai_flows_consumer::{
    publication::{self, Draft, Edition, Head},
    triage::{self, Classification, Ticket},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[path = "flows/fault.rs"]
mod fault;
fn owner() -> Actor {
    Actor::trusted("external-ai-consumer", "author")
}
fn service() -> Actor {
    Actor::trusted("external-ai-consumer", "ai-service").with_kind(rom::PrincipalKind::Service)
}
struct Clock;
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        1000
    }
}
impl rom::Clock for Clock {
    fn now(&self) -> u64 {
        1
    }
}
struct Authority(Arc<AtomicBool>);
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        if actor.authority == "external-ai-consumer" && actor.subject == "author" {
            OwnerIdentity::from_actor(actor)
        } else {
            Err(AiError::Denied)
        }
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor)
            && actor.authority == "external-ai-consumer"
            && actor.subject == "author"
        {
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
        if identity.matches(actor)
            && identity.matches(&owner())
            && prepared.route().maximum_cost().0 == 0
        {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
    fn tool_actor(
        &self,
        actor: &Actor,
        identity: &OwnerIdentity,
        call: &ToolCall,
        reads: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if !self.0.load(Ordering::SeqCst) {
            return Err(rom::Error::Denied);
        }
        if !identity.matches(actor) || !identity.matches(&owner()) {
            return Err(rom::Error::Denied);
        }
        let args = call.arguments();
        let (kind, id) = match call.name() {
            "read_draft" => (Draft::KIND, args["input"].as_str()),
            "prepare_edition" => (Edition::KIND, args["target"].as_str()),
            "publish_head" => (Head::KIND, args["target"].as_str()),
            "read_ticket" => (Ticket::KIND, args["input"].as_str()),
            "classify_ticket" => (Ticket::KIND, args["target"].as_str()),
            _ => return Err(rom::Error::Denied),
        };
        let row = reads
            .load(&rom::Key {
                kind: kind.into(),
                id: id.ok_or(rom::Error::Denied)?.into(),
            })?
            .ok_or(rom::Error::Denied)?;
        let value = row.value.ok_or(rom::Error::Denied)?;
        if value["owner"].as_str() != Some(actor.subject.as_str()) {
            return Err(rom::Error::Denied);
        }
        if call.name() == "prepare_edition" {
            let draft = value["draft"].as_str().ok_or(rom::Error::Denied)?;
            let source = reads
                .load(&rom::Key {
                    kind: Draft::KIND.into(),
                    id: draft.into(),
                })?
                .and_then(|row| row.value)
                .ok_or(rom::Error::Denied)?;
            if source["owner"].as_str() != Some(actor.subject.as_str()) {
                return Err(rom::Error::Denied);
            }
        }
        if call.name() == "publish_head" {
            let edition = args["input"].as_str().ok_or(rom::Error::Denied)?;
            let prepared = reads
                .load(&rom::Key {
                    kind: Edition::KIND.into(),
                    id: edition.into(),
                })?
                .and_then(|row| row.value)
                .ok_or(rom::Error::Denied)?;
            if prepared["owner"].as_str() != Some(actor.subject.as_str())
                || prepared["prepared"] != true
            {
                return Err(rom::Error::Denied);
            }
        }
        Ok(actor.clone())
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        if value == &serde_json::json!({"complete":true}) {
            Ok(value.clone())
        } else {
            Err(AiError::InvalidOutput)
        }
    }
}
struct Adapter {
    publication: bool,
    attempts: Mutex<Vec<PreparedAttempt>>,
}
impl Provider for Adapter {
    fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, rom_ai::Reconciliation> {
        Box::pin(async { Ok(rom_ai::Reconciliation::Unresolved) })
    }
    fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
        Box::pin(async {
            CatalogSnapshot::new(
                "external-static-catalog",
                vec![CatalogModel::text(
                    "fixture/free",
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
                let mut list = self.attempts.lock().unwrap();
                list.push(attempt.clone());
                list.len()
            };
            let evidence = AttemptEvidence::new(
                attempt.identity(),
                None,
                Some(format!("gen-external-{index}")),
            )?;
            let usage = Usage {
                cost: Some(rom_ai::UsdNanos(0)),
                ..Default::default()
            };
            if index == 1 {
                let calls = if self.publication {
                    vec![
                        ToolCall::new(
                            "read-draft-1",
                            "read_draft",
                            serde_json::json!({"input":"draft"}),
                        )?,
                        ToolCall::new(
                            "prepare-edition-1",
                            "prepare_edition",
                            serde_json::json!({"target":"edition","expected_revision":1,"input":{"draft_revision":1,"text":"First historical article"}}),
                        )?,
                        ToolCall::new(
                            "publish-head-1",
                            "publish_head",
                            serde_json::json!({"target":"head","expected_revision":1,"input":"edition"}),
                        )?,
                    ]
                } else {
                    vec![
                        ToolCall::new(
                            "read-ticket-1",
                            "read_ticket",
                            serde_json::json!({"input":"ticket"}),
                        )?,
                        ToolCall::new(
                            "classify-ticket-1",
                            "classify_ticket",
                            serde_json::json!({"target":"ticket","expected_revision":1,"input":{"classification":"urgent"}}),
                        )?,
                    ]
                };
                Completion::tool_calls(calls, usage, evidence)
            } else {
                Completion::output(serde_json::json!({"complete":true}), usage, evidence)
            }
        })
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_publication_flow_prepares_then_publishes_with_retained_sources() {
    journey(false, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_publication_flow_prepares_then_publishes_with_retained_sources() {
    journey(true, true).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_public_ticket_triage_flow_reads_then_classifies_once() {
    journey(false, false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_public_ticket_triage_flow_reads_then_classifies_once() {
    journey(true, false).await;
}
async fn journey(redb: bool, publication_flow: bool) {
    scenario(redb, publication_flow, 0).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_external_consumers_recover_original_tool_receipt_after_lost_ack() {
    for publication in [false, true] {
        scenario(false, publication, 1).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_external_consumers_recover_original_tool_receipt_after_lost_ack() {
    for publication in [false, true] {
        scenario(true, publication, 1).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_external_consumers_recheck_current_tool_grant_before_receipt_recovery() {
    for publication in [false, true] {
        scenario(false, publication, 2).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_external_consumers_recheck_current_tool_grant_before_receipt_recovery() {
    for publication in [false, true] {
        scenario(true, publication, 2).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_external_consumers_cancel_unknown_tool_effect_without_resetting_history() {
    for publication in [false, true] {
        scenario(false, publication, 3).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_external_consumers_cancel_unknown_tool_effect_without_resetting_history() {
    for publication in [false, true] {
        scenario(true, publication, 3).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_external_consumers_report_generation_budget_after_committed_domain_stage() {
    for publication in [false, true] {
        scenario(false, publication, 4).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_external_consumers_report_generation_budget_after_committed_domain_stage() {
    for publication in [false, true] {
        scenario(true, publication, 4).await;
    }
}
async fn scenario(redb: bool, publication_flow: bool, mode: u8) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-external-flow-{}-{}",
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
    let injected = Arc::new(fault::ActionFault {
        inner: raw.clone(),
        armed: AtomicBool::new((1..=3).contains(&mode)),
        publication: publication_flow,
    });
    let mut storage: Arc<dyn Storage> = injected.clone();
    let grant = Arc::new(AtomicBool::new(true));
    let registry = if publication_flow {
        publication::tools(1)
    } else {
        triage::tools(1)
    }
    .unwrap();
    let original = CompletionRequest::new(
        vec![Message::user(if publication_flow {
            "Publish the captured draft revision"
        } else {
            "Classify this authorized ticket"
        })],
        128,
    )
    .unwrap()
    .with_tools(registry.descriptors())
    .unwrap();
    let adapter = Arc::new(Adapter {
        publication: publication_flow,
        attempts: Mutex::new(vec![]),
    });
    let (mut runtime, mut client) = build(
        storage.clone(),
        adapter.clone(),
        publication_flow,
        grant.clone(),
    );
    if publication_flow {
        runtime
            .execute(
                &owner(),
                Command::create(
                    "draft",
                    Draft {
                        owner: "author".into(),
                        text: "First historical article".into(),
                    },
                )
                .idempotency("create-draft"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &owner(),
                Command::create(
                    "edition",
                    Edition {
                        owner: "author".into(),
                        draft: rom::ResourceRef::new("draft").unwrap(),
                        draft_revision: 1,
                        text: "First historical article".into(),
                        prepared: false,
                    },
                )
                .idempotency("capture-edition"),
            )
            .await
            .unwrap();
        runtime
            .execute(
                &owner(),
                Command::create(
                    "head",
                    Head {
                        owner: "author".into(),
                        edition: None,
                    },
                )
                .idempotency("create-head"),
            )
            .await
            .unwrap();
    } else {
        runtime
            .execute(
                &owner(),
                Command::create(
                    "ticket",
                    Ticket {
                        owner: "author".into(),
                        body: "Pump requires inspection".into(),
                        classification: Classification::new("unclassified").unwrap(),
                    },
                )
                .idempotency("create-ticket"),
            )
            .await
            .unwrap();
    }
    let run = client
        .submit(
            &owner(),
            Submission {
                id: "external-run".into(),
                idempotency: "explicit-fresh-run".into(),
                request: original.clone(),
                policy: RoutingPolicy::new(
                    1,
                    vec!["fixture/free".into()],
                    vec![],
                    None,
                    RunLimits {
                        generation_attempts: if mode == 4 { 1 } else { 8 },
                        ..Default::default()
                    },
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();
    let mut intermediate = false;
    if mode == 0 {
        assert!(
            client
                .view(&owner(), &run)
                .await
                .unwrap()
                .read_progress()
                .is_none()
        );
        let mut observed = false;
        for _ in 0..8 {
            runtime.process_work(1).await.unwrap();
            let view = client.view(&owner(), &run).await.unwrap();
            if let Some(progress) = view.read_progress() {
                assert_eq!(progress.status(), rom_ai::flow::ReadStatus::Queued);
                assert_eq!(progress.ordinal(), 1);
                assert_eq!(
                    serde_json::to_value(progress).unwrap(),
                    serde_json::json!({"status":"Queued","ordinal":1})
                );
                observed = true;
                break;
            }
        }
        assert!(
            observed,
            "the public author observes an admitted read without operator grants"
        );
    }
    if (1..=3).contains(&mode) {
        for _ in 0..16 {
            runtime.process_work(1).await.unwrap();
            if !injected.armed.load(Ordering::SeqCst) {
                break;
            }
        }
        assert!(
            !injected.armed.load(Ordering::SeqCst),
            "the application commit actually happened before its lost acknowledgement"
        );
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert!(view.output().is_none());
        assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
        let kind = if publication_flow {
            Edition::KIND
        } else {
            Ticket::KIND
        };
        let journal = storage
            .journal(kind, None, 256, 1024 * 1024)
            .unwrap()
            .events;
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(storage);
        drop(injected);
        drop(raw);
        storage = if redb {
            Arc::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        (runtime, client) = build(
            storage.clone(),
            adapter.clone(),
            publication_flow,
            grant.clone(),
        );
        let view = client.view(&owner(), &run).await.unwrap();
        if mode == 2 {
            grant.store(false, Ordering::SeqCst);
            assert_eq!(
                client
                    .resume(&owner(), &run, view.revision(), "current-grant-recovery")
                    .await
                    .unwrap_err(),
                AiError::Denied
            );
            for _ in 0..4 {
                runtime.process_work(1).await.unwrap();
            }
            assert_eq!(
                storage
                    .journal(kind, None, 256, 1024 * 1024)
                    .unwrap()
                    .events,
                journal
            );
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert_eq!(
                client.view(&owner(), &run).await.unwrap().state(),
                &RunState::AwaitingReconciliation
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        if mode == 3 {
            let cancelled = client
                .cancel(
                    &owner(),
                    &run,
                    view.revision(),
                    "cancel-unknown-domain-effect",
                )
                .await
                .unwrap();
            assert_eq!(cancelled.state(), &RunState::AwaitingReconciliation);
            assert!(cancelled.cancel_requested());
            for _ in 0..4 {
                runtime.process_work(1).await.unwrap();
            }
            assert_eq!(
                storage
                    .journal(kind, None, 256, 1024 * 1024)
                    .unwrap()
                    .events,
                journal
            );
            assert_eq!(adapter.attempts.lock().unwrap().len(), 1);
            assert!(
                client
                    .view(&owner(), &run)
                    .await
                    .unwrap()
                    .output()
                    .is_none()
            );
            runtime.shutdown().await.unwrap();
            return;
        }
        client
            .resume(
                &owner(),
                &run,
                view.revision(),
                "recover-original-domain-command",
            )
            .await
            .unwrap();
        assert_eq!(
            storage
                .journal(kind, None, 256, 1024 * 1024)
                .unwrap()
                .events,
            journal,
            "receipt replay cannot recommit the original domain action"
        );
        intermediate = publication_flow;
    }
    for _ in 0..24 {
        runtime.process_work(1).await.unwrap();
        if publication_flow {
            let edition = runtime
                .read::<Edition>(&owner(), "edition")
                .await
                .unwrap()
                .value
                .unwrap();
            let head = runtime
                .read::<Head>(&owner(), "head")
                .await
                .unwrap()
                .value
                .unwrap();
            intermediate |= edition.prepared && head.edition.is_none();
        }
        if matches!(
            client.view(&owner(), &run).await.unwrap().state(),
            RunState::Completed | RunState::Failed
        ) {
            break;
        }
    }
    let view = client.view(&owner(), &run).await.unwrap();
    assert!(view.read_progress().is_none());
    if mode == 4 {
        // The active attempt identifies the host's private free ledger; it is not an authorization grant.
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(
                &service(),
                record.active_attempt().unwrap().key().account_window(),
            )
            .await
            .unwrap();
        assert_eq!(
            account.value.unwrap().record().unwrap().entries().len(),
            1,
            "generation limit refuses a successor before creating an orphan reservation"
        );
    }
    assert_eq!(
        view.state(),
        if mode == 4 {
            &RunState::Failed
        } else {
            &RunState::Completed
        }
    );
    if mode == 4 {
        assert_eq!(view.failure(), Some(&AiError::BudgetExhausted));
        assert!(view.output().is_none());
    }
    assert_eq!(
        view.counters().tool_calls(),
        if publication_flow { 3 } else { 2 }
    );
    let record = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.request(), &original);
    assert_eq!(
        adapter.attempts.lock().unwrap().len(),
        if mode == 4 { 1 } else { 2 }
    );
    if publication_flow {
        assert!(intermediate);
        let citation = runtime
            .read::<Head>(&owner(), "head")
            .await
            .unwrap()
            .value
            .unwrap()
            .edition
            .unwrap();
        assert_eq!(citation.id(), "edition");
        assert_eq!(
            runtime
                .read::<Edition>(&owner(), citation.id())
                .await
                .unwrap()
                .value
                .unwrap()
                .text,
            "First historical article"
        );
        assert_eq!(
            runtime
                .read::<Head>(&owner(), "head")
                .await
                .unwrap()
                .revision,
            2
        );
        runtime
            .execute(
                &owner(),
                Command::patch(
                    "draft",
                    rom::Patch::new()
                        .set(Draft::text_field(), "Revised current article".to_owned()),
                )
                .at_revision(1)
                .idempotency("edit-current-source-after-publication"),
            )
            .await
            .unwrap();
        assert_eq!(
            runtime
                .read::<Draft>(&owner(), "draft")
                .await
                .unwrap()
                .value
                .unwrap()
                .text,
            "Revised current article"
        );
        assert_eq!(
            runtime
                .read::<Edition>(&owner(), citation.id())
                .await
                .unwrap()
                .value
                .unwrap()
                .text,
            "First historical article",
            "editing the current source cannot replace the already published citation"
        );
    } else {
        assert_eq!(
            runtime
                .read::<Ticket>(&owner(), "ticket")
                .await
                .unwrap()
                .revision,
            2
        );
    }
    runtime.process_work(8).await.unwrap();
    assert_eq!(
        adapter.attempts.lock().unwrap().len(),
        if mode == 4 { 1 } else { 2 }
    );
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

fn build(
    storage: Arc<dyn Storage>,
    adapter: Arc<Adapter>,
    publication_flow: bool,
    grant: Arc<AtomicBool>,
) -> (Runtime, rom_ai::flow::FlowClient) {
    let registry = if publication_flow {
        publication::tools(1)
    } else {
        triage::tools(1)
    }
    .unwrap();
    let builder = if publication_flow {
        publication::register(Runtime::builder())
    } else {
        triage::register(Runtime::builder())
    }
    .clock(Arc::new(Clock));
    FlowHost::new(
        service(),
        adapter,
        Arc::new(Authority(grant)),
        Arc::new(Validator),
        Arc::new(Clock),
    )
    .unwrap()
    .tools(registry)
    .unwrap()
    .install(builder)
    .unwrap()
    .build(storage, Runtime::shared_cpu_pool(2).unwrap())
    .unwrap()
}
