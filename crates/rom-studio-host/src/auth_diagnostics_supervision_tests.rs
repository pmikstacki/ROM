//! Failure between acquiring capacity and spawning work is not a semaphore rejection.
use super::*;

#[tokio::test]
async fn poisoned_job_ownership_reports_before_spawn_without_accepted_or_capacity_counts() {
    let capture = Capture::new();
    let supervisor = Arc::new(Supervisor::observed(1, Some(capture.clone())));
    let poisoned = supervisor.clone();
    assert!(
        std::thread::spawn(move || {
            let _held = poisoned.jobs.lock().unwrap();
            panic!("fixed job ownership poison control");
        })
        .join()
        .is_err()
    );
    assert_eq!(
        supervisor
            .run_tagged(AuthOperation::Session, async { Ok(()) })
            .await,
        Err(Error::Panicked)
    );
    let snapshot = capture.snapshot().unwrap();
    let operation = snapshot.operation(AuthOperation::Session);
    assert_eq!(operation.attempts, 1);
    assert_eq!(operation.admitted, 0);
    assert_eq!(operation.rejected_capacity, 0);
    assert_eq!(operation.outstanding, 0);
    assert_eq!(snapshot.stage(AuthStage::BeforeSpawn).panicked, 1);
    assert_eq!(supervisor.admission.available_permits(), 1);
    assert_eq!(supervisor.drain().await, Err(Error::Panicked));
}
