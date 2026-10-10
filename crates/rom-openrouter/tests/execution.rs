//! Real HTTP observations must finish before the caller's shared execution end.
#![cfg(feature = "test-support")]
mod support;
use support::Credentials;
#[path = "support/stalled_body.rs"]
mod stalled;
use rom_ai::{
    AiError, AttemptEvidence, CatalogModel, CatalogSnapshot, CompletionRequest, Deadline,
    DispatchOutcome, ExecutionDeadline, Message, ModelPrice, PreparedAttempt, Provider,
    RouteCursor, RoutingPolicy, RunLimits, choose,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn prepared() -> PreparedAttempt {
    let request =
        CompletionRequest::new(vec![Message::user("private execution fixture")], 128).unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec!["fixture/model".into()],
        Vec::new(),
        None,
        RunLimits::default(),
    )
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture",
        vec![CatalogModel::text(
            "fixture/model",
            8192,
            true,
            true,
            ModelPrice::free(),
        )],
    )
    .unwrap();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "fixture").unwrap(),
        &request,
    )
    .unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    PreparedAttempt::new(
        "execution:1:1",
        request,
        policy,
        route,
        Deadline::remaining(now, now, now + 2000).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn expired_shared_execution_budget_performs_no_post_or_lookup() {
    let server = stalled::StalledBody::new();
    let provider = server.provider();
    let prepared = prepared();
    let evidence =
        AttemptEvidence::new(prepared.identity(), None, Some("gen-supervisor".into())).unwrap();
    let execution = ExecutionDeadline::from_remaining(Duration::from_millis(1)).unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(matches!(
        provider
            .complete_observed_within(&prepared, execution.clone())
            .await,
        Err(AiError::DeadlineExceeded)
    ));
    assert!(matches!(
        provider
            .reconcile_observed_within(&prepared, &evidence, execution)
            .await,
        Err(AiError::DeadlineExceeded)
    ));
    assert_eq!(server.posts(), 0);
    assert_eq!(server.requests(), 0);
}

#[tokio::test]
async fn real_adapter_returns_known_headers_before_shared_execution_end() {
    let server = stalled::StalledBody::new();
    let provider = server.provider();
    let prepared = prepared();
    let frozen = prepared.clone();
    let execution = ExecutionDeadline::from_remaining(Duration::from_millis(600)).unwrap();
    let started = Instant::now();
    let outcome = provider
        .complete_observed_within(&prepared, execution)
        .await
        .unwrap();
    let elapsed = started.elapsed();
    let DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = outcome
    else {
        panic!("accepted incomplete body cannot imply nonacceptance or completion");
    };
    assert_eq!(evidence.generation_id(), Some("gen-supervisor"));
    assert_eq!(evidence.attempt_id(), frozen.identity());
    assert_eq!(usage.cost, None);
    assert_eq!(prepared, frozen);
    assert_eq!(server.posts(), 1);
    assert!(server.headers_received());
    assert!(
        elapsed < Duration::from_millis(550),
        "adapter did not reserve observation margin: {elapsed:?}"
    );
}

#[tokio::test]
async fn metadata_lookup_consumes_the_existing_budget_instead_of_starting_two_new_seconds() {
    let server = support::Server::once(
        "200 OK",
        "",
        r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":0}}"#,
        Duration::from_millis(1000),
    );
    let provider = server.provider();
    let prepared = prepared();
    let evidence =
        AttemptEvidence::new(prepared.identity(), None, Some("gen-original".into())).unwrap();
    let context = ExecutionDeadline::from_remaining(Duration::from_millis(600)).unwrap();
    // Time spent before provider entry represents authorization and queued permit acquisition.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    assert!(matches!(
        provider
            .reconcile_observed_within(&prepared, &evidence, context)
            .await,
        Err(AiError::DeadlineExceeded)
    ));
    assert!(started.elapsed() < Duration::from_millis(450));
    let request = String::from_utf8(server.captured()).unwrap();
    assert!(request.starts_with("GET /api/v1/generation?id=gen-original "));
}
