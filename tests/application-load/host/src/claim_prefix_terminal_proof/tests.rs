//! Synthetic formatter inputs; not measured Runtime or mixed evidence.
use super::*;
use rom_sqlite::{ClaimPrefixDisposition as D, ClaimPrefixMeasurement, ClaimPrefixReason as R};
fn empty() -> ClaimPrefixSnapshot {
    ClaimPrefixSnapshot::default()
}
fn cell(reason: R, width: u8, disposition: D) -> ClaimPrefixMeasurement {
    ClaimPrefixMeasurement {
        reason,
        width,
        disposition,
        native_successes: u64::MAX,
        native_failures: u64::MAX,
        dropped_samples: 0,
    }
}
fn all_cells() -> ClaimPrefixSnapshot {
    let mut snapshot = empty();
    for reason in [
        R::RequestedLimit,
        R::Idle,
        R::MaintenanceChanged,
        R::ResolutionBarrier,
        R::NotificationBarrier,
        R::ActionBarrier,
        R::DuplicateRoot,
    ] {
        for width in 0..=32 {
            for disposition in [D::Unrestricted, D::AdmittedSingleton, D::Withheld] {
                snapshot.entries.push(cell(reason, width, disposition));
            }
        }
    }
    snapshot
}
fn stopped() -> TerminalState {
    TerminalState {
        lifecycle_succeeded: true,
        producers_stopped: true,
        drained: true,
        status_available: true,
        intake: Some(rom::IntakeState::Stopped),
        owned_work: Some(0),
    }
}

#[test]
fn claim_prefix_whole_json_bound_covers_both_maximum_arrays_and_u64_precision() {
    let mut maximum = all_cells();
    for entry in &mut maximum.entries {
        entry.dropped_samples = u64::MAX;
    }
    assert_eq!(maximum.entries.len(), 693);
    let value = proof(Some(&maximum), Some(&maximum), stopped());
    assert_eq!(value["complete"], false);
    assert_eq!(value["fresh_baseline"], false);
    for summary in ["baseline", "final"] {
        let entries = value[summary]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 693);
        for entry in entries {
            assert!(serde_json::to_vec(entry).unwrap().len() <= 256);
            assert_eq!(entry["native_successes"], "18446744073709551615");
            assert_eq!(entry["native_failures"], "18446744073709551615");
            assert_eq!(entry["dropped_samples"], "18446744073709551615");
        }
    }
    assert!(serde_json::to_vec(&value).unwrap().len() <= 2 * 693 * 256 + 4096);
    const { assert!(2 * 693 * 256 + 4096 < MAXIMUM_BYTES) };
}

#[test]
fn claim_prefix_all_semantically_possible_cells_are_clean_but_duplicate_is_invalid() {
    let maximum = all_cells();
    let entries = maximum
        .entries
        .into_iter()
        .filter(|entry| {
            let snapshot = ClaimPrefixSnapshot {
                entries: vec![*entry],
                ..empty()
            };
            format::clean(Some(&snapshot))
        })
        .collect();
    let mut valid = ClaimPrefixSnapshot { entries, ..empty() };
    // 31 requested +32 idle +32 maintenance +3*32 barriers +31 duplicate-root.
    assert_eq!(valid.entries.len(), 222);
    assert!(format::clean(Some(&valid)));
    assert!(!format::fresh(Some(&valid)));
    valid.entries.push(valid.entries[0]);
    assert!(!format::clean(Some(&valid)));
    assert_eq!(format::summary(Some(&valid))["valid"], false);
}

#[test]
fn claim_prefix_invalid_combinations_cannot_qualify_complete_evidence() {
    for entry in [
        cell(R::RequestedLimit, 1, D::Unrestricted),
        cell(R::Idle, 32, D::Unrestricted),
        cell(R::DuplicateRoot, 1, D::AdmittedSingleton),
        cell(R::NotificationBarrier, 0, D::Withheld),
        cell(R::ResolutionBarrier, 2, D::AdmittedSingleton),
        cell(R::ActionBarrier, 32, D::Withheld),
        cell(R::MaintenanceChanged, 0, D::Withheld),
        cell(R::RequestedLimit, 33, D::Unrestricted),
    ] {
        let snapshot = ClaimPrefixSnapshot {
            entries: vec![entry],
            ..empty()
        };
        assert!(!format::clean(Some(&snapshot)));
        assert_eq!(
            proof(Some(&empty()), Some(&snapshot), stopped())["complete"],
            false
        );
    }
    let zero = ClaimPrefixSnapshot {
        entries: vec![ClaimPrefixMeasurement {
            native_successes: 0,
            native_failures: 0,
            ..cell(R::Idle, 0, D::Unrestricted)
        }],
        ..empty()
    };
    assert!(!format::clean(Some(&zero)));
}

#[test]
fn claim_prefix_missing_overlong_overflow_unavailable_and_dropped_are_explicit() {
    let missing = proof(None, None, stopped());
    assert_eq!(missing["baseline"]["available"], false);
    assert_eq!(missing["baseline"]["entries"], Value::Null);
    assert_eq!(missing["baseline"]["overflow"], Value::Null);
    assert_eq!(missing["complete"], false);
    let mut oversized = all_cells();
    oversized.entries.push(oversized.entries[0]);
    let summary = format::summary(Some(&oversized));
    assert_eq!(summary["entries"], Value::Null);
    assert_eq!(summary["valid"], false);
    for case in 0..3 {
        let mut snapshot = empty();
        match case {
            0 => snapshot.overflow = true,
            1 => snapshot.unavailable = true,
            _ => {
                let mut entry = cell(R::Idle, 0, D::Unrestricted);
                entry.dropped_samples = 1;
                snapshot.entries.push(entry);
            }
        }
        assert!(!format::clean(Some(&snapshot)));
        assert_eq!(
            proof(Some(&empty()), Some(&snapshot), stopped())["complete"],
            false
        );
    }
}

#[test]
fn claim_prefix_quiescence_cannot_be_inferred_from_accounting_alone() {
    for case in 0..6 {
        let mut state = stopped();
        match case {
            0 => state.producers_stopped = false,
            1 => state.drained = false,
            2 => {
                state.status_available = false;
                state.intake = None;
                state.owned_work = None;
            }
            3 => state.lifecycle_succeeded = false,
            4 => state.intake = Some(rom::IntakeState::Open),
            _ => state.owned_work = Some(1),
        }
        let value = proof(Some(&empty()), Some(&empty()), state);
        assert_eq!(value["complete"], false);
        assert_eq!(value["acceptance"], false);
    }
    let value = proof(Some(&empty()), Some(&empty()), stopped());
    assert_eq!(value["complete"], true);
    assert_eq!(value["acknowledgement_attribution"], false);
    assert_eq!(value["wal_or_fsync_attribution"], false);
}

fn directory(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rom-claim-prefix-format-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
#[test]
fn claim_prefix_real_exclusive_writer_preserves_whole_maximum_record_and_refuses_oversize() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let directory = directory("exclusive");
    let mut maximum = all_cells();
    for entry in &mut maximum.entries {
        entry.dropped_samples = u64::MAX;
    }
    let value = proof(Some(&maximum), Some(&maximum), stopped());
    let encoded = serde_json::to_vec(&value).unwrap();
    assert!(encoded.len() <= MAXIMUM_BYTES);
    crate::runner::write_bounded(&directory, "maximum.json", &value, MAXIMUM_BYTES).unwrap();
    let path = directory.join("maximum.json");
    let metadata = std::fs::symlink_metadata(&path).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), encoded);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap(),
        value
    );
    assert!(
        crate::runner::write_bounded(
            &directory,
            "maximum.json",
            &json!({"replacement":true}),
            MAXIMUM_BYTES
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), encoded);
    let exact = json!("a".repeat(MAXIMUM_BYTES - 2));
    assert_eq!(serde_json::to_vec(&exact).unwrap().len(), MAXIMUM_BYTES);
    crate::runner::write_bounded(&directory, "exact.json", &exact, MAXIMUM_BYTES).unwrap();
    let oversized = json!("a".repeat(MAXIMUM_BYTES - 1));
    assert_eq!(
        serde_json::to_vec(&oversized).unwrap().len(),
        MAXIMUM_BYTES + 1
    );
    assert!(
        crate::runner::write_bounded(&directory, "oversized.json", &oversized, MAXIMUM_BYTES)
            .is_err()
    );
    assert!(!directory.join("oversized.json").exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn claim_prefix_failed_lifecycle_writes_incomplete_after_actual_runtime_shutdown() {
    use std::sync::{Arc, atomic::AtomicUsize};
    let directory = directory("failed-lifecycle");
    let storage = Arc::new(rom_sqlite::Sqlite::open(directory.join("database")).unwrap());
    let observation =
        ClaimPrefixTerminalObservation::begin(&Database::Sqlite(storage.clone())).unwrap();
    let runtime =
        crate::application::runtime(storage.clone(), Arc::new(AtomicUsize::new(0))).unwrap();
    runtime.shutdown().await.unwrap();
    let result = observation.finish(&directory, &runtime, &Err(rom::Error::Storage), false);
    drop(runtime);
    drop(storage);
    assert!(result.is_err());
    let value: Value = serde_json::from_slice(
        &std::fs::read(directory.join("load-final-claim-prefix.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(value["lifecycle_result"], "failed");
    assert_eq!(value["drained_accounting"], true);
    assert_eq!(value["owned_work"], 0);
    assert_eq!(value["producers_stopped"], false);
    assert_eq!(value["complete"], false);
    assert_eq!(value["acceptance"], false);
    std::fs::remove_dir_all(directory).unwrap();
}
