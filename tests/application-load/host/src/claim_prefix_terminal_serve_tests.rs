//! Declare beside work_terminal_tests under stage_measurements_tests.
use super::{actual_serve_fixture, actual_serve_fixture_with_existing, read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
const NAME: &str = "load-final-claim-prefix.json";
#[tokio::test]
async fn claim_prefix_actual_serve_exports_complete_private_proof() {
    let directory = actual_serve_fixture().await;
    let metadata = std::fs::symlink_metadata(directory.join(NAME)).unwrap();
    assert!(metadata.is_file());
    assert!(metadata.len() <= 512 * 1024);
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    let value = read(&directory, NAME);
    assert_eq!(
        value["schema"],
        "rom-application-load-terminal-claim-prefix-v1"
    );
    assert_eq!(value["baseline"]["entries"], serde_json::json!([]));
    for field in [
        "fresh_baseline",
        "clean",
        "complete",
        "producers_stopped",
        "drained_accounting",
    ] {
        assert_eq!(value[field], true);
    }
    assert_eq!(value["acceptance"], false);
    assert_eq!(value["acknowledgement_attribution"], false);
    std::fs::remove_dir_all(directory).unwrap();
}
#[tokio::test]
async fn claim_prefix_create_failure_still_attempts_existing_independent_proofs() {
    let (directory, result) = actual_serve_fixture_with_existing(Some(NAME)).await;
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(directory.join(NAME)).unwrap(),
        b"preserved terminal witness"
    );
    for name in [
        "load-final-work-utilization.json",
        "load-final-work-batches.json",
        "load-final-storage-stages.json",
        "load-final-storage-publication.json",
        "load-final-core-overload-proof.json",
        "load-final-authentication-proof.json",
    ] {
        assert_eq!(read(&directory, name)["lifecycle_result"], "succeeded");
    }
    std::fs::remove_dir_all(directory).unwrap();
}
#[tokio::test]
async fn claim_prefix_is_attempted_after_existing_work_proof_failure() {
    let existing = "load-final-work-utilization.json";
    let (directory, result) = actual_serve_fixture_with_existing(Some(existing)).await;
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(directory.join(existing)).unwrap(),
        b"preserved terminal witness"
    );
    let value = read(&directory, NAME);
    assert_eq!(value["complete"], true);
    assert_eq!(value["acceptance"], false);
    assert_eq!(value["lifecycle_result"], "succeeded");
    std::fs::remove_dir_all(directory).unwrap();
}
