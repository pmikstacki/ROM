//! Genuine SQLite operations and terminal diagnostic evidence contract.
use super::LifecycleObservation;
use crate::{application, database::Database};
use rom::{Error, Runtime, Storage, WorkState};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};

async fn fixture() -> (
    PathBuf,
    Runtime,
    Arc<rom_sqlite::Sqlite>,
    LifecycleObservation,
) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rom-load-terminal-stages-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
    let observation = LifecycleObservation::begin(&Database::Sqlite(storage.clone())).unwrap();
    let runtime = application::runtime(storage.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    runtime
        .execute(&rom_demo::bootstrap_actor(), application::seeded(0))
        .await
        .unwrap();
    runtime.process_reactions(32).await.unwrap();
    let ledger = storage.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(ledger.records.len(), 1);
    assert_eq!(ledger.records[0].state, WorkState::Done);
    (directory, runtime, storage, observation)
}
fn read(directory: &std::path::Path, name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(directory.join(name)).unwrap()).unwrap()
}
#[tokio::test]
async fn drained_sqlite_writes_actual_fixed_private_stage_and_publication_evidence() {
    let (d, runtime, _, observation) = fixture().await;
    runtime.shutdown().await.unwrap();
    observation.finish(&d, &runtime, &Ok(()), true).unwrap();
    let stages = read(&d, "load-final-storage-stages.json");
    let publications = read(&d, "load-final-storage-publication.json");
    for (name, proof, cells) in [
        ("load-final-storage-stages.json", &stages, 18),
        ("load-final-storage-publication.json", &publications, 10),
    ] {
        let metadata = std::fs::metadata(d.join(name)).unwrap();
        assert!(metadata.len() <= 16 * 1024);
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(proof["acceptance"], false);
        assert_eq!(proof["complete"], true);
        assert_eq!(proof["producers_stopped"], true);
        assert_eq!(proof["owned_work"], 0);
        assert_eq!(
            proof["baseline"]["entries"].as_array().unwrap().len(),
            cells
        );
        assert!(
            proof["baseline"]["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["samples"] == 0)
        );
        let entries = proof["final"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), cells);
        assert!(entries.iter().any(|x| x["samples"].as_u64().unwrap() > 0));
        assert!(!serde_json::to_string(proof).unwrap().contains("body"));
    }
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn stopped_runtime_cannot_replace_producer_completion() {
    let (d, runtime, _, observation) = fixture().await;
    runtime.shutdown().await.unwrap();
    let result = observation.finish(&d, &runtime, &Ok(()), false);
    assert!(result.is_err());
    for name in [
        "load-final-storage-stages.json",
        "load-final-storage-publication.json",
    ] {
        let proof = read(&d, name);
        assert_eq!(proof["complete"], false);
        assert_eq!(proof["producers_stopped"], false);
    }
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn running_intake_never_claims_complete_even_with_no_owned_work() {
    let (d, runtime, _, observation) = fixture().await;
    let result = observation.finish(&d, &runtime, &Ok(()), true);
    runtime.shutdown().await.unwrap();
    assert!(result.is_err());
    assert_eq!(
        read(&d, "load-final-storage-stages.json")["complete"],
        false
    );
    assert_eq!(
        read(&d, "load-final-storage-publication.json")["complete"],
        false
    );
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn failed_lifecycle_retains_real_samples_without_claiming_success() {
    let (d, runtime, _, observation) = fixture().await;
    runtime.shutdown().await.unwrap();
    observation
        .finish(&d, &runtime, &Err(Error::Storage), true)
        .unwrap();
    for name in [
        "load-final-storage-stages.json",
        "load-final-storage-publication.json",
    ] {
        let proof = read(&d, name);
        assert_eq!(proof["lifecycle_result"], "failed");
        assert_eq!(proof["complete"], true);
        assert!(
            proof["final"]["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["samples"].as_u64().unwrap() > 0)
        );
    }
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn existing_stage_file_is_preserved_and_publication_capture_still_runs() {
    let (d, runtime, _, observation) = fixture().await;
    runtime.shutdown().await.unwrap();
    let original = b"preserved terminal witness";
    std::fs::write(d.join("load-final-storage-stages.json"), original).unwrap();
    assert!(observation.finish(&d, &runtime, &Ok(()), true).is_err());
    assert_eq!(
        std::fs::read(d.join("load-final-storage-stages.json")).unwrap(),
        original
    );
    assert_eq!(
        read(&d, "load-final-storage-publication.json")["complete"],
        true
    );
    std::fs::remove_dir_all(d).unwrap();
}

async fn actual_serve_fixture() -> PathBuf {
    let (directory, result) = actual_serve_fixture_with_existing(None).await;
    result.unwrap();
    directory
}

async fn actual_serve_fixture_with_existing(
    existing: Option<&str>,
) -> (PathBuf, Result<(), Box<dyn std::error::Error>>) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rom-load-stage-serve-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir(directory.join("objects")).unwrap();
    // Create the genuine file-backed store before starting the shared serve body.
    drop(Database::open("sqlite", &directory.join("database")).unwrap());
    let stop = directory.join("stop");
    std::fs::write(&stop, b"").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let configuration = crate::configuration::Configuration {
        mode: "serve".into(),
        adapter: "sqlite".into(),
        directory: directory.clone(),
        source_directory: directory.clone(),
        backup_directory: directory.join("unused-backup"),
        issuer: "https://127.0.0.1:44392/application/o/rom-synthetic-identity/".into(),
        client_id: "diagnostic-fixture".into(),
        client_secret: "fictitious-unused-secret".into(),
        verified_synthetic_subject: "diagnostic-fixture".into(),
        stop_file: stop,
    };
    if let Some(name) = existing {
        std::fs::write(directory.join(name), b"preserved terminal witness").unwrap();
    }
    let result = crate::runner::serve_with_listener(configuration, false, listener).await;
    // The actual shared lifecycle must close before checking any evidence.
    (directory, result)
}

#[tokio::test]
async fn actual_serve_body_captures_stages_after_host_and_worker_shutdown() {
    let directory = actual_serve_fixture().await;
    for name in [
        "load-final-storage-stages.json",
        "load-final-storage-publication.json",
    ] {
        let proof = read(&directory, name);
        assert_eq!(proof["complete"], true);
        assert_eq!(proof["owned_work"], 0);
        assert_eq!(proof["producers_stopped"], true);
        assert_eq!(proof["acceptance"], false);
    }
    assert_eq!(
        read(&directory, "load-final-core-overload-proof.json")["exact_counts"],
        true
    );
    assert_eq!(
        read(&directory, "load-final-authentication-proof.json")["lifecycle_result"],
        "succeeded"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[path = "work_terminal_tests.rs"]
mod work_terminal;

#[path = "claim_prefix_terminal_serve_tests.rs"]
mod claim_prefix_terminal;
