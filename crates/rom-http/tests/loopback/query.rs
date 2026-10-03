use super::support::*;

#[tokio::test]
async fn all_wire_observation_forms_apply_field_projection_and_predicate_authority() {
    use rom::Resource;
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .policy(rom_consumer::task_policy)
                .field_policy(|_, access, name, _| {
                    matches!(access, rom::Access::Write) || name != "note"
                })
                .query_policy(|_, name| name != "note"),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let mut command = create();
    if let rom::Operation::Create(ref mut value) = command.operation {
        value["note"] = json!("SECRET-FIELD");
    }
    let response = server
        .post(
            "/invoke",
            "owner-secret",
            &serde_json::to_string(&command).unwrap(),
        )
        .await;
    assert_eq!(status(&response), 200);
    assert!(!response.contains("SECRET-FIELD"));
    assert!(body(&response)["value"].get("note").is_none());
    for (route, payload) in [
        ("/read", json!({"kind":"tasks","id":"one"})),
        (
            "/query",
            json!({"kind":"tasks","field":"done","value":false}),
        ),
        ("/journal", json!({"kind":"tasks","after":null})),
    ] {
        let response = server
            .post(route, "owner-secret", &payload.to_string())
            .await;
        assert_eq!(status(&response), 200);
        assert!(!response.contains("SECRET-FIELD"));
        assert!(!response.contains("note"));
    }
    let response = server
        .post(
            "/query",
            "owner-secret",
            &json!({"kind":"tasks","field":"note","value":"SECRET-FIELD"}).to_string(),
        )
        .await;
    assert_eq!(status(&response), 403);
    assert!(!response.contains("SECRET-FIELD"));
    let mut live = server
        .stream(
            "/live",
            &json!({"kind":"tasks","field":"done","value":false}).to_string(),
        )
        .await;
    let response = until(&mut live, "visible").await;
    assert!(!response.contains("SECRET-FIELD"));
    drop(live);
    server.finish().await;
}

#[tokio::test]
async fn structured_queries_and_patches_use_the_generic_wire_contract() {
    let server = Server::start(Limits::default()).await;
    assert_eq!(
        status(
            &server
                .post(
                    "/invoke",
                    "owner-secret",
                    &serde_json::to_string(&create()).unwrap()
                )
                .await
        ),
        200
    );
    let query =
        r#"{"kind":"tasks","query":{"filters":[{"field":"done","value":false}],"limit":1}}"#;
    let response = server.post("/query", "owner-secret", query).await;
    assert_eq!(status(&response), 200);
    assert_eq!(body(&response).as_array().unwrap().len(), 1);
    let patch = r#"{"kind":"tasks","id":"one","expected":1,"idempotency":"patch","operation":{"type":"patch","input":{"done":{"op":"set","value":true}}}}"#;
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", patch).await),
        200
    );
    assert_eq!(
        body(&server.post("/query", "owner-secret", query).await)
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let mut stream = server.stream("/live", query).await;
    assert!(until(&mut stream, "event: data").await.contains("data: []"));
    drop(stream);
    let ambiguous = r#"{"kind":"tasks","field":"done","value":true,"query":{"filters":[]}}"#;
    assert_eq!(
        status(&server.post("/query", "owner-secret", ambiguous).await),
        400
    );
    server.finish().await;
}

#[tokio::test]
async fn ranges_sort_and_client_anchor_share_query_and_live_wire_paths() {
    use rom::{Direction, Resource};
    let server = Server::start(Limits::default()).await;
    let actor = Actor::trusted("local", "alice");
    for (id, title) in [("a", "z"), ("b", "é"), ("c", "z")] {
        server
            .runtime
            .execute(
                &actor,
                Command::create(
                    id,
                    Task {
                        owner: "alice".into(),
                        title: title.into(),
                        done: false,
                        note: None,
                    },
                )
                .idempotency(id),
            )
            .await
            .unwrap();
    }
    let query = Task::title_field()
        .at_least("z".into())
        .order_by(Task::title_field(), Direction::Asc)
        .limit(1);
    let request = json!({"kind":Task::KIND,"query":query.spec()}).to_string();
    let response = server.post("/query", "owner-secret", &request).await;
    assert_eq!(status(&response), 200);
    assert_eq!(body(&response)[0]["key"]["id"], json!("a"));
    let first = server.runtime.query(&actor, &query).await.unwrap();
    let next = query.after_snapshot(&first[0]).unwrap().limit(5);
    let request = json!({"kind":Task::KIND,"query":next.spec()}).to_string();
    let response = server.post("/query", "owner-secret", &request).await;
    assert_eq!(status(&response), 200);
    assert_eq!(
        body(&response)
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["key"]["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["c", "b"]
    );
    let mut stream = server.stream("/live", &request).await;
    let data = until(&mut stream, "event: data").await;
    assert!(data.contains("\"id\":\"c\""));
    assert!(!data.contains("\"id\":\"a\""));
    drop(stream);
    let response = server.post("/query", "other-secret", &request).await;
    assert_eq!(status(&response), 200);
    assert_eq!(body(&response), json!([]));
    let mut bad = next.spec().clone();
    bad.after.as_mut().unwrap().kind = "other".into();
    assert_eq!(
        status(
            &server
                .post(
                    "/query",
                    "owner-secret",
                    &json!({"kind":Task::KIND,"query":bad}).to_string()
                )
                .await
        ),
        400
    );
    server.finish().await;
}
