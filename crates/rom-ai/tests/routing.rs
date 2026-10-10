use rom_ai::{
    AiError, CatalogModel, CatalogSnapshot, CompletionRequest, Deadline, Message, ModelPrice,
    OutputSchema, RouteCursor, RoutingPolicy, RunLimits, ToolDescriptor, UsdNanos, choose,
};
use serde_json::json;

fn request() -> CompletionRequest {
    CompletionRequest::new(vec![Message::user("public task")], 256).unwrap()
}
fn catalog() -> CatalogSnapshot {
    CatalogSnapshot::new(
        "fixture-v1",
        vec![
            CatalogModel::text("free-a", 4096, true, true, ModelPrice::free()),
            CatalogModel::text("free-b", 4096, true, false, ModelPrice::free()),
            CatalogModel::text(
                "paid",
                4096,
                true,
                true,
                ModelPrice::new(UsdNanos(100_000_000), UsdNanos(400_000_000), UsdNanos(0)),
            ),
        ],
    )
    .unwrap()
}
fn policy() -> RoutingPolicy {
    RoutingPolicy::new(
        1,
        vec!["free-a".into(), "free-b".into()],
        vec![],
        None,
        RunLimits::default(),
    )
    .unwrap()
}

#[test]
fn independent_runs_keep_candidate_and_paid_state() {
    let policy = policy();
    let first = RouteCursor::new(1, "fixture-v1")
        .unwrap()
        .reject("free-a")
        .unwrap();
    let second = RouteCursor::new(1, "fixture-v1").unwrap();
    assert_eq!(
        choose(&policy, &catalog(), &first, &request())
            .unwrap()
            .model(),
        "free-b"
    );
    assert_eq!(
        choose(&policy, &catalog(), &second, &request())
            .unwrap()
            .model(),
        "free-a"
    );
    let restored: RouteCursor =
        serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
    assert_eq!(
        choose(&policy, &catalog(), &restored, &request())
            .unwrap()
            .model(),
        "free-b"
    );
    let exhausted = first.reject("free-b").unwrap();
    assert_eq!(
        choose(&policy, &catalog(), &exhausted, &request()),
        Err(AiError::ProviderUnavailable)
    );
}

#[test]
fn capability_prices_and_deadline_fail_closed() {
    let tools = request()
        .with_tools(vec![
            ToolDescriptor::new(
                "read_task",
                "Read authorized task",
                json!({"type":"object"}),
            )
            .unwrap(),
        ])
        .unwrap();
    let cursor = RouteCursor::new(1, "fixture-v1")
        .unwrap()
        .reject("free-a")
        .unwrap();
    assert_eq!(
        choose(&policy(), &catalog(), &cursor, &tools),
        Err(AiError::UnsupportedCapability)
    );
    assert_eq!(UsdNanos::from_usd("-1"), Err(AiError::InvalidRequest));
    assert_eq!(UsdNanos::from_usd("NaN"), Err(AiError::InvalidRequest));
    assert_eq!(
        UsdNanos::from_usd("18446744074"),
        Err(AiError::InvalidRequest)
    );
    assert_eq!(
        Deadline::remaining(100, 90, 90),
        Err(AiError::DeadlineExceeded)
    );
    assert_eq!(
        Deadline::remaining(90, 100, 110),
        Err(AiError::DeadlineExceeded)
    );
    assert_eq!(
        Deadline::remaining(100, 100, 150).unwrap().remaining_ms(),
        50
    );
}

#[test]
fn request_limits_and_schema_requirements_are_hard() {
    assert!(CompletionRequest::new(vec![Message::user("x".repeat(32 * 1024 + 1))], 256).is_err());
    assert!(CompletionRequest::new(vec![Message::user("task")], 0).is_err());
    assert!(CompletionRequest::new(vec![Message::user("task")], u32::MAX).is_err());
    let with_schema = request().with_schema(OutputSchema::new(1, "reply", json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false})).unwrap()).unwrap();
    let no_schema = CatalogSnapshot::new(
        "fixture-v1",
        vec![CatalogModel::text(
            "free-a",
            4096,
            false,
            true,
            ModelPrice::free(),
        )],
    )
    .unwrap();
    assert_eq!(
        choose(
            &policy(),
            &no_schema,
            &RouteCursor::new(1, "fixture-v1").unwrap(),
            &with_schema
        ),
        Err(AiError::UnsupportedCapability)
    );
    let too_small = CatalogSnapshot::new(
        "fixture-v1",
        vec![CatalogModel::text(
            "free-a",
            1,
            true,
            true,
            ModelPrice::free(),
        )],
    )
    .unwrap();
    assert_eq!(
        choose(
            &policy(),
            &too_small,
            &RouteCursor::new(1, "fixture-v1").unwrap(),
            &request()
        ),
        Err(AiError::UnsupportedCapability)
    );
}

#[test]
fn debug_and_public_errors_do_not_disclose_private_payloads() {
    let private = "PRIVATE user notes sk-secret-token";
    let request = CompletionRequest::new(vec![Message::user(private)], 256).unwrap();
    assert!(!format!("{request:?}").contains(private));
    let error: AiError =
        serde_json::from_value::<AiError>(json!({"ProviderUnavailable":{"raw":private}}))
            .unwrap_err()
            .into();
    assert!(!format!("{error:?}").contains(private));
}

#[test]
fn exact_money_rounds_up_and_never_panics_on_overflow() {
    assert_eq!(UsdNanos::from_usd("0.0000000001").unwrap(), UsdNanos(1));
    assert_eq!(
        UsdNanos::from_usd("1.000000001").unwrap(),
        UsdNanos(1_000_000_001)
    );
    assert_eq!(
        ModelPrice::new(UsdNanos(1), UsdNanos(0), UsdNanos(0))
            .maximum_cost(1, 0)
            .unwrap(),
        UsdNanos(1)
    );
    let maximum = ModelPrice::new(UsdNanos(u64::MAX), UsdNanos(u64::MAX), UsdNanos(u64::MAX));
    assert_eq!(
        maximum.maximum_cost(u64::MAX, u64::MAX),
        Err(AiError::InvalidRequest)
    );
}

#[test]
fn completion_tool_ids_limits_and_debug_are_validated() {
    use rom_ai::{AttemptEvidence, Completion, ToolCall, Usage};
    let evidence = AttemptEvidence::new("run-1-attempt-1", None, None).unwrap();
    let private = "PRIVATE tool payload sk-secret";
    let call = ToolCall::new("call-1", "read_task", json!({"notes":private})).unwrap();
    let completion =
        Completion::tool_calls(vec![call.clone()], Usage::default(), evidence.clone()).unwrap();
    assert!(!format!("{completion:?}").contains(private));
    assert_eq!(
        Completion::tool_calls(vec![call.clone(), call], Usage::default(), evidence.clone()),
        Err(AiError::InvalidOutput)
    );
    assert!(Completion::output(json!("x".repeat(64 * 1024)), Usage::default(), evidence).is_err());
}

#[test]
fn telemetry_failure_cannot_escape_or_render_payloads() {
    use rom_ai::{FlowObservation, FlowObserver, ObservationKind, observe_safely};
    struct Failing;
    impl FlowObserver for Failing {
        fn observe(&self, _: &FlowObservation) {
            panic!("observer failed");
        }
    }
    let observation =
        FlowObservation::new("opaque-run-id", 1, 1, 42, ObservationKind::Completed).unwrap();
    observe_safely(Some(&Failing), &observation);
    observe_safely(None, &observation);
    assert!(
        FlowObservation::new(
            "PRIVATE notes with spaces",
            1,
            1,
            42,
            ObservationKind::Completed
        )
        .is_err()
    );
}

#[test]
fn cursor_version_and_injected_serialized_limits_are_rejected() {
    let wrong_policy = RouteCursor::new(2, "fixture-v1").unwrap();
    assert_eq!(
        choose(&policy(), &catalog(), &wrong_policy, &request()),
        Err(AiError::Conflict)
    );
    let mut malformed = serde_json::to_value(request()).unwrap();
    malformed["max_output_tokens"] = json!(u32::MAX);
    let restored: CompletionRequest = serde_json::from_value(malformed).unwrap();
    assert_eq!(
        choose(
            &policy(),
            &catalog(),
            &RouteCursor::new(1, "fixture-v1").unwrap(),
            &restored
        ),
        Err(AiError::InvalidRequest)
    );
    let mut cursor = serde_json::to_value(RouteCursor::new(1, "fixture-v1").unwrap()).unwrap();
    cursor["next_candidate"] = json!(257);
    let restored: RouteCursor = serde_json::from_value(cursor).unwrap();
    assert_eq!(
        choose(&policy(), &catalog(), &restored, &request()),
        Err(AiError::InvalidRequest)
    );
}

#[test]
fn paid_routing_requires_explicit_budget_authority() {
    let caps = ModelPrice::new(UsdNanos(100_000_000), UsdNanos(400_000_000), UsdNanos(0));
    let paid_policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["paid".into()],
        Some(caps),
        RunLimits::default(),
    )
    .unwrap();
    let cursor = RouteCursor::new(1, "fixture-v1").unwrap();
    assert_eq!(
        choose(&paid_policy, &catalog(), &cursor, &request()),
        Err(AiError::Denied)
    );
    let authorized = paid_policy.with_budget_reference("account-day-1").unwrap();
    assert_eq!(
        choose(&authorized, &catalog(), &cursor, &request())
            .unwrap()
            .model(),
        "paid"
    );
    assert_eq!(
        choose(&policy(), &catalog(), &cursor, &request())
            .unwrap()
            .maximum_cost(),
        UsdNanos(0)
    );
}

#[test]
fn nested_json_envelopes_are_finite_before_serialization() {
    let mut schema = json!({"type":"string"});
    for _ in 0..40 {
        schema = json!({"type":"object","properties":{"next":schema}});
    }
    assert_eq!(
        OutputSchema::new(1, "deep", schema),
        Err(AiError::InvalidRequest)
    );
}

#[test]
fn provider_boundary_is_object_safe_send_and_preserves_unresolved_evidence() {
    use rom_ai::{
        AiFuture, AttemptEvidence, Completion, PreparedAttempt, Provider, Reconciliation,
    };
    use std::task::{Context, Poll, Waker};
    struct Offline;
    impl Provider for Offline {
        fn catalog<'a>(&'a self, _: Deadline) -> AiFuture<'a, CatalogSnapshot> {
            Box::pin(async { Ok(catalog()) })
        }
        fn complete<'a>(&'a self, _: &'a PreparedAttempt) -> AiFuture<'a, Completion> {
            Box::pin(async { Err(AiError::UnknownOutcome) })
        }
        fn reconcile<'a>(&'a self, _: &'a AttemptEvidence) -> AiFuture<'a, Reconciliation> {
            Box::pin(async { Ok(Reconciliation::Unresolved) })
        }
    }
    fn require_send<T: Send>(_: &T) {}
    let provider: Box<dyn Provider> = Box::new(Offline);
    let evidence = AttemptEvidence::new("run-1-attempt-1", None, None).unwrap();
    let mut pending = provider.reconcile(&evidence);
    require_send(&pending);
    let mut context = Context::from_waker(Waker::noop());
    assert_eq!(
        pending.as_mut().poll(&mut context),
        Poll::Ready(Ok(Reconciliation::Unresolved))
    );
}

#[test]
fn paid_reservation_uses_request_ceiling_not_stale_catalog_quote() {
    let caps = ModelPrice::new(UsdNanos(100_000_000), UsdNanos(400_000_000), UsdNanos(0));
    let policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["paid".into()],
        Some(caps),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("account-day-1")
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "quote",
        vec![CatalogModel::text(
            "paid",
            4096,
            true,
            true,
            ModelPrice::new(UsdNanos(1), UsdNanos(1), UsdNanos(0)),
        )],
    )
    .unwrap();
    let request = request();
    let selected = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "quote").unwrap(),
        &request,
    )
    .unwrap();
    assert_eq!(
        selected.maximum_cost(),
        caps.maximum_cost(
            request.input_bound().unwrap(),
            u64::from(request.max_output_tokens())
        )
        .unwrap()
    );
}

#[test]
fn native_tool_continuation_retains_calls_and_rejects_forged_results() {
    use rom_ai::ToolCall;
    let tool = ToolDescriptor::new(
        "read_task",
        "Read authorized task",
        json!({"type":"object"}),
    )
    .unwrap();
    let assistant = Message::assistant_calls(vec![
        ToolCall::new("call-1", "read_task", json!({"id":"task-1"})).unwrap(),
    ])
    .unwrap();
    let answer = Message::tool_result("call-1", "authorized task");
    let request = CompletionRequest::new(
        vec![Message::user("read task"), assistant.clone(), answer],
        256,
    )
    .unwrap()
    .with_tools(vec![tool])
    .unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(encoded["messages"][1]["tool_calls"][0]["id"], "call-1");
    assert_eq!(encoded["messages"][2]["tool_call_id"], "call-1");
    assert!(
        CompletionRequest::new(
            vec![assistant, Message::tool_result("forged-id", "bad")],
            256
        )
        .is_err()
    );
}

#[test]
fn prepared_contract_rejects_forged_routes_before_dispatch() {
    use rom_ai::{Deadline, PreparedAttempt, RouteDecision};
    let caps = ModelPrice::new(UsdNanos(100_000_000), UsdNanos(400_000_000), UsdNanos(9));
    let policy = RoutingPolicy::new(
        1,
        vec![],
        vec!["paid".into()],
        Some(caps),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("account-day-1")
    .unwrap();
    let catalog = CatalogSnapshot::new(
        "quote",
        vec![CatalogModel::text("paid", 8192, true, true, caps)],
    )
    .unwrap();
    let request = request();
    let route = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "quote").unwrap(),
        &request,
    )
    .unwrap();
    let deadline = Deadline::remaining(100, 100, 200).unwrap();
    for (path, value) in [
        (vec!["model"], json!("external")),
        (vec!["model"], json!("")),
        (vec!["maximum_cost"], json!(0)),
        (vec!["maximum_cost"], json!(route.maximum_cost().0 + 1)),
        (vec!["tier"], json!("Free")),
        (vec!["next_cursor", "tier"], json!("Free")),
        (vec!["next_cursor", "next_candidate"], json!(0)),
        (vec!["next_cursor", "next_candidate"], json!(2)),
        (vec!["next_cursor", "policy_version"], json!(2)),
        (vec!["next_cursor", "rejected"], json!(["paid"])),
    ] {
        let mut encoded = serde_json::to_value(&route).unwrap();
        let mut target = &mut encoded;
        for component in &path {
            target = &mut target[*component];
        }
        *target = value;
        let forged: RouteDecision = serde_json::from_value(encoded).unwrap();
        assert!(
            PreparedAttempt::new(
                "attempt-1",
                request.clone(),
                policy.clone(),
                forged,
                deadline
            )
            .is_err(),
            "accepted {path:?}"
        );
    }
    let changed =
        CompletionRequest::new(vec![Message::user("more expensive input".repeat(30))], 512)
            .unwrap();
    assert!(
        PreparedAttempt::new(
            "attempt-1",
            changed.clone(),
            policy.clone(),
            route.clone(),
            deadline
        )
        .is_err()
    );
    let recomputed = choose(
        &policy,
        &catalog,
        &RouteCursor::new(1, "quote").unwrap(),
        &changed,
    )
    .unwrap();
    assert!(
        PreparedAttempt::new("attempt-1", changed, policy.clone(), recomputed, deadline).is_ok()
    );
    let mut denied = serde_json::to_value(&policy).unwrap();
    denied["budget_reference"] = serde_json::Value::Null;
    let denied = serde_json::from_value(denied).unwrap();
    assert!(PreparedAttempt::new("attempt-1", request, denied, route, deadline).is_err());
}

#[test]
fn decoded_prepared_contract_requires_explicit_validation() {
    use rom_ai::{Deadline, PreparedAttempt};
    let policy = policy();
    let request = request();
    let route = choose(
        &policy,
        &catalog(),
        &RouteCursor::new(1, "fixture-v1").unwrap(),
        &request,
    )
    .unwrap();
    let prepared = PreparedAttempt::new(
        "attempt-1",
        request,
        policy,
        route,
        Deadline::remaining(100, 100, 200).unwrap(),
    )
    .unwrap();
    prepared.validate().unwrap();
    for (expires, remaining) in [(0, 1), (100, 101), (100, 0), (1_000_000, 300_001)] {
        let mut encoded = serde_json::to_value(&prepared).unwrap();
        encoded["deadline"]["expires_at_unix_ms"] = json!(expires);
        encoded["deadline"]["remaining_ms"] = json!(remaining);
        let decoded: PreparedAttempt = serde_json::from_value(encoded).unwrap();
        assert!(decoded.validate().is_err());
    }
    let mut encoded = serde_json::to_value(&prepared).unwrap();
    encoded["route"]["model"] = json!("external");
    let decoded: PreparedAttempt = serde_json::from_value(encoded).unwrap();
    assert!(decoded.validate().is_err());
}

#[test]
fn continuation_recomputes_exact_cap_without_resetting_frozen_routing_identity() {
    let previous = request();
    let next = CompletionRequest::new(
        vec![Message::user("new authorized tool context".repeat(32))],
        256,
    )
    .unwrap();
    let policy = RoutingPolicy::new(
        1,
        vec!["free-a".into()],
        vec!["paid".into()],
        Some(ModelPrice::new(
            UsdNanos(100_000_000),
            UsdNanos(400_000_000),
            UsdNanos(0),
        )),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("paid-window")
    .unwrap();
    let cursor = RouteCursor::new(1, "fixture-v1")
        .unwrap()
        .reject("free-a")
        .unwrap();
    let route = choose(&policy, &catalog(), &cursor, &previous).unwrap();
    let derived = route.for_request(&policy, &previous, &next).unwrap();
    assert_eq!(derived.model(), route.model());
    assert_eq!(derived.tier(), route.tier());
    assert_eq!(derived.next_cursor(), route.next_cursor());
    assert!(derived.next_cursor().is_rejected("free-a"));
    assert_eq!(derived.next_cursor().catalog_identity(), "fixture-v1");
    assert_eq!(
        derived.maximum_cost(),
        policy
            .paid_caps()
            .unwrap()
            .maximum_cost(
                next.input_bound().unwrap(),
                u64::from(next.max_output_tokens())
            )
            .unwrap()
    );
    assert!(derived.maximum_cost() > route.maximum_cost());
    let mut forged = serde_json::to_value(&route).unwrap();
    forged["maximum_cost"] = json!(0);
    let forged: rom_ai::RouteDecision = serde_json::from_value(forged).unwrap();
    assert!(forged.for_request(&policy, &previous, &next).is_err());
    let mut forged = serde_json::to_value(&route).unwrap();
    forged["model"] = json!("external");
    let forged: rom_ai::RouteDecision = serde_json::from_value(forged).unwrap();
    assert!(forged.for_request(&policy, &previous, &next).is_err());
    assert!(
        route.for_request(&policy, &next, &next).is_err(),
        "a caller cannot substitute the frozen previous request"
    );
    let changed = RoutingPolicy::new(
        2,
        vec!["free-a".into()],
        vec!["paid".into()],
        policy.paid_caps(),
        RunLimits::default(),
    )
    .unwrap()
    .with_budget_reference("paid-window")
    .unwrap();
    assert!(route.for_request(&changed, &previous, &next).is_err());
}
