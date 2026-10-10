//! Real terminal serve exports; these tests do not invoke future formatters directly.
use super::actual_serve_fixture;
use std::{os::unix::fs::PermissionsExt, path::Path};

fn proof(directory: &Path, name: &str, schema: &str, maximum: u64) -> serde_json::Value {
    let path = directory.join(name);
    assert!(
        path.exists(),
        "actual serve did not export {name}: {}",
        directory.display()
    );
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_file());
    assert!(metadata.len() <= maximum);
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    let proof: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(proof["schema"], schema);
    assert_eq!(proof["diagnostic_only"], true);
    assert_eq!(proof["acceptance"], false);
    for field in [
        "complete",
        "producers_stopped",
        "drained_accounting",
        "fresh_baseline",
        "status_available",
    ] {
        assert_eq!(proof[field], true, "terminal proof field {field}");
    }
    assert_eq!(proof["intake"], "Stopped");
    assert_eq!(proof["owned_work"], 0);
    assert_eq!(proof["lifecycle_result"], "succeeded");
    proof
}

#[tokio::test]
async fn work_terminal_actual_serve_exports_native() {
    // Shared real serve returns only after Host and worker shutdown.
    let directory = actual_serve_fixture().await;
    let proof = proof(
        &directory,
        "load-final-work-utilization.json",
        "rom-application-load-terminal-work-utilization-v1",
        128 * 1024,
    );
    assert_eq!(proof["baseline"]["entries"], serde_json::json!([]));
    assert_eq!(proof["baseline"]["overflow"], false);
    assert_eq!(proof["baseline"]["unavailable"], false);
    assert_eq!(proof["final"]["overflow"], false);
    assert_eq!(proof["final"]["unavailable"], false);
    assert!(proof["final"]["entries"].as_array().unwrap().len() <= 462);
    // Immediate stop legitimately permits no native Work activity.
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn work_terminal_actual_serve_exports_host() {
    let directory = actual_serve_fixture().await;
    let proof = proof(
        &directory,
        "load-final-work-batches.json",
        "rom-application-load-terminal-work-batches-v1",
        32 * 1024,
    );
    assert_eq!(proof["baseline"]["prefix"], serde_json::json!([]));
    assert_eq!(proof["baseline"]["atomic"], serde_json::json!([]));
    assert_eq!(proof["baseline"]["prefix_failed_calls"], "0");
    assert_eq!(proof["baseline"]["prefix_failed_ns"], "0");
    assert_eq!(proof["baseline"]["unavailable"], false);
    assert_eq!(proof["final"]["unavailable"], false);
    for category in ["prefix", "atomic"] {
        assert!(proof["final"][category].as_array().unwrap().len() <= 33);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn work_terminal_actual_serve_attempts_sibling_and_existing_proofs_after_create_failure() {
    for (existing, sibling) in [
        (
            "load-final-work-utilization.json",
            "load-final-work-batches.json",
        ),
        (
            "load-final-work-batches.json",
            "load-final-work-utilization.json",
        ),
    ] {
        let (directory, result) = super::actual_serve_fixture_with_existing(Some(existing)).await;
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(directory.join(existing)).unwrap(),
            b"preserved terminal witness"
        );
        for name in [
            sibling,
            "load-final-storage-stages.json",
            "load-final-storage-publication.json",
            "load-final-core-overload-proof.json",
            "load-final-authentication-proof.json",
        ] {
            let p = super::read(&directory, name);
            assert_eq!(
                p["lifecycle_result"], "succeeded",
                "independent writer {name}"
            );
        }
        assert_eq!(super::read(&directory, sibling)["complete"], true);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

async fn file_backed_fixture(
    process: bool,
) -> (
    std::path::PathBuf,
    rom::Runtime,
    std::sync::Arc<rom_sqlite::Sqlite>,
    std::sync::Arc<crate::ObservedStorage>,
    crate::work_terminal_proof::WorkTerminalObservation,
) {
    use rom::Storage;
    use std::sync::{Arc, atomic::AtomicUsize};
    let directory = std::env::temp_dir().join(format!(
        "rom-work-terminal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let database = crate::database::Database::open("sqlite", &directory.join("database")).unwrap();
    let native = match &database {
        crate::database::Database::Sqlite(native) => native.clone(),
        crate::database::Database::Redb(_) => unreachable!(),
    };
    let observed = Arc::new(crate::ObservedStorage::new(native.clone()));
    let observation =
        crate::work_terminal_proof::WorkTerminalObservation::begin(&database, &observed).unwrap();
    let runtime =
        crate::application::runtime(observed.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    let seeded = runtime
        .execute(&rom_demo::bootstrap_actor(), crate::application::seeded(0))
        .await;
    let processed = if process {
        runtime.process_reactions(32).await.map(|_| ())
    } else {
        Ok(())
    };
    let shutdown = runtime.shutdown().await;
    seeded.unwrap();
    processed.unwrap();
    shutdown.unwrap();
    let ledger = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert_eq!(ledger.records.len(), 1);
    if process {
        assert_eq!(ledger.records[0].state, rom::WorkState::Done);
    }
    (directory, runtime, native, observed, observation)
}

#[tokio::test]
async fn work_terminal_real_file_work_exports_nonzero_fixed_cells_and_strings() {
    let (directory, runtime, native, observed, observation) = file_backed_fixture(true).await;
    observation
        .finish(&directory, &observed, &runtime, &Ok(()), true)
        .unwrap();
    let native_proof = proof(
        &directory,
        "load-final-work-utilization.json",
        "rom-application-load-terminal-work-utilization-v1",
        128 * 1024,
    );
    let host_proof = proof(
        &directory,
        "load-final-work-batches.json",
        "rom-application-load-terminal-work-batches-v1",
        32 * 1024,
    );
    assert_eq!(native_proof["baseline"]["entries"], serde_json::json!([]));
    let entries = native_proof["final"]["entries"].as_array().unwrap();
    assert!(entries.iter().any(|entry| {
        entry["semantic"] == "changed"
            && entry["native_successes"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                > 0
    }));
    for entry in entries {
        assert!(entry["width"].as_u64().unwrap() <= 32);
        for field in [
            "native_successes",
            "native_failures",
            "elapsed_ns",
            "dropped_samples",
        ] {
            let text = entry[field].as_str().unwrap();
            assert_eq!(text.parse::<u64>().unwrap().to_string(), text);
        }
    }
    assert_eq!(host_proof["baseline"]["prefix"], serde_json::json!([]));
    assert_eq!(host_proof["baseline"]["atomic"], serde_json::json!([]));
    assert!(
        host_proof["final"]["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["width"].as_u64().unwrap() > 0
                && entry["calls"].as_str().unwrap().parse::<u64>().unwrap() > 0)
    );
    assert_eq!(host_proof["unknown_outcome_classification"], "unavailable");
    drop(runtime);
    drop(observed);
    drop(native);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn work_terminal_drained_runtime_without_producer_completion_is_incomplete() {
    let (directory, runtime, native, observed, observation) = file_backed_fixture(true).await;
    assert!(
        observation
            .finish(&directory, &observed, &runtime, &Ok(()), false)
            .is_err()
    );
    for name in [
        "load-final-work-utilization.json",
        "load-final-work-batches.json",
    ] {
        let p = super::read(&directory, name);
        assert_eq!(p["complete"], false);
        assert_eq!(p["producers_stopped"], false);
        assert_eq!(p["drained_accounting"], true);
    }
    drop(runtime);
    drop(observed);
    drop(native);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn work_terminal_lost_ack_preserves_native_success_failed_prefix_and_reopened_ledger() {
    use rom::Storage;
    use std::sync::Arc;
    let (directory, runtime, native, observed, observation) = file_backed_fixture(false).await;
    let before = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    let now = before.records[0].pending.cause.started_at;
    native.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == usize::MAX {
            Err(rom::Error::Storage)
        } else {
            Ok(())
        }
    })));
    let result = observed.reaction_claim_prefix(now, 16);
    native.on_commit(None);
    // The synchronous producer has returned; Runtime was already shut down by the fixture.
    assert_eq!(result, Err(rom::Error::Unknown));
    let committed = native.work_snapshot(12000, 16 * 1024 * 1024).unwrap();
    assert!(matches!(
        committed.records[0].state,
        rom::WorkState::Leased { .. }
    ));
    observation
        .finish(&directory, &observed, &runtime, &Ok(()), true)
        .unwrap();
    let n = super::read(&directory, "load-final-work-utilization.json");
    let entries = n["final"]["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["origin"] == "claim_prefix"
        && e["width"] == 1
        && e["semantic"] == "changed"
        && e["native_successes"] == "1"
        && e["native_failures"] == "0"));
    let h = super::read(&directory, "load-final-work-batches.json");
    assert_eq!(h["final"]["prefix_failed_calls"], "1");
    assert_eq!(h["final"]["prefix"], serde_json::json!([]));
    assert_eq!(h["unknown_outcome_classification"], "unavailable");
    drop(runtime);
    drop(observed);
    drop(native);
    let reopened = crate::database::Database::open("sqlite", &directory.join("database")).unwrap();
    assert_eq!(
        reopened
            .storage()
            .work_snapshot(12000, 16 * 1024 * 1024)
            .unwrap(),
        committed
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}
