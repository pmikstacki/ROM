use crate::authentication_evidence::finish;
use rom::{Actor, Error, Runtime};
use rom_studio_host::{HostConfig, StudioHost};
use std::{os::unix::fs::PermissionsExt, sync::Arc};

async fn fixture() -> (std::path::PathBuf, StudioHost) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rom-load-auth-evidence-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let runtime = Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let config = HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/rom-studio-host/tests/assets"),
        Actor::trusted("fixture", "bootstrap"),
    )
    .authentication_diagnostics(true);
    let host = StudioHost::new(runtime, config).unwrap();
    host.shutdown().await.unwrap();
    (directory, host)
}

#[tokio::test]
async fn drained_host_records_a_bounded_private_snapshot() {
    let (directory, host) = fixture().await;
    finish(&directory, &host, Ok(())).unwrap();
    let file = directory.join("load-final-authentication-proof.json");
    let metadata = std::fs::metadata(&file).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert!(metadata.len() <= 256 * 1024);
    let proof: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(proof["schema"], "rom-application-load-authentication-v2");
    assert_eq!(proof["observer_enabled"], true);
    assert_eq!(proof["lifecycle_result"], "succeeded");
    assert_eq!(proof["snapshot"]["schema_version"], 2);
    assert!(
        proof["snapshot"]["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|operation| operation["outstanding"] == 0)
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn host_failure_still_records_evidence_and_retains_the_failure() {
    let (directory, host) = fixture().await;
    let error = finish(&directory, &host, Err(Error::Storage)).unwrap_err();
    assert_eq!(error.downcast_ref::<Error>(), Some(&Error::Storage));
    let proof: serde_json::Value = serde_json::from_slice(
        &std::fs::read(directory.join("load-final-authentication-proof.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(proof["lifecycle_result"], "failed");
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn an_existing_proof_is_preserved_and_cannot_mask_host_failure() {
    let (directory, host) = fixture().await;
    let file = directory.join("load-final-authentication-proof.json");
    std::fs::write(&file, b"preserved").unwrap();
    assert!(finish(&directory, &host, Ok(())).is_err());
    let error = finish(&directory, &host, Err(Error::Storage)).unwrap_err();
    assert_eq!(error.downcast_ref::<Error>(), Some(&Error::Storage));
    assert_eq!(std::fs::read(&file).unwrap(), b"preserved");
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn occupied_listener_fails_before_host_and_preserves_unclaimed_work() {
    let (directory, host) = fixture().await;
    drop(host);
    std::fs::create_dir(directory.join("objects")).unwrap();
    let database = crate::database::Database::open("sqlite", &directory.join("database")).unwrap();
    let storage = database.storage();
    let runtime = crate::application::runtime(
        storage.clone(),
        Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    )
    .unwrap();
    runtime
        .execute(&rom_demo::bootstrap_actor(), crate::application::seeded(0))
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
    let original = storage.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(original.records.len(), 1);
    drop(runtime);
    drop(storage);
    drop(database);
    let occupied = tokio::net::TcpListener::bind("127.0.0.1:44391")
        .await
        .unwrap();
    let config = crate::configuration::Configuration {
        mode: "serve".into(),
        adapter: "sqlite".into(),
        directory: directory.clone(),
        source_directory: directory.join("unused-source"),
        backup_directory: directory.join("unused-backup"),
        issuer: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/".into(),
        client_id: "fixture".into(),
        client_secret: "synthetic-fixture".into(),
        verified_synthetic_subject: "fixture".into(),
        stop_file: directory.join("stop"),
    };
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        crate::runner::serve(config, false),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::AddrInUse
    );
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let database = crate::database::Database::open("sqlite", &directory.join("database")).unwrap();
    let storage = database.storage();
    let after = storage.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(
        serde_json::to_vec(&original).unwrap(),
        serde_json::to_vec(&after).unwrap()
    );
    assert!(
        !directory
            .join("load-final-authentication-proof.json")
            .exists()
    );
    drop(occupied);
    drop(storage);
    drop(database);
    std::fs::remove_dir_all(directory).unwrap();
}
