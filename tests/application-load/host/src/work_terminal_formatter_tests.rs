//! Fixed-size formatter controls; synthetic aggregate inputs are not runtime measurements.
use super::*;
use crate::work_batch_observation::{Snapshot, WidthCounts};
use rom_sqlite::{SemanticEffect, WorkCommitMeasurement, WorkCommitOrigin};
use serde_json::Value;

fn empty_native() -> WorkUtilizationSnapshot {
    WorkUtilizationSnapshot {
        entries: vec![],
        overflow: false,
        unavailable: false,
    }
}
fn empty_host() -> Snapshot {
    Snapshot {
        prefix: vec![],
        atomic: vec![],
        prefix_failed_calls: 0,
        prefix_failed_ns: 0,
        unavailable: false,
    }
}
fn maximum_native() -> WorkUtilizationSnapshot {
    let mut snapshot = empty_native();
    for origin in [
        WorkCommitOrigin::ClaimPrefix,
        WorkCommitOrigin::Claim,
        WorkCommitOrigin::Materialize,
        WorkCommitOrigin::Finish,
        WorkCommitOrigin::DeliveryStarted,
        WorkCommitOrigin::DeliveryFinished,
        WorkCommitOrigin::Other,
    ] {
        for width in 0..=32 {
            for semantic in [SemanticEffect::Unchanged, SemanticEffect::Changed] {
                snapshot.entries.push(WorkCommitMeasurement {
                    origin,
                    width,
                    semantic,
                    native_successes: u64::MAX,
                    native_failures: u64::MAX,
                    elapsed_ns: u64::MAX,
                    dropped_samples: 0,
                });
            }
        }
    }
    snapshot
}
fn maximum_host() -> Snapshot {
    let cells = || {
        (0..=32)
            .map(|width| WidthCounts {
                width,
                calls: u64::MAX,
                failures: u64::MAX,
                elapsed_ns: u64::MAX,
                source_claims: u64::MAX,
                notification_claims: u64::MAX,
                resolution_only_claims: u64::MAX,
            })
            .collect()
    };
    Snapshot {
        prefix: cells(),
        atomic: cells(),
        prefix_failed_calls: u64::MAX,
        prefix_failed_ns: u64::MAX,
        unavailable: false,
    }
}

#[test]
fn work_terminal_formatters_preserve_u64_max_and_all_fixed_cells() {
    let n = maximum_native();
    assert_eq!(n.entries.len(), 462);
    let formatted = native::summary(&n);
    assert_eq!(formatted["entries"].as_array().unwrap().len(), 462);
    for entry in formatted["entries"].as_array().unwrap() {
        for field in ["native_successes", "native_failures", "elapsed_ns"] {
            assert_eq!(entry[field], "18446744073709551615");
        }
        assert_eq!(entry["dropped_samples"], "0");
    }
    let h = maximum_host();
    let formatted = host::summary(Some(&h));
    for field in ["prefix", "atomic"] {
        assert_eq!(formatted[field].as_array().unwrap().len(), 33);
        for entry in formatted[field].as_array().unwrap() {
            for field in [
                "calls",
                "failures",
                "elapsed_ns",
                "source_claims",
                "notification_claims",
                "resolution_only_claims",
            ] {
                assert_eq!(entry[field], "18446744073709551615");
            }
        }
    }
    assert_eq!(formatted["prefix_failed_calls"], "18446744073709551615");
    assert_eq!(formatted["prefix_failed_ns"], "18446744073709551615");
}

#[test]
fn work_terminal_invalid_snapshots_cannot_be_fresh_clean_evidence() {
    assert!(native::fresh(&empty_native()));
    assert!(host::fresh(Some(&empty_host())));
    assert!(!host::clean(None));
    assert!(!host::fresh(None));
    assert_eq!(
        host::summary(None),
        serde_json::json!({"unavailable": true})
    );
    let mut n = empty_native();
    n.overflow = true;
    assert!(!native::clean(&n) && !native::fresh(&n));
    n.overflow = false;
    n.unavailable = true;
    assert!(!native::clean(&n) && !native::fresh(&n));
    assert!(!native::fresh(&maximum_native()));
    let mut h = empty_host();
    h.unavailable = true;
    assert!(!host::clean(Some(&h)) && !host::fresh(Some(&h)));
    assert!(!host::fresh(Some(&maximum_host())));
}

#[test]
fn work_terminal_failed_prefix_is_separate_from_sparse_idle_cells() {
    let mut h = empty_host();
    h.prefix_failed_calls = 1;
    h.prefix_failed_ns = 7;
    h.prefix.push(WidthCounts {
        width: 0,
        ..Default::default()
    });
    let formatted = host::summary(Some(&h));
    assert_eq!(formatted["prefix"], serde_json::json!([]));
    assert_eq!(formatted["prefix_failed_calls"], "1");
    assert_eq!(formatted["prefix_failed_ns"], "7");
    assert!(!host::fresh(Some(&h)));
}

#[tokio::test]
async fn work_terminal_maximum_cells_fit_whole_files_and_dirty_baselines_remain_incomplete() {
    use std::{
        os::unix::fs::PermissionsExt,
        sync::{Arc, atomic::AtomicUsize},
    };
    let directory = std::env::temp_dir().join(format!(
        "rom-terminal-format-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Arc::new(rom_sqlite::Sqlite::open(directory.join("database")).unwrap());
    let observed = ObservedStorage::new(storage.clone());
    let observer = storage.observe_stages();
    let observation = WorkTerminalObservation {
        observer,
        native_baseline: maximum_native(),
        host_baseline: Some(maximum_host()),
    };
    let runtime = crate::application::runtime(storage, Arc::new(AtomicUsize::new(0))).unwrap();
    runtime.shutdown().await.unwrap();
    assert!(
        observation
            .finish(&directory, &observed, &runtime, &Ok(()), true)
            .is_err()
    );
    for (name, maximum) in [
        ("load-final-work-utilization.json", 128 * 1024),
        ("load-final-work-batches.json", 32 * 1024),
    ] {
        let path = directory.join(name);
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() <= maximum);
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        let p: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(p["complete"], false);
        assert_eq!(p["fresh_baseline"], false);
        assert_eq!(p["acceptance"], false);
    }
    drop(runtime);
    drop(observed);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn work_terminal_overflow_unavailable_and_missing_baselines_never_claim_complete() {
    use std::sync::{Arc, atomic::AtomicUsize};
    for case in 0..4 {
        let directory = std::env::temp_dir().join(format!(
            "rom-terminal-unavailable-{}-{case}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let storage = Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap());
        let observed = ObservedStorage::new(storage.clone());
        let mut native_baseline = empty_native();
        let mut host_baseline = Some(empty_host());
        match case {
            0 => native_baseline.overflow = true,
            1 => native_baseline.unavailable = true,
            2 => host_baseline = None,
            3 => host_baseline.as_mut().unwrap().unavailable = true,
            _ => unreachable!(),
        }
        let observation = WorkTerminalObservation {
            observer: storage.observe_stages(),
            native_baseline,
            host_baseline,
        };
        let runtime = crate::application::runtime(storage, Arc::new(AtomicUsize::new(0))).unwrap();
        runtime.shutdown().await.unwrap();
        assert!(
            observation
                .finish(&directory, &observed, &runtime, &Ok(()), true)
                .is_err()
        );
        for (name, incomplete) in [
            ("load-final-work-utilization.json", case < 2),
            ("load-final-work-batches.json", case >= 2),
        ] {
            let p: Value =
                serde_json::from_slice(&std::fs::read(directory.join(name)).unwrap()).unwrap();
            assert_eq!(p["complete"], !incomplete);
            assert_eq!(p["fresh_baseline"], !incomplete);
            assert_eq!(p["acceptance"], false);
        }
        drop(runtime);
        drop(observed);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
