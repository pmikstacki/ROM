//! Operator routes use the same verified actor and bounded body admission as Resource routes.
use super::support::*;
use rom::operator::WorkHandle;

fn request_bodies() -> [(&'static str, serde_json::Value); 3] {
    let handle = WorkHandle::from_work_id("not-an-authorization-credential");
    [
        ("/work/list", json!({"limit": 8})),
        ("/work/read", json!({"handle": handle})),
        (
            "/work/control",
            json!({
                "handle": handle,
                "expected": {"generation": "unknown", "revision": 0},
                "key": "operator-retry",
                "retry_epoch": 0,
                "operation": "Retry"
            }),
        ),
    ]
}

#[tokio::test]
async fn default_policy_denies_work_without_disclosing_its_existence() {
    let server = Server::start(Limits::default()).await;
    for (route, request) in request_bodies() {
        let response = server
            .post(route, "owner-secret", &request.to_string())
            .await;
        assert_eq!(status(&response), 403, "{route}: {response}");
        assert_eq!(body(&response), json!({"error": "denied"}));
    }
    assert_eq!(server.store.counts().unwrap(), [0, 0, 0, 0]);
    server.finish().await;
}

#[tokio::test]
async fn operator_capabilities_use_current_verified_actor_and_default_to_false() {
    let server = Server::start(Limits::default()).await;
    let response = server
        .post("/work/capabilities", "owner-secret", "{}")
        .await;
    assert_eq!(status(&response), 200, "{response}");
    assert_eq!(
        body(&response),
        json!({"protocol_version": 1, "inspect": false, "retry": false, "reconcile": false})
    );
    let denied = server
        .post("/work/capabilities", "forged-admin", "{}")
        .await;
    assert_eq!(status(&denied), 403);
    server.finish().await;
}

#[tokio::test]
async fn operator_routes_refuse_actor_fields_and_oversized_bodies() {
    let server = Server::start(Limits {
        body_bytes: 512,
        ..Limits::default()
    })
    .await;
    for (route, mut request) in request_bodies()
        .into_iter()
        .chain([("/work/capabilities", json!({}))])
    {
        request["actor"] = json!({"subject": "admin"});
        let forged = server
            .post(route, "owner-secret", &request.to_string())
            .await;
        assert_eq!(status(&forged), 400, "{route}: {forged}");
        let oversized = server.post(route, "owner-secret", &" ".repeat(513)).await;
        assert_eq!(status(&oversized), 413, "{route}: {oversized}");
        let unauthenticated = server.post(route, "forged-admin", "not-json").await;
        assert_eq!(status(&unauthenticated), 403, "{route}: {unauthenticated}");
    }
    server.finish().await;
}

#[tokio::test]
async fn operator_capabilities_accept_only_an_empty_object() {
    let server = Server::start(Limits::default()).await;
    for request in [
        "[]",
        "null",
        "true",
        r#"{"extra":1}"#,
        r#"{"extra":1,"extra":2}"#,
    ] {
        let response = server
            .post("/work/capabilities", "owner-secret", request)
            .await;
        assert_eq!(status(&response), 400, "{request}: {response}");
    }
    server.finish().await;
}

#[tokio::test]
async fn operator_routes_reject_top_level_positional_arrays() {
    let server = Server::start(Limits::default()).await;
    let handle = WorkHandle::from_work_id("not-an-authorization-credential");
    let cases = [
        ("/work/list", json!([null, null, null, 8, null])),
        ("/work/read", json!([handle])),
        (
            "/work/control",
            json!([handle, {"generation":"unknown","revision":0}, "operator-retry", 0, "Retry"]),
        ),
    ];
    let mut statuses = Vec::new();
    for (route, request) in cases {
        let response = server
            .post(route, "owner-secret", &request.to_string())
            .await;
        statuses.push(status(&response));
    }
    assert_eq!(statuses, [400, 400, 400]);
    assert_eq!(server.store.counts().unwrap(), [0, 0, 0, 0]);
    server.finish().await;
}

#[derive(Clone, rom::Resource)]
#[resource(name = "array-values")]
struct ArrayValue {
    items: Vec<String>,
}

#[tokio::test]
async fn structured_request_admission_preserves_nested_resource_arrays() {
    use rom::Resource;
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            ArrayValue::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let request = json!({"kind":"array-values","id":"one","expected":null,"idempotency":"create-array","operation":{"type":"create","input":{"items":["a","b"]}}});
    let response = server
        .post("/invoke", "owner-secret", &request.to_string())
        .await;
    assert_eq!(status(&response), 200, "{response}");
    assert_eq!(body(&response)["value"]["items"], json!(["a", "b"]));
    server.finish().await;
}
