use super::support::*;
use rom::Resource;

#[derive(Clone, Debug, PartialEq, Resource)]
#[resource(name = "measurements")]
struct Measurement {
    reading: rom::FiniteF64,
}

#[tokio::test]
async fn query_anchor_route_preserves_native_float_codec_and_moving_boundary() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Measurement::definition()
                .policy(|_, _, _| true)
                .allow_all_fields()
                .query_policy(|_, _| true),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let command: Invocation = Command::create(
        "one",
        Measurement {
            reading: rom::FiniteF64::new(1.0).unwrap(),
        },
    )
    .idempotency("measurement-one")
    .into();
    let created = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&command).unwrap(),
        )
        .await;
    assert_eq!(status(&created), 200);
    let spec = json!({"filters":[{"field":"reading","value":1}],"order":[{"field":"reading","direction":"asc"}],"limit":1});
    let request = json!({"query":spec,"view":body(&created)});
    let response = server
        .post("/query/anchor", "owner-secret", &request.to_string())
        .await;
    assert_eq!(status(&response), 200, "{response}");
    let anchor = body(&response);
    assert_eq!(anchor["kind"], "measurements");
    assert!(anchor["values"][0]["value"].is_f64());
    assert!(anchor["filters"][0]["value"].is_f64());
    let mut page = spec;
    page["after"] = anchor;
    let next = server
        .post(
            "/query",
            "owner-secret",
            &json!({"kind":"measurements","query":page}).to_string(),
        )
        .await;
    assert_eq!(status(&next), 200, "{next}");
    assert_eq!(body(&next), json!([]));
    let denied = server
        .post("/query/anchor", "invalid-secret", &request.to_string())
        .await;
    assert_eq!(status(&denied), 403);
    server.finish().await;
}
