use rom::{Actor, Runtime};
use rom_studio_host::{HostConfig, StudioHost};
use std::sync::Arc;
#[path = "support/blob.rs"]
mod blob;
fn runtime() -> Runtime {
    Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap()
}
fn config() -> HostConfig {
    HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        Actor::trusted("host", "configuration"),
    )
}
#[tokio::test]
async fn blob_transport_requires_exact_shared_runtime_not_a_similar_configuration() {
    let runtime = runtime();
    let other = self::runtime();
    let service = rom_blob::BlobService::builder(runtime.clone())
        .store("attachments", Arc::new(blob::Memory::default()))
        .build()
        .unwrap();
    assert!(StudioHost::new(other.clone(), config().blobs(service.clone())).is_err());
    assert!(StudioHost::new(runtime.clone(), config().blobs(service.clone())).is_ok());
    service.shutdown().await.unwrap();
    runtime.shutdown().await.unwrap();
    other.shutdown().await.unwrap();
}
#[tokio::test]
async fn impossible_body_admission_is_rejected_without_a_semaphore_panic() {
    let runtime = runtime();
    let mut config = config();
    config.http_limits.bodies = usize::MAX;
    assert!(matches!(
        StudioHost::new(runtime.clone(), config),
        Err(rom::Error::TooLarge)
    ));
    runtime.shutdown().await.unwrap();
}
