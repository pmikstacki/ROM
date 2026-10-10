//! Host-configured discovery uses actual provider tags, never display-name guesses.
#![cfg(feature = "test-support")]
mod support;
use rom_ai::{
    CompletionRequest, Deadline, Message, Provider, RouteCursor, RoutingPolicy, RunLimits, choose,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use support::Server;

const MODELS: &str = r#"{"data":[{"id":"fixture/model","context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["structured_outputs","tools"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}"#;
const ENDPOINTS: &str = r#"{"data":{"id":"fixture/model","endpoints":[{"tag":"trusted-provider","provider_name":"Untrusted display label","model_id":"fixture/model","context_length":8192,"max_prompt_tokens":8192,"max_completion_tokens":4096,"supported_parameters":["structured_outputs","tools","max_tokens"],"pricing":{"prompt":"0","completion":"0","request":"0"}}]}}"#;
fn deadline() -> Deadline {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    Deadline::remaining(now, now, now + 1000).unwrap()
}
#[tokio::test]
async fn configured_endpoint_membership_enables_only_actual_provider_tag() {
    let server = Server::sequence(vec![
        (
            "200 OK".into(),
            String::new(),
            MODELS.into(),
            Duration::ZERO,
        ),
        (
            "200 OK".into(),
            String::new(),
            ENDPOINTS.into(),
            Duration::ZERO,
        ),
    ]);
    let provider = server
        .provider()
        .with_endpoint_models(vec!["fixture/model".into()])
        .unwrap();
    let snapshot = provider.catalog(deadline()).await.unwrap();
    assert_eq!(snapshot.models()[0].providers, vec!["trusted-provider"]);
    assert!(
        String::from_utf8_lossy(&server.captured())
            .starts_with("GET /api/v1/models/fixture/model/endpoints ")
    );
    let request = CompletionRequest::new(vec![Message::user("fixture")], 256).unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec!["fixture/model".into()],
        Vec::new(),
        None,
        RunLimits::default(),
    )
    .unwrap()
    .with_allowed_providers(vec!["trusted-provider".into()])
    .unwrap();
    assert!(
        choose(
            &policy,
            &snapshot,
            &RouteCursor::new(1, snapshot.identity()).unwrap(),
            &request
        )
        .is_ok()
    );
    let impostor = policy
        .with_allowed_providers(vec!["Untrusted-display-label".into()])
        .unwrap();
    assert!(
        choose(
            &impostor,
            &snapshot,
            &RouteCursor::new(1, snapshot.identity()).unwrap(),
            &request
        )
        .is_err()
    );
}
#[tokio::test]
async fn host_endpoint_ids_reject_duplicates_unbounded_lists_and_url_injection() {
    let server = Server::once("200 OK", "", MODELS, Duration::ZERO);
    for ids in [
        vec!["fixture/model".into(), "fixture/model".into()],
        vec!["fixture/model?credential=planted".into()],
        (0..257)
            .map(|index| format!("fixture/model-{index}"))
            .collect(),
    ] {
        assert!(server.provider().with_endpoint_models(ids).is_err());
    }
    assert!(server.captured().is_empty());
}
