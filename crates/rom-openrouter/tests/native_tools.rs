//! Real HTTP native continuation through public typed tools and both durable adapters.
#![cfg(feature = "test-support")]
mod support;
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_ai::flow::{AiRun, FlowAuthority, FlowHost, OwnerIdentity, RunState, Submission};
use rom_ai::{
    AiClock, AiError, AiResult, CompletionRequest, Message, OutputValidator, PreparedAttempt,
    RoutingPolicy, RunLimits, ToolRegistry,
};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, rom::Resource)]
#[resource(name = "native_tool_note", version = 1)]
struct Note {
    text: String,
}
fn owner() -> Actor {
    Actor::trusted("native-tools", "owner")
}
fn service() -> Actor {
    Actor::trusted("native-tools", "service").with_kind(rom::PrincipalKind::Service)
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
        if OwnerIdentity::from_actor(&owner())?.matches(actor) {
            OwnerIdentity::from_actor(actor)
        } else {
            Err(AiError::Denied)
        }
    }
    fn inspect(&self, actor: &Actor, identity: &OwnerIdentity) -> AiResult<()> {
        if identity.matches(actor) && OwnerIdentity::from_actor(&owner())?.matches(actor) {
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
        call: &rom_ai::ToolCall,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<Actor> {
        if identity.matches(actor) && identity.matches(&owner()) && call.name() == "read_note" {
            Ok(actor.clone())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
struct Validator;
impl OutputValidator for Validator {
    fn validate(&self, value: &serde_json::Value) -> AiResult<serde_json::Value> {
        // This request uses text output, so the consumer validates and decodes its JSON text.
        let decoded = match value {
            serde_json::Value::String(text) => {
                serde_json::from_str(text).map_err(|_| AiError::InvalidOutput)?
            }
            value => value.clone(),
        };
        if decoded == serde_json::json!({"answer":"safe"}) {
            Ok(decoded)
        } else {
            Err(AiError::InvalidOutput)
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_real_native_tools_continue_with_fresh_catalog_without_resetting_route() {
    journey(false, 0).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_real_native_tools_continue_with_fresh_catalog_without_resetting_route() {
    journey(true, 0).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_current_endpoint_ineligible_for_tool_context_fails_without_successor_post() {
    journey(false, 1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_current_endpoint_ineligible_for_tool_context_fails_without_successor_post() {
    journey(true, 1).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sqlite_escaped_native_arguments_fail_preparation_without_successor_post_or_reservation() {
    journey(false, 2).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redb_escaped_native_arguments_fail_preparation_without_successor_post_or_reservation() {
    journey(true, 2).await;
}
async fn journey(redb: bool, mode: u8) {
    let ineligible = mode == 1;
    let escaped = mode == 2;
    let model = "fixture/native";
    let models = serde_json::json!({"data":[{"id":model,"context_length":262144,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["tools"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]});
    let endpoints = serde_json::json!({"data":{"id":model,"endpoints":[{"tag":"trusted-provider","model_id":model,"context_length":262144,"max_prompt_tokens":if ineligible {3000}else{262144},"max_completion_tokens":4096,"supported_parameters":["max_tokens","tools"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}});
    let arguments = serde_json::to_string(
        &serde_json::json!({"input":if escaped {"\\".repeat(512)}else{"note".into()}}),
    )
    .unwrap();
    let first = serde_json::json!({"id":"gen-native-1","model":model,"choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[{"id":"note-1","type":"function","function":{"name":"read_note","arguments":arguments}}]}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0}});
    let last = serde_json::json!({"id":"gen-native-2","model":model,"choices":[{"finish_reason":"stop","message":{"content":"{\"answer\":\"safe\"}"}}],"usage":{"prompt_tokens":5,"completion_tokens":1,"cost":0}});
    let server = support::Server::sequence(
        vec![models, endpoints, first, last]
            .into_iter()
            .map(|body| {
                (
                    "200 OK".into(),
                    String::new(),
                    body.to_string(),
                    Duration::ZERO,
                )
            })
            .collect(),
    );
    let directory = std::env::temp_dir().join(format!(
        "rom-native-tools-{}-{}-{}",
        std::process::id(),
        if redb { "redb" } else { "sqlite" },
        mode
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    let mut registry = ToolRegistry::new(1).unwrap();
    registry
        .read::<String, String, _>(
            "read_note",
            "Read a current authorized note",
            move |context, id| {
                Box::pin(async move {
                    // The escape case exercises a typed query against the same authorized source.
                    let target = if escaped { "note" } else { id.as_str() };
                    Ok(context
                        .read::<Note>(target)
                        .await
                        .map_err(|_| AiError::Denied)?
                        .value
                        .ok_or(AiError::Denied)?
                        .text)
                })
            },
        )
        .unwrap();
    let request = CompletionRequest::new(vec![Message::user("Read the note and answer")], 64)
        .unwrap()
        .with_tools(registry.descriptors())
        .unwrap();
    let (runtime, client) = FlowHost::new(
        service(),
        Arc::new(server.provider()),
        Arc::new(Authority),
        Arc::new(Validator),
        Arc::new(Clock),
    )
    .unwrap()
    .tools(registry)
    .unwrap()
    .install(
        Runtime::builder().resource(
            Note::definition()
                .policy(|actor, _, _| actor.subject == "owner")
                .field_policy(|actor, _, _, _| actor.subject == "owner"),
        ),
    )
    .unwrap()
    .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
    .unwrap();
    runtime
        .execute(
            &owner(),
            Command::create(
                "note",
                Note {
                    text: "authorized private note".repeat(if ineligible { 200 } else { 1 }),
                },
            )
            .idempotency("note-create"),
        )
        .await
        .unwrap();
    let policy = RoutingPolicy::new(1, vec![model.into()], vec![], None, RunLimits::default())
        .unwrap()
        .with_allowed_providers(vec!["trusted-provider".into()])
        .unwrap();
    let run = client
        .submit(
            &owner(),
            Submission {
                id: "native-tools".into(),
                idempotency: "native-tools-submit".into(),
                request: request.clone(),
                policy,
            },
        )
        .await
        .unwrap();
    let mut first_cursor = None;
    for _ in 0..16 {
        runtime.process_work(2).await.unwrap();
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        if first_cursor.is_none() {
            first_cursor = record.cursor().cloned();
        }
        if record.state() == &RunState::Completed || record.state() == &RunState::Failed {
            break;
        }
    }
    let view = client.view(&owner(), &run).await.unwrap();
    if escaped {
        assert_eq!(
            view.state(),
            &RunState::Failed,
            "a definite bounded encoder refusal must be actionable before accepting another generation"
        );
        assert_eq!(view.failure(), Some(&AiError::InvalidRequest));
        assert!(view.output().is_none());
        let record = runtime
            .read::<AiRun>(&service(), &run.0)
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(record.counters().generation_attempts(), 1);
        assert_eq!(record.counters().tool_calls(), 1);
        assert_eq!(record.request(), &request);
        let original = record.active_attempt().unwrap();
        let account = runtime
            .read::<rom_ai::flow::AiBudget>(&service(), original.key().account_window())
            .await
            .unwrap()
            .value
            .unwrap()
            .record()
            .unwrap();
        assert_eq!(
            account.entries().len(),
            1,
            "preflight rejection must not create a successor reservation"
        );
        assert_eq!(
            server
                .requests()
                .iter()
                .filter(|bytes| bytes.starts_with(b"POST "))
                .count(),
            1
        );
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(storage);
        std::fs::remove_dir_all(directory).unwrap();
        return;
    }
    if ineligible {
        assert_eq!(view.state(), &RunState::Failed);
        assert!(view.failure().is_some());
        assert!(view.output().is_none());
        assert_eq!(
            server
                .requests()
                .iter()
                .filter(|bytes| bytes.starts_with(b"POST "))
                .count(),
            1
        );
        assert_eq!(
            runtime
                .read::<Note>(&owner(), "note")
                .await
                .unwrap()
                .revision,
            1
        );
        runtime.shutdown().await.unwrap();
        drop(client);
        drop(runtime);
        drop(storage);
        std::fs::remove_dir_all(directory).unwrap();
        return;
    }
    assert_eq!(
        view.state(),
        &RunState::Completed,
        "native continuation must accept fresh metadata while preserving frozen routing identity"
    );
    assert_eq!(view.output(), Some(&serde_json::json!({"answer":"safe"})));
    let record = runtime
        .read::<AiRun>(&service(), &run.0)
        .await
        .unwrap()
        .value
        .unwrap()
        .record()
        .unwrap();
    assert_eq!(record.request(), &request);
    assert_eq!(record.cursor(), first_cursor.as_ref());
    assert_eq!(record.counters().tool_calls(), 1);
    let requests = server.requests();
    let posts: Vec<_> = requests
        .iter()
        .filter(|bytes| bytes.starts_with(b"POST "))
        .collect();
    assert_eq!(posts.len(), 2);
    let position = posts[1]
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&posts[1][position + 4..]).unwrap();
    assert_eq!(wire["messages"][1]["tool_calls"][0]["id"], "note-1");
    assert_eq!(wire["messages"][2]["tool_call_id"], "note-1");
    assert_eq!(
        wire["messages"][2]["content"],
        "\"authorized private note\""
    );
    assert_eq!(wire["tools"][0]["function"]["name"], "read_note");
    runtime.shutdown().await.unwrap();
    drop(client);
    drop(runtime);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}
