//! Real projection boundary; authentication/session route is a separate pending integration scope.
use super::*;
#[tokio::test]
async fn denied_final_projection_records_cause_without_changing_unknown_response() {
    let runtime = rom::Runtime::builder()
        .resource(rom_blob::definition())
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            rom::Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let actor = rom::Actor::trusted("fixture", "alice");
    let config = crate::HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        rom::Actor::trusted("fixture", "bootstrap"),
    )
    .authentication_diagnostics(true);
    let host = crate::StudioHost::new(runtime.clone(), config).unwrap();
    runtime.revoke(&actor);
    let response = projected(&host.shared, &actor, "one", true).await;
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4096).await;
    host.shutdown().await.unwrap();
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    // Missing resource is deliberately irrelevant: revocation denies before disclosure.
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
    assert_eq!(body["error"], "outcome_unknown");
    assert_eq!(snapshot.stage(AuthStage::BlobProjectionResult).denied, 1);
    assert_eq!(snapshot.operation(AuthOperation::Blob).attempts, 0);
}
