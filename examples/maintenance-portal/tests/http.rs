//! Real loopback transport conformance; fixed fixture credentials are not production authentication.
mod support;

use rom::{Storage, json};
use std::sync::{Arc, atomic::Ordering};
use support::Server;

async fn journey(open: impl Fn() -> Arc<dyn Storage>) {
    let store = open();
    let server = Server::start(store.clone()).await;
    let initial_layout = rom_maintenance_portal::WorkspaceLayout::initial().unwrap();
    let layout = initial_layout.as_str();
    let resources = [
        (
            "equipment",
            "pump-1",
            json!({"owner":"alice","title":"Feed pump","active":true}),
        ),
        (
            "inspections",
            "check-1",
            json!({"owner":"alice","equipment":"pump-1","notes":"Valve inspected","inspected_at":"2026-10-08T12:00:00Z"}),
        ),
        (
            "work-orders",
            "repair-1",
            json!({"owner":"alice","equipment":"pump-1","title":"Replace seal","completed":false}),
        ),
        (
            "portal-settings",
            "workspace",
            json!({"owner":"alice","show_history":true,"columns":4,"layout":layout}),
        ),
    ];
    for (kind, id, input) in resources {
        let request = json!({"kind":kind,"id":id,"expected":null,"idempotency":format!("create-{kind}"),"operation":{"type":"create","input":input}});
        let created = server.post("/invoke", "fixture-alice", &request).await;
        assert_eq!(created.status, 200, "{kind}: {:?}", created.value);
        assert_eq!(created.value["key"]["kind"], kind);
        assert_eq!(created.value["key"]["id"], id);
        assert_eq!(created.value["revision"], 1);
        let read = json!({"kind":kind,"id":id});
        assert_eq!(server.post("/read", "fixture-bob", &read).await.status, 403);
        assert_eq!(server.post("/read", "unverified", &read).await.status, 403);
        assert_eq!(server.post_public("/read", &read).await.status, 403);
    }
    let discovery = server.post("/discover", "fixture-alice", &json!({})).await;
    assert_eq!(discovery.status, 200);
    let rendered = discovery.value.to_string();
    for kind in ["equipment", "inspections", "work-orders", "portal-settings"] {
        assert!(rendered.contains(kind));
    }
    let invalid_date = json!({"kind":"inspections","id":"invalid-date","expected":null,"idempotency":"invalid-date","operation":{"type":"create","input":{"owner":"alice","equipment":"pump-1","notes":"invalid","inspected_at":"yesterday"}}});
    assert_eq!(
        server
            .post("/invoke", "fixture-alice", &invalid_date)
            .await
            .status,
        400
    );
    assert_eq!(
        store
            .journal("inspections", None, 100, 1_048_576)
            .unwrap()
            .events
            .len(),
        1
    );
    let preferences = json!({"kind":"portal-settings","id":"workspace","expected":1,"idempotency":"layout-settings","operation":{"type":"replace","input":{"owner":"alice","show_history":false,"columns":3,"layout":layout}}});
    assert_eq!(
        server
            .post("/invoke", "fixture-alice", &preferences)
            .await
            .status,
        200
    );
    let transferred = json!({"kind":"portal-settings","id":"workspace","expected":2,"idempotency":"transfer-settings","operation":{"type":"replace","input":{"owner":"bob","show_history":false,"columns":3,"layout":layout}}});
    assert_eq!(
        server
            .post("/invoke", "fixture-alice", &transferred)
            .await
            .status,
        403
    );
    let complete = json!({"kind":"work-orders","id":"repair-1","expected":1,"idempotency":"complete-work","operation":{"type":"action","input":{"name":"complete","input":null}}});
    assert_eq!(
        server
            .post("/invoke", "fixture-bob", &complete)
            .await
            .status,
        403
    );
    let first = server.post("/invoke", "fixture-alice", &complete).await;
    assert_eq!(first.status, 200, "{:?}", first.value);
    assert_eq!(first.value["revision"], 2);
    let other_owner = server.post("/invoke", "fixture-bob", &complete).await;
    assert_eq!(other_owner.status, 403);
    assert!(!other_owner.value.to_string().contains("Replace seal"));
    let replay = server.post("/invoke", "fixture-alice", &complete).await;
    assert_eq!(replay.status, 200);
    assert_eq!(first.value, replay.value);
    let open_query = json!({"kind":"work-orders","field":"completed","value":false});
    let list = server.post("/query", "fixture-alice", &open_query).await;
    assert_eq!(list.status, 200);
    assert_eq!(list.value, json!([]));

    // A retained receipt cannot bypass the current host credential decision.
    server.alice_enabled.store(false, Ordering::SeqCst);
    let denied = server.post("/invoke", "fixture-alice", &complete).await;
    assert_eq!(denied.status, 403);
    assert!(!denied.value.to_string().contains("Replace seal"));
    server.alice_enabled.store(true, Ordering::SeqCst);
    assert_eq!(
        server
            .post("/invoke", "fixture-alice", &complete)
            .await
            .value,
        first.value
    );
    let weak_store = Arc::downgrade(&store);
    server.finish().await;
    drop(store);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while weak_store.upgrade().is_some() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();

    let store = open();
    let restarted = Server::start(store.clone()).await;
    let settings = restarted
        .post(
            "/read",
            "fixture-alice",
            &json!({"kind":"portal-settings","id":"workspace"}),
        )
        .await;
    assert_eq!(settings.status, 200);
    assert_eq!(settings.value["revision"], 2);
    assert_eq!(settings.value["value"]["columns"], 3);
    assert_eq!(settings.value["value"]["show_history"], false);
    let inspection = restarted
        .post(
            "/read",
            "fixture-alice",
            &json!({"kind":"inspections","id":"check-1"}),
        )
        .await;
    assert_eq!(inspection.status, 200);
    assert_eq!(inspection.value["value"]["equipment"], "pump-1");
    assert_eq!(
        inspection.value["value"]["inspected_at"],
        "2026-10-08T12:00:00.000000000Z"
    );
    let replay = restarted.post("/invoke", "fixture-alice", &complete).await;
    assert_eq!(replay.status, 200);
    assert_eq!(replay.value, first.value);
    let events = store.journal("work-orders", None, 100, 1_048_576).unwrap();
    assert_eq!(events.events.len(), 2);
    restarted.finish().await;
}

#[tokio::test]
async fn generic_http_journey_on_sqlite() {
    let directory = tempfile::tempdir().unwrap();
    journey(|| Arc::new(rom_sqlite::Sqlite::open(directory.path().join("portal.sqlite")).unwrap()))
        .await;
}

#[tokio::test]
async fn generic_http_journey_on_redb() {
    let directory = tempfile::tempdir().unwrap();
    journey(|| Arc::new(rom_redb::Redb::open(directory.path().join("portal.redb")).unwrap())).await;
}
