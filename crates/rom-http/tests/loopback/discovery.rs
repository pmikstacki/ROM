use super::support::*;

#[tokio::test]
async fn discovery_route_uses_explicit_grants_and_strict_authenticated_input() {
    use rom::{DiscoveryTarget, Resource};
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .action(COMPLETE)
                .discovery_policy(|actor, target| {
                    actor.subject == "alice"
                        && match target {
                            DiscoveryTarget::Resource => true,
                            DiscoveryTarget::Field(name) => name == "done",
                            DiscoveryTarget::Action(name) => name == "complete",
                        }
                }),
        )
        .resource(rom_consumer::Setting::definition())
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(
        runtime,
        store,
        Limits {
            body_bytes: 128,
            ..Limits::default()
        },
    )
    .await;
    let response = server.post("/discover", "owner-secret", "{}").await;
    assert_eq!(status(&response), 200);
    assert_eq!(
        body(&response),
        json!({"version":1,"resources":[{
            "kind":"tasks","version":1,"fields":[{"name":"done","shape":{"type":"bool"}}],"actions":["complete"],"action_inputs":[{"name":"complete","version":1,"input":{"type":"unit"}}]
        }]})
    );
    assert_eq!(server.store.counts().unwrap(), [0; 4]);
    let hidden = server.post("/discover", "other-secret", "{}").await;
    assert_eq!(status(&hidden), 200);
    assert_eq!(body(&hidden), json!({"version":1,"resources":[]}));
    for token in ["invalid-secret", "expired-secret"] {
        let denied = server.post("/discover", token, "{}").await;
        assert_eq!(status(&denied), 403);
        assert_eq!(body(&denied), json!({"error":"denied"}));
    }
    for input in ["[]", "null", r#"{"extra":true}"#, r#"{"x":1,"x":2}"#] {
        let invalid = server.post("/discover", "owner-secret", input).await;
        assert_eq!(status(&invalid), 400, "{input}");
    }
    assert_eq!(
        status(
            &server
                .post("/discover", "owner-secret", &" ".repeat(129))
                .await
        ),
        413
    );
    server.runtime.revoke(&Actor::trusted("local", "alice"));
    assert_eq!(
        status(&server.post("/discover", "owner-secret", "{}").await),
        403
    );
    server.finish().await;
}

#[tokio::test]
async fn discovery_http_never_returns_an_oversized_partial_catalog() {
    use rom::Resource;
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .limits(rom::Limits {
            snapshot_bytes: 1,
            ..rom::Limits::default()
        })
        .resource(Task::definition().discovery_policy(|_, _| true))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let response = server.post("/discover", "owner-secret", "{}").await;
    assert_eq!(status(&response), 413);
    assert_eq!(body(&response), json!({"error":"too_large"}));
    server.finish().await;
}

#[tokio::test]
async fn studio_wire_fixtures_use_actual_http_values_and_live_frames() {
    use rom::{Action, Input, Resource};
    #[derive(Clone, Input)]
    struct Details {
        #[input(rename = "count-value")]
        count: u64,
        note: Option<String>,
    }
    const DETAILS: Action<Task, Details> = Action::new("details", |_, _| Ok(vec![]));
    const SCALAR: Action<Task, bool> = Action::new("toggle", |task, done| {
        task.done = done;
        Ok(vec![])
    });
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .policy(rom_consumer::task_policy)
                .allow_all_fields()
                .action(DETAILS)
                .action(SCALAR)
                .discovery_policy(|_, _| true),
        )
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let discovered = body(&server.post("/discover", "owner-secret", "{}").await);
    assert_eq!(
        discovered["resources"][0]["action_inputs"],
        json!([
            {"name":"details","version":1,"input":{"type":"object","value":[
                {"name":"count-value","shape":{"type":"u64"}},
                {"name":"note","shape":{"type":"nullable","value":{"type":"string"}}}
            ]}},
            {"name":"toggle","version":1,"input":{"type":"scalar","value":{"shape":{"type":"bool"}}}}
        ])
    );
    println!("STUDIO_DISCOVERY_FIXTURE={discovered}");
    let invocation = create();
    let serialized = serde_json::to_string(&invocation).unwrap();
    let response = server.post("/invoke", "owner-secret", &serialized).await;
    assert_eq!(status(&response), 200);
    let view = body(&response);
    assert_eq!(
        view,
        json!({"key":{"kind":"tasks","id":"one"},"revision":1,"value":{"owner":"alice","display-title":"visible","done":false,"note":null}})
    );
    println!("STUDIO_INVOCATION_FIXTURE={serialized}");
    println!("STUDIO_VIEW_FIXTURE={view}");
    let query =
        json!({"kind":"tasks","query":{"filters":[{"field":"done","value":false}],"limit":1}});
    let snapshot = body(
        &server
            .post("/query", "owner-secret", &query.to_string())
            .await,
    );
    assert_eq!(snapshot, json!([view]));
    println!("STUDIO_QUERY_FIXTURE={query}");
    println!("STUDIO_SNAPSHOT_FIXTURE={snapshot}");
    let mut socket = server.stream("/live", &query.to_string()).await;
    let frame = until(&mut socket, "\n\n").await;
    let data = frame
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap();
    assert_eq!(serde_json::from_str::<rom::Value>(data).unwrap(), snapshot);
    println!(
        "STUDIO_LIVE_FIXTURE={}",
        frame.split_once("\r\n\r\n").unwrap().1.escape_debug()
    );
    drop(socket);
    let denied = server.post("/discover", "invalid-secret", "{}").await;
    assert_eq!(status(&denied), 403);
    assert_eq!(body(&denied), json!({"error":"denied"}));
    println!("STUDIO_ERROR_FIXTURE={}", body(&denied));
    let action: Invocation = Command::action("one", SCALAR, true)
        .at_revision(1)
        .idempotency("toggle-one")
        .into();
    let named: Invocation = Command::action(
        "one",
        DETAILS,
        Details {
            count: u64::MAX,
            note: None,
        },
    )
    .at_revision(1)
    .idempotency("details-one")
    .into();
    let action_body = serde_json::to_string(&action).unwrap();
    let named_body = serde_json::to_string(&named).unwrap();
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", &named_body).await),
        200
    );
    assert_eq!(
        status(&server.post("/invoke", "owner-secret", &action_body).await),
        200
    );
    println!("STUDIO_ACTION_FIXTURE={action_body}");
    println!("STUDIO_NAMED_ACTION_FIXTURE={named_body}");
    server.finish().await;
}
