mod support;

use rom::{Storage, json};
use std::sync::Arc;
use support::Server;

async fn journey(store: Arc<dyn Storage>) {
    let server = Server::start(store.clone()).await;
    let create = json!({"kind":"maintenance-guides","id":"electrical","expected":null,"idempotency":"publish-guide","operation":{"type":"create","input":{"title":"Electrical inspection","nominal_voltage":230,"updated_at":"2026-10-08T12:00:00Z","internal_notes":"PRIVATE PUBLISHER NOTE"}}});
    let created = server.post("/invoke", "fixture-alice", &create).await;
    assert_eq!(created.status, 200, "{:?}", created.value);
    let layout = rom_maintenance_portal::WorkspaceLayout::initial().unwrap();
    let private_settings = json!({"kind":"portal-settings","id":"workspace","expected":null,"idempotency":"private-settings","operation":{"type":"create","input":{"owner":"alice","show_history":true,"columns":4,"layout":layout.as_str()}}});
    assert_eq!(
        server
            .post("/invoke", "fixture-alice", &private_settings)
            .await
            .status,
        200
    );
    let discovery = server.post_public("/discover", &json!({})).await;
    assert_eq!(discovery.status, 200);
    let resources = discovery.value["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["kind"], "maintenance-guides");
    assert!(!discovery.value.to_string().contains("internal_notes"));
    let read = json!({"kind":"maintenance-guides","id":"electrical"});
    let projection = server.post_public("/read", &read).await;
    assert_eq!(projection.status, 200);
    assert_eq!(projection.value["value"]["nominal_voltage"], 230);
    assert!(projection.value["value"].get("internal_notes").is_none());
    assert!(!projection.value.to_string().contains("PRIVATE"));
    let query =
        json!({"kind":"maintenance-guides","field":"title","value":"Electrical inspection"});
    let queried = server.post_public("/query", &query).await;
    assert_eq!(queried.status, 200);
    assert_eq!(queried.value, json!([projection.value]));
    let private_query = json!({"kind":"maintenance-guides","field":"internal_notes","value":"PRIVATE PUBLISHER NOTE"});
    assert_eq!(
        server.post_public("/query", &private_query).await.status,
        403
    );
    assert_eq!(server.post_public("/invoke", &create).await.status, 403);
    let replace = json!({"kind":"maintenance-guides","id":"electrical","expected":1,"idempotency":"guest-edit","operation":{"type":"replace","input":{"title":"Guest replacement","nominal_voltage":1,"updated_at":"2026-10-08T12:00:00Z","internal_notes":""}}});
    assert_eq!(server.post_public("/invoke", &replace).await.status, 403);
    assert_eq!(server.post("/read", "unverified", &read).await.status, 403);
    server
        .alice_enabled
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        server.post("/read", "fixture-alice", &read).await.status,
        403
    );
    assert_eq!(server.post_public("/read", &read).await.status, 200);
    let private_read = json!({"kind":"portal-settings","id":"workspace"});
    assert_eq!(server.post_public("/read", &private_read).await.status, 403);
    assert_eq!(
        store
            .journal("maintenance-guides", None, 10, 65536)
            .unwrap()
            .events
            .len(),
        1
    );
    server.finish().await;
}

#[tokio::test]
async fn public_catalog_sqlite_uses_safe_generic_projections() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_sqlite::Sqlite::open(directory.path().join("public.sqlite")).unwrap(),
    ))
    .await;
}

#[tokio::test]
async fn public_catalog_redb_uses_safe_generic_projections() {
    let directory = tempfile::tempdir().unwrap();
    journey(Arc::new(
        rom_redb::Redb::open(directory.path().join("public.redb")).unwrap(),
    ))
    .await;
}
