//! Diagnostic failure cannot modify the accepted authentication result.
use super::*;

#[tokio::test]
async fn poisoned_capture_is_unavailable_but_accepted_work_and_drain_remain_unchanged() {
    let capture = Capture::new();
    let poisoned = capture.clone();
    assert!(
        std::thread::spawn(move || {
            let _held = poisoned.state.lock().unwrap();
            panic!("fixed observer poison control");
        })
        .join()
        .is_err()
    );
    assert!(matches!(capture.snapshot(), Err(Error::Panicked)));
    let supervisor = crate::lifecycle::Supervisor::observed(1, Some(capture.clone()));
    assert_eq!(
        supervisor
            .run_tagged(AuthOperation::Session, async { Ok(9_u32) })
            .await,
        Ok(9)
    );
    supervisor.drain().await.unwrap();
    assert!(matches!(capture.snapshot(), Err(Error::Panicked)));
}

#[test]
fn counter_saturation_is_explicit_and_does_not_wrap() {
    let capture = Capture::new();
    capture.state.lock().unwrap().operations[AuthOperation::Session as usize].attempts = u64::MAX;
    capture.attempt(AuthOperation::Session);
    let snapshot = capture.snapshot().unwrap();
    assert_eq!(
        snapshot.operation(AuthOperation::Session).attempts,
        u64::MAX
    );
    assert!(snapshot.overflowed);
}

#[test]
fn serialized_snapshot_labels_its_bounded_arrays_and_retains_first_stage_failure_after_cap() {
    let capture = Capture::new();
    for _ in 0..33 {
        capture.rejected(AuthOperation::GenericHttpResolver, AuthOutcome::Overloaded);
    }
    capture.record(
        AuthOperation::GenericHttpResolver,
        AuthStage::OriginalTokenVerification,
        AuthOutcome::Denied,
    );
    let snapshot = capture.snapshot().unwrap();
    assert_eq!(snapshot.failures.len(), 32);
    assert_eq!(snapshot.omitted_failures, 2);
    let stage = snapshot.stage(AuthStage::OriginalTokenVerification);
    assert_eq!(stage.denied, 1);
    assert!(stage.first_failure_micros.is_some());
    assert_eq!(stage.first_failure_outcome, Some(AuthOutcome::Denied));
    let json = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert_eq!(
        json["operation_labels"][AuthOperation::GenericHttpResolver as usize],
        "GenericHttpResolver"
    );
    assert_eq!(
        json["stage_labels"][AuthStage::OriginalTokenVerification as usize],
        "OriginalTokenVerification"
    );
}

#[test]
fn v2_full_proof_bounds_all_stages_and_capped_failures() {
    let capture = Capture::new();
    for stage in [
        AuthStage::BlobReserveResult,
        AuthStage::BlobUploadResult,
        AuthStage::BlobUploadUnattached,
        AuthStage::BlobProjectionResult,
    ] {
        for _ in 0..9 {
            capture.record(AuthOperation::Blob, stage, AuthOutcome::Denied);
        }
    }
    {
        let mut state = capture.state.lock().unwrap();
        for c in &mut state.operations {
            c.attempts = u64::MAX;
            c.admitted = u64::MAX;
            c.rejected_capacity = u64::MAX;
            c.rejected_closed = u64::MAX;
            c.before_spawn_failed = u64::MAX;
            c.completed = u64::MAX;
            c.downstream_overloaded = u64::MAX;
            c.denied = u64::MAX;
            c.panicked = u64::MAX;
            c.released = u64::MAX;
            c.outstanding = u64::MAX;
            c.maximum_outstanding = u64::MAX;
        }
        for c in &mut state.stages {
            c.succeeded = u64::MAX;
            c.denied = u64::MAX;
            c.overloaded = u64::MAX;
            c.closed = u64::MAX;
            c.panicked = u64::MAX;
            c.other = u64::MAX;
            c.first_failure_micros = Some(u64::MAX);
            c.first_failure_outcome = Some(AuthOutcome::Denied);
        }
        for f in state.failures.iter_mut().flatten() {
            f.sequence = u64::MAX;
        }
        state.omitted_failures = u64::MAX;
    }
    let snapshot = capture.snapshot().unwrap();
    assert_eq!(snapshot.schema_version, 2);
    assert_eq!(snapshot.stage_labels.len(), 19);
    assert_eq!(snapshot.failures.len(), 32);
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema":"rom-application-load-authentication-v2",
        "scope":"fixed-category Host lifecycle attribution; not per-request evidence",
        "observer_enabled":true,"lifecycle_result":"failed","snapshot":snapshot,
    }))
    .unwrap();
    assert!(bytes.len() <= 256 * 1024);
}
