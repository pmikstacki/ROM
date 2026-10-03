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
            "kind":"tasks","version":1,"fields":[{"name":"done","shape":{"type":"bool"}}],"actions":["complete"]
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
