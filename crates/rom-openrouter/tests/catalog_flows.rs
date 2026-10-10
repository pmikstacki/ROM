//! Demand-aware routing exercises real HTTP and public flows over both durable adapters.
#![cfg(feature = "test-support")]
#[path = "support/credentials.rs"]
mod credentials;
use credentials::Credentials;
#[path = "support/router.rs"]
mod router;
use rom::{Actor, Runtime, Storage};
use rom_ai::flow::{AiRun, FlowAuthority, FlowHost, OwnerIdentity, RunState, Submission};
use rom_ai::{
    AiClock, AiError, AiResult, CatalogSnapshot, CompletionRequest, Deadline, Message,
    OutputSchema, OutputValidator, PreparedAttempt, Provider, RoutingPolicy, RunLimits,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
fn owner() -> Actor {
    Actor::trusted("fixture", "owner")
}
fn is_owner(actor: &Actor) -> bool {
    OwnerIdentity::from_actor(&owner()).unwrap().matches(actor)
}
fn service() -> Actor {
    Actor::trusted("fixture", "ai-host").with_kind(rom::PrincipalKind::Service)
}
struct Clock;
impl AiClock for Clock {
    fn now_unix_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
struct Authority;
impl FlowAuthority for Authority {
    fn submit(&self, actor: &Actor, _: &Submission) -> AiResult<OwnerIdentity> {
        if is_owner(actor) {
            OwnerIdentity::from_actor(actor)
        } else {
            Err(AiError::Denied)
        }
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor) && is_owner(actor) {
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
        if identity.matches(actor) && is_owner(actor) && prepared.route().maximum_cost().0 == 0 {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        if value == &serde_json::json!({"answer":"safe"}) {
            Ok(value.clone())
        } else {
            Err(AiError::InvalidOutput)
        }
    }
}
fn input() -> (CompletionRequest, RoutingPolicy) {
    let request=CompletionRequest::new(vec![Message::user("private fixture")],128).unwrap().with_schema(OutputSchema::new(1,"answer",serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"additionalProperties":false})).unwrap()).unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec!["fixture/bad".into(), "fixture/alternate".into()],
        Vec::new(),
        None,
        RunLimits::default(),
    )
    .unwrap()
    .with_allowed_providers(vec!["trusted-provider".into()])
    .unwrap();
    (request, policy)
}
fn provider(invalid: u8) -> (router::Router, rom_openrouter::OpenRouter) {
    let models:Vec<_>=["fixture/bad","fixture/alternate"].into_iter().map(|id|serde_json::json!({"id":id,"context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["structured_outputs"],"pricing":{"prompt":"0","completion":"0","request":"0"}})).collect();
    let mut routes = BTreeMap::new();
    routes.insert(
        "/api/v1/models".into(),
        serde_json::json!({"data":models}).to_string(),
    );
    for id in ["fixture/bad", "fixture/alternate"] {
        let incapable = id == "fixture/bad" && invalid == 2;
        let price = if id == "fixture/bad" && invalid == 1 {
            "0.000000001"
        } else {
            "0"
        };
        routes.insert(format!("/api/v1/models/{id}/endpoints"),serde_json::json!({"data":{"id":id,"endpoints":[{"tag":"trusted-provider","model_id":id,"context_length":8192,"max_prompt_tokens":8192,"max_completion_tokens":4096,"supported_parameters":if incapable{vec!["max_tokens"]}else{vec!["max_tokens","structured_outputs"]},"pricing":{"prompt":price,"completion":"0","request":"0"}}]}}).to_string());
    }
    routes.insert("/api/v1/chat/completions".into(),r#"{"id":"gen-alternate","model":"fixture/alternate","choices":[{"finish_reason":"stop","message":{"content":"{\"answer\":\"safe\"}"}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0}}"#.into());
    let server = router::Router::new(routes);
    let adapter = server.provider();
    (server, adapter)
}
#[tokio::test]
async fn catalog_for_excludes_paid_only_and_incapable_whitelisted_endpoints() {
    for invalid in [1, 2] {
        let (server, adapter) = provider(invalid);
        let (request, policy) = input();
        let now = Clock.now_unix_ms();
        let snapshot: CatalogSnapshot = adapter
            .catalog_for(
                &request,
                &policy,
                Deadline::remaining(now, now, now + 1000).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            snapshot
                .models()
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["fixture/alternate"]
        );
        assert!(
            server
                .requests()
                .iter()
                .all(|bytes| !bytes.starts_with(b"POST "))
        );
    }
}
#[tokio::test]
async fn explicit_host_set_cannot_broaden_to_policy_alternate() {
    let (_server, adapter) = provider(1);
    let adapter = adapter
        .with_endpoint_models(vec!["fixture/bad".into()])
        .unwrap();
    let (request, policy) = input();
    let now = Clock.now_unix_ms();
    let snapshot = adapter
        .catalog_for(
            &request,
            &policy,
            Deadline::remaining(now, now, now + 1000).unwrap(),
        )
        .await
        .unwrap();
    assert!(snapshot.models().is_empty());
}
#[tokio::test]
async fn shared_raw_cache_reapplies_each_whitelist_and_request() {
    let (_server, adapter) = provider(2);
    let (request, policy) = input();
    let now = Clock.now_unix_ms();
    let first = adapter
        .catalog_for(
            &request,
            &policy,
            Deadline::remaining(now, now, now + 1000).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.models().len(), 1);
    let deny = policy
        .clone()
        .with_allowed_providers(vec!["other-provider".into()])
        .unwrap();
    let now = Clock.now_unix_ms();
    assert!(
        adapter
            .catalog_for(
                &request,
                &deny,
                Deadline::remaining(now, now, now + 1000).unwrap()
            )
            .await
            .unwrap()
            .models()
            .is_empty()
    );
    let plain = CompletionRequest::new(vec![Message::user("private fixture")], 128).unwrap();
    let now = Clock.now_unix_ms();
    let broader = adapter
        .catalog_for(
            &plain,
            &policy,
            Deadline::remaining(now, now, now + 1000).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(broader.models().len(), 2);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_actual_http_flow_selects_alternate_eligible_endpoint() {
    flow(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_actual_http_flow_selects_alternate_eligible_endpoint() {
    flow(true).await;
}
async fn flow(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for invalid in [1, 2] {
        let path = std::env::temp_dir().join(format!(
            "rom-openrouter-flow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        let storage: Arc<dyn Storage> = if redb {
            Arc::new(rom_redb::Redb::open(path.join("database")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(path.join("database")).unwrap())
        };
        let (server, adapter) = provider(invalid);
        let (runtime, client) = FlowHost::new(
            service(),
            Arc::new(adapter),
            Arc::new(Authority),
            Arc::new(Validator),
            Arc::new(Clock),
        )
        .unwrap()
        .install(Runtime::builder())
        .unwrap()
        .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
        let (request, policy) = input();
        let run = client
            .submit(
                &owner(),
                Submission {
                    id: "actual-route".into(),
                    idempotency: "actual-route-submit".into(),
                    request,
                    policy,
                },
            )
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::Completed);
        assert_eq!(view.output(), Some(&serde_json::json!({"answer":"safe"})));
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            record.active_attempt().unwrap().prepared().route().model(),
            "fixture/alternate"
        );
        let posts: Vec<_> = server
            .requests()
            .into_iter()
            .filter(|bytes| bytes.starts_with(b"POST "))
            .collect();
        assert_eq!(posts.len(), 1);
        let position = posts[0]
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&posts[0][position + 4..]).unwrap();
        assert_eq!(body["model"], "fixture/alternate");
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(storage);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[tokio::test]
async fn one_catalog_call_discovers_all_six_candidates_with_four_concurrent_refreshes() {
    let ids: Vec<_> = (0..6)
        .map(|index| format!("fixture/model-{index}"))
        .collect();
    let mut routes = BTreeMap::new();
    let models:Vec<_>=ids.iter().map(|id|serde_json::json!({"id":id,"context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["structured_outputs"],"pricing":{"prompt":"0","completion":"0","request":"0"}})).collect();
    routes.insert(
        "/api/v1/models".into(),
        serde_json::json!({"data":models}).to_string(),
    );
    for id in &ids {
        routes.insert(format!("/api/v1/models/{id}/endpoints"),serde_json::json!({"data":{"id":id,"endpoints":[{"tag":"trusted-provider","model_id":id,"context_length":8192,"supported_parameters":["max_tokens","structured_outputs"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}}).to_string());
    }
    let server = router::Router::delayed(routes, std::time::Duration::from_millis(40));
    let adapter = server.provider();
    let (request, _) = input();
    let policy =
        RoutingPolicy::new(1, ids.clone(), Vec::new(), None, RunLimits::default()).unwrap();
    let now = Clock.now_unix_ms();
    let catalog = adapter
        .catalog_for(
            &request,
            &policy,
            Deadline::remaining(now, now, now + 1000).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        catalog
            .models()
            .iter()
            .map(|model| model.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(server.requests().len(), 7);
    assert!((2..=4).contains(&server.peak_connections()));
}

struct MetadataAuthority(Arc<std::sync::atomic::AtomicBool>);
impl FlowAuthority for MetadataAuthority {
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
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        prepared.validate().map_err(|_| rom::Error::Denied)?;
        if identity.matches(actor)
            && is_owner(actor)
            && self.0.load(Ordering::SeqCst)
            && prepared.policy().budget_reference() == Some("wire-metadata-account")
            && prepared.route().maximum_cost() == rom_ai::UsdNanos(10)
        {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_native_metadata_recovery_holds_output_and_checks_current_budget_grant() {
    native_metadata_flow(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_native_metadata_recovery_holds_output_and_checks_current_budget_grant() {
    native_metadata_flow(true).await;
}
async fn native_metadata_flow(redb: bool) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for revoke in [false, true] {
        let path = std::env::temp_dir().join(format!(
            "rom-openrouter-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        let open = || -> Arc<dyn Storage> {
            if redb {
                Arc::new(rom_redb::Redb::open(path.join("database")).unwrap())
            } else {
                Arc::new(rom_sqlite::Sqlite::open(path.join("database")).unwrap())
            }
        };
        let mut routes = BTreeMap::new();
        routes.insert("/api/v1/models".into(),r#"{"data":[{"id":"fixture/model","context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["structured_outputs"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}"#.into());
        routes.insert("/api/v1/models/fixture/model/endpoints".into(),r#"{"data":{"id":"fixture/model","endpoints":[{"tag":"trusted-provider","model_id":"fixture/model","context_length":8192,"supported_parameters":["max_tokens","structured_outputs"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}}"#.into());
        routes.insert(
            "/api/v1/chat/completions".into(),
            r#"{"id":"gen-original","error":{"code":500,"message":"fixture accepted error"}}"#
                .into(),
        );
        routes.insert("/api/v1/generation?id=gen-original".into(),r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":0.000000003,"native_tokens_prompt":4,"native_tokens_completion":1}}"#.into());
        let server = router::Router::new(routes);
        let adapter = Arc::new(server.provider());
        let grant = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let build = |storage: Arc<dyn Storage>| {
            FlowHost::new(
                service(),
                adapter.clone(),
                Arc::new(MetadataAuthority(grant.clone())),
                Arc::new(Validator),
                Arc::new(Clock),
            )
            .unwrap()
            .install(Runtime::builder())
            .unwrap()
            .build(storage, Runtime::shared_cpu_pool(2).unwrap())
            .unwrap()
        };
        let (runtime, client) = build(open());
        runtime
            .execute(
                &service(),
                rom::Command::create(
                    "wire-metadata-account",
                    rom_ai::flow::AiBudget::new(
                        &service(),
                        "wire-metadata-account",
                        rom_ai::UsdNanos(20),
                    )
                    .unwrap(),
                )
                .idempotency("wire-metadata-account-create"),
            )
            .await
            .unwrap();
        let (request, _) = input();
        let policy = RoutingPolicy::new(
            1,
            Vec::new(),
            vec!["fixture/model".into()],
            Some(rom_ai::ModelPrice::new(
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(0),
                rom_ai::UsdNanos(10),
            )),
            RunLimits::default(),
        )
        .unwrap()
        .with_budget_reference("wire-metadata-account")
        .unwrap();
        let run = client
            .submit(
                &owner(),
                Submission {
                    id: "wire-metadata-run".into(),
                    idempotency: "wire-metadata-submit".into(),
                    request,
                    policy,
                },
            )
            .await
            .unwrap();
        runtime.process_work(8).await.unwrap();
        let held = client.view(&owner(), &run).await.unwrap();
        assert_eq!(held.state(), &RunState::AwaitingReconciliation);
        if revoke {
            grant.store(false, Ordering::SeqCst);
        }
        let result = client
            .resume(&owner(), &run, held.revision(), "wire-metadata-lookup")
            .await;
        if revoke {
            assert_eq!(result, Err(AiError::Denied));
        } else {
            assert!(result.is_ok(), "native metadata result: {result:?}");
        }
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), "wire-metadata-account")
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            account.reserved(),
            rom_ai::UsdNanos(if revoke { 10 } else { 0 })
        );
        assert_eq!(
            account.settled(),
            rom_ai::UsdNanos(if revoke { 0 } else { 3 })
        );
        let view = client.view(&owner(), &run).await.unwrap();
        assert_eq!(view.state(), &RunState::AwaitingReconciliation);
        assert!(view.output().is_none());
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        let (runtime, client) = build(open());
        if !revoke {
            client
                .resume(&owner(), &run, held.revision(), "wire-metadata-lookup")
                .await
                .unwrap();
        }
        assert_eq!(
            server
                .requests()
                .iter()
                .filter(|bytes| bytes.starts_with(b"POST "))
                .count(),
            1
        );
        assert_eq!(
            server
                .requests()
                .iter()
                .filter(|bytes| bytes.starts_with(b"GET /api/v1/generation?"))
                .count(),
            usize::from(!revoke)
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
        drop(client);
        drop(runtime);
        std::fs::remove_dir_all(path).unwrap();
    }
}
