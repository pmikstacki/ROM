use rom_ai::{
    AiError, CatalogModel, CatalogSnapshot, CompletionRequest, Deadline, Message, ModelPrice,
    PreparedAttempt, Provider, RouteCursor, RoutingPolicy, RunLimits, choose,
};
mod support;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use support::Server;

fn deadline(duration: u64) -> Deadline {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    Deadline::remaining(now, now, now + duration).unwrap()
}
fn attempt() -> PreparedAttempt {
    let request = CompletionRequest::new(vec![Message::user("private-fixture")], 256).unwrap();
    prepared(request)
}
fn prepared(request: CompletionRequest) -> PreparedAttempt {
    let policy = RoutingPolicy::new(
        1,
        vec!["fixture/model".into()],
        vec![],
        None,
        RunLimits::default(),
    )
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture",
        vec![CatalogModel::text(
            "fixture/model",
            4096,
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
    PreparedAttempt::new("run:1:1", request, policy, route, deadline(1000)).unwrap()
}

fn body(server: &Server) -> serde_json::Value {
    let bytes = server.captured();
    let position = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    serde_json::from_slice(&bytes[position + 4..]).unwrap()
}

#[tokio::test]
async fn catalog_parses_sub_nanodollar_token_rates_before_scaling() {
    let server = Server::once(
        "200 OK",
        "",
        r#"{"data":[{"id":"fixture/model","context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"supported_parameters":["structured_outputs","tools"],"pricing":{"prompt":"0.0000000001","completion":"0.0000000002","request":"0.000000001"}}]}"#,
        Duration::ZERO,
    );
    let catalog = server.provider().catalog(deadline(1000)).await.unwrap();
    assert_eq!(catalog.models().len(), 1);
    let model = &catalog.models()[0];
    assert_eq!(model.price.prompt_per_million, rom_ai::UsdNanos(100_000));
    assert_eq!(
        model.price.completion_per_million,
        rom_ai::UsdNanos(200_000)
    );
    assert_eq!(model.price.request, rom_ai::UsdNanos(1));
    assert!(model.supports_schema && model.supports_tools);
    assert!(model.providers.is_empty());
}

#[tokio::test]
async fn catalog_model_count_is_bounded_before_route_admission() {
    let model = serde_json::json!({"id":"fixture/model","context_length":8192,"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"pricing":{"prompt":"0","completion":"0","request":"0"}});
    let models: Vec<_> = (0..4097)
        .map(|index| {
            let mut model = model.clone();
            model["id"] = serde_json::json!(format!("fixture/model-{index}"));
            model
        })
        .collect();
    let body = serde_json::to_string(&serde_json::json!({"data":models})).unwrap();
    let server = Server::once("200 OK", "", &body, Duration::ZERO);
    assert!(matches!(
        server.provider().catalog(deadline(1000)).await,
        Err(AiError::InvalidOutput)
    ));
}

#[tokio::test]
async fn wire_schema_is_preserved_and_parameters_are_required() {
    let schema = serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"additionalProperties":false});
    let request = CompletionRequest::new(vec![Message::user("private-fixture")], 256)
        .unwrap()
        .with_schema(rom_ai::OutputSchema::new(1, "answer_schema", schema.clone()).unwrap())
        .unwrap();
    let attempt = prepared(request);
    let server = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-fixture","model":"fixture/model","choices":[{"finish_reason":"stop","message":{"content":"{\"answer\":\"safe\"}"}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0}}"#,
        Duration::ZERO,
    );
    let result = server.provider().complete(&attempt).await.unwrap();
    let rom_ai::Completion::Output {
        value, evidence, ..
    } = result
    else {
        panic!("expected output")
    };
    assert_eq!(value, serde_json::json!({"answer":"safe"}));
    assert_eq!(evidence.attempt_id(), attempt.identity());
    assert_eq!(evidence.generation_id(), Some("gen-fixture"));
    let wire = body(&server);
    assert_eq!(wire["response_format"]["json_schema"]["schema"], schema);
    assert_eq!(wire["provider"]["require_parameters"], true);
    assert_eq!(wire["provider"]["allow_fallbacks"], false);
    assert_eq!(
        wire["provider"]["max_price"],
        serde_json::json!({"prompt":0,"completion":0,"request":0})
    );
    assert!(wire.get("models").is_none() && wire.get("plugins").is_none());
    assert_eq!(wire["stream"], false);
}

#[tokio::test]
async fn native_tool_calls_retain_ids_and_continuation_wire_format() {
    let tool = rom_ai::ToolDescriptor::new("read_task","Read authorized task",serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false})).unwrap();
    let request = CompletionRequest::new(vec![Message::user("private-fixture")], 256)
        .unwrap()
        .with_tools(vec![tool.clone()])
        .unwrap();
    let server = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-tools","model":"fixture/model","choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":"read_task","arguments":"{\"id\":\"task-1\"}"}}]}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0}}"#,
        Duration::ZERO,
    );
    let rom_ai::Completion::ToolCalls { calls, .. } = server
        .provider()
        .complete(&prepared(request))
        .await
        .unwrap()
    else {
        panic!("expected native tools")
    };
    assert_eq!(calls[0].id(), "call-1");
    assert_eq!(calls[0].arguments(), &serde_json::json!({"id":"task-1"}));
    let continuation = CompletionRequest::new(
        vec![
            Message::user("private-fixture"),
            Message::assistant_calls(calls).unwrap(),
            Message::tool_result("call-1", "authorized result"),
        ],
        256,
    )
    .unwrap()
    .with_tools(vec![tool])
    .unwrap();
    let next = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-result","model":"fixture/model","choices":[{"finish_reason":"stop","message":{"content":"safe"}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0}}"#,
        Duration::ZERO,
    );
    next.provider()
        .complete(&prepared(continuation))
        .await
        .unwrap();
    let wire = body(&next);
    assert_eq!(
        wire["messages"][1]["tool_calls"][0]["function"]["arguments"],
        r#"{"id":"task-1"}"#
    );
    assert_eq!(wire["messages"][2]["tool_call_id"], "call-1");
    assert_eq!(wire["tools"][0]["function"]["name"], "read_task");
    assert_eq!(wire["parallel_tool_calls"], false);
}

#[tokio::test]
async fn accepted_error_retains_header_id_without_claiming_nonacceptance() {
    let server = Server::once(
        "200 OK",
        "X-Generation-Id: gen-original-header\r\n",
        r#"{"error":{"code":429,"message":"private"}}"#,
        Duration::ZERO,
    );
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = server
        .provider()
        .complete_observed(&attempt())
        .await
        .unwrap()
    else {
        panic!("accepted error is uncertain")
    };
    assert_eq!(evidence.generation_id(), Some("gen-original-header"));
    assert_eq!(usage.cost, None);
}

fn paid_prepared() -> PreparedAttempt {
    let request = CompletionRequest::new(vec![Message::user("private-fixture")], 256).unwrap();
    let policy = RoutingPolicy::new(
        1,
        Vec::new(),
        vec!["fixture/model".into()],
        Some(ModelPrice::new(
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(0),
            rom_ai::UsdNanos(10),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("trusted-account-reference")
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture",
        vec![CatalogModel::text(
            "fixture/model",
            4096,
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
    PreparedAttempt::new("run:1:1", request, policy, route, deadline(1000)).unwrap()
}

#[tokio::test]
async fn numeric_usage_cost_rounds_original_lexeme_up_without_float_loss() {
    let server = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-cost","model":"fixture/model","choices":[{"finish_reason":"stop","message":{"content":"safe"}}],"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0.0000000010000000000000000001,"cost_details":{"upstream_inference_cost":0}}}"#,
        Duration::ZERO,
    );
    let rom_ai::Completion::Output { usage, .. } =
        server.provider().complete(&paid_prepared()).await.unwrap()
    else {
        panic!("expected output")
    };
    assert_eq!(usage.cost, Some(rom_ai::UsdNanos(2)));
    assert_eq!(usage.input_tokens, Some(4));
}

#[tokio::test]
async fn metadata_only_generation_lookup_is_unresolved_and_uses_original_id() {
    let server = Server::once(
        "200 OK",
        "",
        r#"{"data":{"id":"gen-recorded","model":"fixture/model","total_cost":0.000000003,"tokens_prompt":4,"tokens_completion":1}}"#,
        Duration::ZERO,
    );
    let evidence =
        rom_ai::AttemptEvidence::new("run:1:1", None, Some("gen-recorded".into())).unwrap();
    assert_eq!(
        server.provider().reconcile(&evidence).await.unwrap(),
        rom_ai::Reconciliation::Unresolved
    );
    assert!(
        String::from_utf8_lossy(&server.captured())
            .starts_with("GET /api/v1/generation?id=gen-recorded ")
    );
}

#[tokio::test]
async fn unexpected_sse_remains_unknown_and_retains_generation_header() {
    let server = Server::once(
        "200 OK",
        "Content-Type: text/event-stream\r\nX-Generation-Id: gen-stream\r\n",
        "data: {\"error\":{\"code\":429}}\n\n",
        Duration::ZERO,
    );
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = server
        .provider()
        .complete_observed(&attempt())
        .await
        .unwrap()
    else {
        panic!("unexpected stream is uncertain")
    };
    assert_eq!(evidence.generation_id(), Some("gen-stream"));
    assert_eq!(usage.cost, None);
}

#[tokio::test]
async fn accepted_body_rate_limit_is_unknown_not_retryable() {
    let server = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-fixture","error":{"code":429,"message":"planted-private-provider-message","metadata":{"error_type":"rate_limit_exceeded"}}}"#,
        Duration::ZERO,
    );
    let result = server.provider().complete(&attempt()).await;
    assert!(matches!(result, Err(AiError::UnknownOutcome)));
    assert!(
        String::from_utf8_lossy(&server.captured()).starts_with("POST /api/v1/chat/completions ")
    );
}

#[tokio::test]
async fn preacceptance_http_rate_limit_retains_retry_floor() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429,"message":"private"}}"#,
        Duration::ZERO,
    );
    assert_eq!(
        server.provider().complete(&attempt()).await,
        Err(AiError::RateLimited {
            retry_after_ms: 3000
        })
    );
}

#[tokio::test]
async fn catalog_delay_consumes_the_original_deadline() {
    let server = Server::once("200 OK", "", r#"{"data":[]}"#, Duration::from_millis(150));
    let provider = server.provider();
    // Client construction precedes the caller's deadline and is not catalog execution.
    let started = std::time::Instant::now();
    assert!(matches!(
        provider.catalog(deadline(25)).await,
        Err(AiError::DeadlineExceeded)
    ));
    assert!(started.elapsed() < Duration::from_millis(120));
}

#[tokio::test]
async fn catalog_cache_reuses_identity_and_does_not_mask_expired_deadline() {
    let server = Server::once("200 OK", "", r#"{"data":[]}"#, Duration::ZERO);
    let provider = server.provider();
    let first = provider.catalog(deadline(1000)).await.unwrap();
    let second = provider.catalog(deadline(1000)).await.unwrap();
    assert_eq!(first, second);
    let expired = deadline(1);
    tokio::time::sleep(Duration::from_millis(3)).await;
    assert_eq!(
        provider.catalog(expired).await,
        Err(AiError::DeadlineExceeded)
    );
}

#[tokio::test]
async fn exact_scientific_cost_and_invalid_amounts_do_not_become_free() {
    for (raw, expected) in [
        ("1.0000000000000000001e-9", Some(2)),
        ("1e-10000", Some(1)),
        ("0e10000", Some(0)),
        ("-1", None),
        ("1e10000", None),
        ("18446744073.709551616", None),
    ] {
        let reply = format!(
            r#"{{"id":"gen-money","model":"fixture/model","choices":[{{"finish_reason":"stop","message":{{"content":"safe"}}}}],"usage":{{"prompt_tokens":4,"completion_tokens":1,"cost":{raw}}}}}"#
        );
        let server = Server::once("200 OK", "", &reply, Duration::ZERO);
        let result = server
            .provider()
            .complete_observed(&paid_prepared())
            .await
            .unwrap();
        match expected {
            Some(cost) => {
                let rom_ai::DispatchOutcome::Completed(rom_ai::Completion::Output {
                    usage, ..
                }) = result
                else {
                    panic!("valid exact amount {raw}")
                };
                assert_eq!(usage.cost, Some(rom_ai::UsdNanos(cost)));
            }
            None => {
                let rom_ai::DispatchOutcome::Uncertain { usage, .. } = result else {
                    panic!("invalid amount {raw} must hold")
                };
                assert_eq!(usage.cost, None);
            }
        }
    }
}

#[tokio::test]
async fn oversized_accepted_body_retains_original_generation_for_recovery() {
    let reply = "x".repeat(128 * 1024 + 1);
    let server = Server::once(
        "200 OK",
        "X-Generation-Id: gen-large\r\n",
        &reply,
        Duration::ZERO,
    );
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = server
        .provider()
        .complete_observed(&attempt())
        .await
        .unwrap()
    else {
        panic!("oversized accepted response is uncertain")
    };
    assert_eq!(evidence.generation_id(), Some("gen-large"));
    assert_eq!(usage.cost, None);
}

#[tokio::test]
async fn header_and_body_generation_identity_disagreement_is_held() {
    let server = Server::once(
        "200 OK",
        "X-Generation-Id: gen-original\r\n",
        r#"{"id":"gen-foreign","model":"fixture/model","choices":[{"finish_reason":"stop","message":{"content":"safe"}}],"usage":{"cost":0}}"#,
        Duration::ZERO,
    );
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = server
        .provider()
        .complete_observed(&attempt())
        .await
        .unwrap()
    else {
        panic!("contradictory identities cannot complete")
    };
    assert_eq!(evidence.generation_id(), Some("gen-original"));
    assert_eq!(usage.cost, None);
}

#[tokio::test]
async fn observed_metadata_preserves_exact_known_cost_without_claiming_output() {
    let prepared = paid_prepared();
    let evidence =
        rom_ai::AttemptEvidence::new(prepared.identity(), None, Some("gen-original".into()))
            .unwrap();
    let server = Server::once(
        "200 OK",
        "",
        r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":0.000000003,"native_tokens_prompt":4,"native_tokens_completion":1,"native_tokens_reasoning":0,"cancelled":true}}"#,
        Duration::ZERO,
    );
    let observed = server
        .provider()
        .reconcile_observed(&prepared, &evidence)
        .await
        .unwrap();
    let rom_ai::ReconciliationObservation::Uncertain {
        evidence: actual,
        usage,
    } = observed
    else {
        panic!("metadata cannot reconstruct the answer");
    };
    assert_eq!(actual, evidence);
    assert_eq!(usage.cost, Some(rom_ai::UsdNanos(3)));
    assert_eq!(usage.input_tokens, Some(4));
    assert_eq!(usage.output_tokens, Some(1));
    let request = String::from_utf8(server.captured()).unwrap();
    assert!(request.starts_with("GET /api/v1/generation?id=gen-original "));
    assert!(!request.contains("/chat/completions"));
}

#[tokio::test]
async fn observed_metadata_rejects_foreign_model_identity_and_financial_amplification() {
    for body in [
        r#"{"data":{"id":"gen-original","model":"foreign/model","total_cost":0.000000003}}"#,
        r#"{"data":{"id":"gen-foreign","model":"fixture/model","total_cost":0.000000003}}"#,
        r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":0.000000011}}"#,
        r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":-0.000000001}}"#,
        r#"{"data":{"id":"gen-original","model":"fixture/model","total_cost":0,"native_tokens_completion":257}}"#,
    ] {
        let prepared = paid_prepared();
        let evidence =
            rom_ai::AttemptEvidence::new(prepared.identity(), None, Some("gen-original".into()))
                .unwrap();
        let server = Server::once("200 OK", "", body, Duration::ZERO);
        assert_eq!(
            server
                .provider()
                .reconcile_observed(&prepared, &evidence)
                .await,
            Err(AiError::InvalidOutput)
        );
    }
}

#[tokio::test]
async fn observed_lookup_missing_generation_and_lookup_miss_remain_unresolved() {
    let prepared = paid_prepared();
    let missing = rom_ai::AttemptEvidence::new(prepared.identity(), None, None).unwrap();
    let server = Server::once("404 Not Found", "", "{}", Duration::ZERO);
    assert_eq!(
        server
            .provider()
            .reconcile_observed(&prepared, &missing)
            .await
            .unwrap(),
        rom_ai::ReconciliationObservation::Resolved(rom_ai::Reconciliation::Unresolved)
    );
    assert!(server.captured().is_empty());
    let evidence =
        rom_ai::AttemptEvidence::new(prepared.identity(), None, Some("gen-original".into()))
            .unwrap();
    assert_eq!(
        server
            .provider()
            .reconcile_observed(&prepared, &evidence)
            .await
            .unwrap(),
        rom_ai::ReconciliationObservation::Resolved(rom_ai::Reconciliation::Unresolved)
    );
}

#[tokio::test]
async fn generation_explicitly_disables_vendor_response_cache() {
    let server = Server::once(
        "200 OK",
        "",
        r#"{"id":"gen-fixture","model":"fixture/model","choices":[{"finish_reason":"stop","message":{"content":"fresh"}}],"usage":{"cost":0}}"#,
        Duration::ZERO,
    );
    server.provider().complete(&attempt()).await.unwrap();
    let request = String::from_utf8(server.captured())
        .unwrap()
        .to_ascii_lowercase();
    assert!(request.contains("\r\nx-openrouter-cache: false\r\n"));
}

#[tokio::test]
async fn nonacceptance_generation_header_is_uncertain_and_preserved() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\nX-Generation-Id: gen-refusal-header\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = outcome
    else {
        panic!("generation contradicts safe refusal")
    };
    assert_eq!(evidence.attempt_id(), prepared.identity());
    assert_eq!(evidence.generation_id(), Some("gen-refusal-header"));
    assert_eq!(usage.cost, None);
}

#[tokio::test]
async fn nonacceptance_generation_body_is_uncertain_and_preserved() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"id":"gen-refusal-body","error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain { evidence, .. } = outcome else {
        panic!("body generation contradicts safe refusal")
    };
    assert_eq!(evidence.attempt_id(), prepared.identity());
    assert_eq!(evidence.generation_id(), Some("gen-refusal-body"));
}

#[tokio::test]
async fn nonacceptance_positive_usage_is_uncertain_and_retains_valid_cost() {
    let request = CompletionRequest::new(vec![Message::user("synthetic")], 32).unwrap();
    let cap = ModelPrice::new(
        rom_ai::UsdNanos(0),
        rom_ai::UsdNanos(0),
        rom_ai::UsdNanos(10),
    );
    let policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["fixture/model".into()],
        Some(cap),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("fixture-account")
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture-paid",
        vec![CatalogModel::text("fixture/model", 4096, true, true, cap)],
    )
    .unwrap();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "fixture-paid").unwrap(),
        &request,
    )
    .unwrap();
    let prepared = PreparedAttempt::new("run:1:1", request, policy, route, deadline(1000)).unwrap();
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429},"usage":{"prompt_tokens":4,"completion_tokens":1,"cost":0.000000003}}"#,
        Duration::ZERO,
    );
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = outcome
    else {
        panic!("usage cannot become uncharged refusal")
    };
    assert_eq!(evidence.attempt_id(), prepared.identity());
    assert_eq!(usage.cost, Some(rom_ai::UsdNanos(3)));
}

#[tokio::test]
async fn nonacceptance_malformed_body_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        "{",
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
}

#[tokio::test]
async fn nonacceptance_duplicate_usage_cannot_authorize_zero_charge_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429},"usage":{"cost":0.000000003},"usage":{"cost":0}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
}

#[tokio::test]
async fn nonacceptance_duplicate_error_code_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":500,"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
}

#[tokio::test]
async fn nonacceptance_partial_body_retains_generation_header_as_unknown() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\nX-Generation-Id: gen-truncated\r\nContent-Length: 300\r\n",
        "{",
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = outcome
    else {
        panic!("partial body cannot prove refusal")
    };
    assert_eq!(evidence.generation_id(), Some("gen-truncated"));
    assert_eq!(usage.cost, None);
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_contradictory_generation_ids_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\nX-Generation-Id: gen-header\r\n",
        r#"{"id":"gen-body","error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain { evidence, .. } = outcome else {
        panic!("contradictory identities cannot prove refusal")
    };
    assert_eq!(evidence.generation_id(), Some("gen-header"));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_cost_above_ceiling_is_unknown_not_zero_charge_refusal() {
    let request = CompletionRequest::new(vec![Message::user("paid fixture")], 32).unwrap();
    let cap = ModelPrice::new(
        rom_ai::UsdNanos(0),
        rom_ai::UsdNanos(0),
        rom_ai::UsdNanos(10),
    );
    let policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["fixture/model".into()],
        Some(cap),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("fixture-account")
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "fixture-paid",
        vec![CatalogModel::text("fixture/model", 4096, true, true, cap)],
    )
    .unwrap();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "fixture-paid").unwrap(),
        &request,
    )
    .unwrap();
    let prepared = PreparedAttempt::new("run:1:1", request, policy, route, deadline(1000)).unwrap();
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429},"usage":{"cost":0.000000011}}"#,
        Duration::ZERO,
    );
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain {
        evidence, usage, ..
    } = outcome
    else {
        panic!("invalid cost is not zero-charge refusal")
    };
    assert_eq!(evidence.attempt_id(), prepared.identity());
    assert_eq!(usage.cost, None);
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_invalid_generation_header_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "X-Generation-Id: malformed\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain { evidence, .. } = outcome else {
        panic!("ambiguous header cannot prove refusal")
    };
    assert_eq!(evidence.generation_id(), None);
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_duplicate_generation_headers_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "X-Generation-Id: malformed\r\nX-Generation-Id: gen-second\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain { evidence, .. } = outcome else {
        panic!("ambiguous header cannot prove refusal")
    };
    assert_eq!(evidence.generation_id(), Some("gen-second"));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_duplicate_retry_headers_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\nRetry-After: 30\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    let outcome = server
        .provider()
        .complete_observed(&prepared)
        .await
        .unwrap();
    let rom_ai::DispatchOutcome::Uncertain { evidence, .. } = outcome else {
        panic!("ambiguous header cannot prove refusal")
    };
    assert_eq!(evidence.generation_id(), None);
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_nontext_generation_header_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "X-Generation-Id: \u{0080}\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_nontext_retry_header_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: \u{0080}\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_malformed_retry_header_cannot_authorize_retry() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: garbage\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let prepared = attempt();
    assert!(matches!(
        server
            .provider()
            .complete_observed(&prepared)
            .await
            .unwrap(),
        rom_ai::DispatchOutcome::Uncertain { .. }
    ));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn nonacceptance_post_timeout_never_becomes_confirmed_refusal() {
    let (mut server, post_received) = Server::once_held(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429}}"#,
    );
    let provider = server.provider();
    let original = attempt();
    let request = original.request().clone();
    let policy = original.policy().clone();
    let route = original.route().clone();

    // Active blocking work prevents auto-advance during real TCP setup.
    // Sender disconnection releases the keeper during panic unwinding, too.
    let (clock_release, clock_hold) = std::sync::mpsc::channel::<()>();
    let (clock_ready, clock_started) = tokio::sync::oneshot::channel();
    let keeper = tokio::task::spawn_blocking(move || {
        let _ = clock_ready.send(());
        clock_hold.recv_timeout(Duration::from_secs(2))
    });
    clock_started.await.unwrap();

    // Existing Deserialize admits separate wall expiry and remaining-time budgets.
    // This fixture tests post-dispatch classification, not end-to-end performance.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let expiry = now.checked_add(2000).unwrap();
    let bounded: Deadline = serde_json::from_value(serde_json::json!({
        "expires_at_unix_ms": expiry,
        "remaining_ms": 25,
    }))
    .unwrap();
    assert_eq!(bounded.expires_at_unix_ms(), expiry);
    assert_eq!(bounded.remaining_ms(), 25);
    let prepared =
        PreparedAttempt::new(original.identity(), request, policy, route, bounded).unwrap();
    let timer_started = tokio::time::Instant::now();
    let completion = provider.complete_observed(&prepared);
    tokio::pin!(completion);
    tokio::select! {
        biased;
        _ = &mut completion => panic!("completion ended before full POST receipt"),
        receipt = post_received => receipt.expect("full POST receipt within real setup bounds"),
    }
    assert_eq!(server.requests().len(), 1);
    assert_eq!(timer_started.elapsed(), Duration::ZERO);

    // Only a recorded full POST permits the paused clock to advance.
    drop(clock_release);
    assert!(matches!(
        keeper.await.unwrap(),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(matches!(
        completion.await,
        Err(AiError::DeadlineExceeded | AiError::UnknownOutcome)
    ));
    assert!(timer_started.elapsed() > Duration::ZERO);
    // Tokio rounds timer deadlines up to its millisecond granularity.
    assert!(timer_started.elapsed() <= Duration::from_millis(26));
    server.release_response();
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn nonacceptance_pre_dispatch_timeout_records_no_post() {
    let server = Server::once(
        "429 Too Many Requests",
        "Retry-After: 3\r\n",
        r#"{"error":{"code":429}}"#,
        Duration::ZERO,
    );
    let provider = server.provider();
    let original = attempt();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let observed = now.checked_sub(26).unwrap();
    let expired = Deadline::remaining(observed, observed, now.checked_sub(1).unwrap()).unwrap();
    assert_eq!(expired.remaining_ms(), 25);
    assert!(expired.expires_at_unix_ms() < now);
    let prepared = PreparedAttempt::new(
        original.identity(),
        original.request().clone(),
        original.policy().clone(),
        original.route().clone(),
        expired,
    )
    .unwrap();
    assert!(matches!(
        provider.complete_observed(&prepared).await,
        Err(AiError::DeadlineExceeded)
    ));
    assert!(server.requests().is_empty());
}
