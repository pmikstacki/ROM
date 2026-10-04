use crate::{HostConfig, StudioHost};
use rom::{Actor, Runtime};
use std::sync::{Arc, atomic::Ordering};
#[path = "../tests/support/blob.rs"]
mod blob;
#[tokio::test]
async fn authentication_failure_does_not_skip_accepted_blob_drain() {
    let runtime = Runtime::builder()
        .resource(rom_blob::definition())
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let store = Arc::new(blob::Memory::default());
    let blobs = rom_blob::BlobService::builder(runtime.clone())
        .store("attachments", store.clone())
        .build()
        .unwrap();
    let actor = Actor::trusted("fixture", "alice");
    blobs
        .reserve(
            &actor,
            "one",
            "attachments",
            rom_blob::Digest::of(b"one"),
            3,
            "reserve",
        )
        .await
        .unwrap();
    let host = StudioHost::new(
        runtime.clone(),
        HostConfig::new(
            "https://studio.example",
            "/rom-studio/",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
            actor.clone(),
        )
        .blobs(blobs.clone()),
    )
    .unwrap();
    let failed = host
        .shared
        .auth
        .run::<(), _>(async { panic!("injected trusted auth panic") })
        .await;
    assert!(matches!(failed, Err(rom::Error::Panicked)));
    store.pause_create.store(true, Ordering::SeqCst);
    let input = Box::pin(futures_util::stream::iter([Ok(b"one".to_vec())]));
    let upload = tokio::spawn(async move { blobs.upload(&actor, "one", input).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), store.started.notified())
        .await
        .unwrap();
    let mut shutdown = tokio::spawn(async move { host.shutdown().await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut shutdown)
            .await
            .is_err()
    );
    store.release.notify_one();
    assert!(matches!(shutdown.await.unwrap(), Err(rom::Error::Panicked)));
    upload.await.unwrap().unwrap();
    assert_eq!(runtime.status().unwrap().intake, rom::IntakeState::Stopped);
}
