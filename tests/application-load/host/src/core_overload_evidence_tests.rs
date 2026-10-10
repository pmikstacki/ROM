//! Private core evidence writer and lifecycle error preservation regressions.
use crate::core_overload_evidence::{finish, write_snapshot};
use rom::{Actor, CoreOverloadStats, Error, IntakeState, Runtime};
use rom_studio_host::{HostConfig, StudioHost};
use std::{os::unix::fs::PermissionsExt, sync::Arc};

async fn fixture() -> (std::path::PathBuf, Runtime, StudioHost) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "rom-load-core-proof-{}-{nonce}",
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
    let host = StudioHost::new(runtime.clone(), config).unwrap();
    host.shutdown().await.unwrap();
    (directory, runtime, host)
}
fn read(directory: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(directory.join("load-final-core-overload-proof.json")).unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn zero_counts_write_exact_private_fixed_fields_and_cannot_replace() {
    let (d, r, _) = fixture().await;
    finish(&d, &r, CoreOverloadStats::default(), &Ok(()), true).unwrap();
    let p = d.join("load-final-core-overload-proof.json");
    let before = std::fs::read(&p).unwrap();
    let m = std::fs::metadata(&p).unwrap();
    assert!(m.len() <= 4096);
    assert_eq!(m.permissions().mode() & 0o777, 0o600);
    let proof = read(&d);
    assert_eq!(proof["exact_counts"], true);
    assert_eq!(proof["final"]["io_no_permits"], 0);
    let mut keys = proof
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "baseline",
            "drained_accounting",
            "exact_counts",
            "final",
            "fresh_baseline",
            "intake",
            "lifecycle_result",
            "overflowed",
            "owned_work",
            "producers_stopped",
            "schema",
            "scope"
        ]
    );
    assert!(finish(&d, &r, CoreOverloadStats::default(), &Ok(()), true).is_err());
    assert_eq!(std::fs::read(p).unwrap(), before);
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn stopped_owned_zero_cannot_substitute_for_completed_async_producers() {
    let (d, r, _) = fixture().await;
    assert!(finish(&d, &r, CoreOverloadStats::default(), &Ok(()), false).is_err());
    let p = read(&d);
    assert_eq!(p["drained_accounting"], true);
    assert_eq!(p["exact_counts"], false);
    assert_eq!(p["producers_stopped"], false);
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn overflow_and_nonzero_baseline_never_claim_exact_or_subtract_totals() {
    for baseline_case in [false, true] {
        let (d, r, _) = fixture().await;
        let baseline = if baseline_case {
            CoreOverloadStats {
                io_no_permits: 3,
                ..Default::default()
            }
        } else {
            CoreOverloadStats::default()
        };
        let counts = CoreOverloadStats {
            io_no_permits: u64::MAX,
            overflowed: true,
            ..Default::default()
        };
        assert!(write_snapshot(&d, baseline, counts, r.status().unwrap(), true, true).is_err());
        let p = read(&d);
        assert_eq!(p["exact_counts"], false);
        assert_eq!(p["final"]["io_no_permits"], u64::MAX);
        assert_eq!(
            p["baseline"]["io_no_permits"],
            if baseline_case { 3 } else { 0 }
        );
        std::fs::remove_dir_all(d).unwrap();
    }
}
#[tokio::test]
async fn open_intake_or_remaining_owned_work_rejects_completeness() {
    for open in [false, true] {
        let (d, r, _) = fixture().await;
        let mut status = r.status().unwrap();
        if open {
            status.intake = IntakeState::Open;
        } else {
            status.owned_work = 1;
        }
        assert!(
            write_snapshot(
                &d,
                CoreOverloadStats::default(),
                CoreOverloadStats::default(),
                status,
                true,
                true
            )
            .is_err()
        );
        assert_eq!(read(&d)["exact_counts"], false);
        std::fs::remove_dir_all(d).unwrap();
    }
}
#[tokio::test]
async fn core_writer_failure_still_captures_auth_and_preserves_original_host_error() {
    let (d, r, h) = fixture().await;
    std::fs::write(d.join("load-final-core-overload-proof.json"), b"preserved").unwrap();
    // Same runner order: core borrows original; auth still runs; auth/original wins.
    let lifecycle = Err(Error::Storage);
    let core = finish(&d, &r, CoreOverloadStats::default(), &lifecycle, true);
    let authentication = crate::authentication_evidence::finish(&d, &h, lifecycle);
    let error = authentication.and(core).unwrap_err();
    assert_eq!(error.downcast_ref::<Error>(), Some(&Error::Storage));
    assert!(d.join("load-final-authentication-proof.json").exists());
    assert_eq!(
        std::fs::read(d.join("load-final-core-overload-proof.json")).unwrap(),
        b"preserved"
    );
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn successful_lifecycle_with_failed_core_proof_fails_after_auth_capture() {
    let (d, r, h) = fixture().await;
    std::fs::write(d.join("load-final-core-overload-proof.json"), b"preserved").unwrap();
    let lifecycle = Ok(());
    let core = finish(&d, &r, CoreOverloadStats::default(), &lifecycle, true);
    let auth = crate::authentication_evidence::finish(&d, &h, lifecycle);
    assert!(auth.and(core).is_err());
    assert!(d.join("load-final-authentication-proof.json").exists());
    std::fs::remove_dir_all(d).unwrap();
}
#[tokio::test]
async fn both_proof_failures_cannot_replace_the_first_host_or_worker_error() {
    for original in [Error::Storage, Error::Panicked] {
        let (d, r, h) = fixture().await;
        std::fs::write(
            d.join("load-final-core-overload-proof.json"),
            b"core preserved",
        )
        .unwrap();
        std::fs::write(
            d.join("load-final-authentication-proof.json"),
            b"auth preserved",
        )
        .unwrap();
        let lifecycle = Err(original.clone());
        let core = finish(&d, &r, CoreOverloadStats::default(), &lifecycle, true);
        let auth = crate::authentication_evidence::finish(&d, &h, lifecycle);
        let error = auth.and(core).unwrap_err();
        assert_eq!(error.downcast_ref::<Error>(), Some(&original));
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[tokio::test]
async fn nonzero_baseline_without_overflow_rejects_a_false_fresh_lifecycle() {
    let (d, r, _) = fixture().await;
    let baseline = CoreOverloadStats {
        action_no_permits: 1,
        ..Default::default()
    };
    assert!(
        write_snapshot(
            &d,
            baseline,
            CoreOverloadStats::default(),
            r.status().unwrap(),
            true,
            true
        )
        .is_err()
    );
    let proof = read(&d);
    assert_eq!(proof["overflowed"], false);
    assert_eq!(proof["fresh_baseline"], false);
    assert_eq!(proof["exact_counts"], false);
    std::fs::remove_dir_all(d).unwrap();
}
