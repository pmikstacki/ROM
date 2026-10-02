use rom::{Error, Limits, Runtime};
use rom_sqlite::Sqlite;
use std::{panic::AssertUnwindSafe, sync::Arc};
use tokio::sync::Semaphore;

#[test]
fn invalid_concurrency_returns_error_without_unwinding() {
    let pool = Runtime::shared_cpu_pool(1).unwrap();
    let mut unwound = Vec::new();
    for capacity in [0, Semaphore::MAX_PERMITS + 1, usize::MAX] {
        for setting in ["actions", "io_jobs", "subscriptions"] {
            let mut limits = Limits::default();
            match setting {
                "actions" => limits.actions = capacity,
                "io_jobs" => limits.io_jobs = capacity,
                _ => limits.subscriptions = capacity,
            }
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                Runtime::builder()
                    .limits(limits)
                    .build(Arc::new(Sqlite::open(":memory:").unwrap()), pool.clone())
            }));
            match result {
                Ok(result) => assert!(matches!(result, Err(Error::Unsupported(_)))),
                Err(_) => unwound.push((setting, capacity)),
            }
        }
    }
    assert!(unwound.is_empty(), "configurations unwound: {unwound:?}");
}

#[tokio::test]
async fn maximum_concurrency_is_valid_and_reported() {
    let runtime = Runtime::builder()
        .limits(Limits {
            actions: Semaphore::MAX_PERMITS,
            io_jobs: Semaphore::MAX_PERMITS,
            subscriptions: Semaphore::MAX_PERMITS,
            ..Limits::default()
        })
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let status = runtime.status().unwrap();
    assert_eq!(status.available_action_permits, Semaphore::MAX_PERMITS);
    assert_eq!(status.available_io_permits, Semaphore::MAX_PERMITS);
    assert_eq!(
        status.available_subscription_permits,
        Semaphore::MAX_PERMITS
    );
    assert!(status.is_ready());
    runtime.shutdown().await.unwrap();
}
