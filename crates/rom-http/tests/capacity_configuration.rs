use rom::{Error, Runtime};
use rom_http::{Http, Limits};
use rom_sqlite::Sqlite;
use std::{panic::AssertUnwindSafe, sync::Arc};
use tokio::sync::Semaphore;

#[tokio::test]
async fn http_body_capacity_is_fallible_at_engine_boundaries() {
    let runtime = Runtime::builder()
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    for bodies in [0, Semaphore::MAX_PERMITS + 1, usize::MAX] {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Http::new(
                runtime.clone(),
                Arc::new(|_| Err(Error::Denied)),
                Limits {
                    bodies,
                    ..Limits::default()
                },
            )
        }));
        assert!(result.is_ok(), "bodies={bodies} unwound");
        assert!(matches!(result.unwrap(), Err(Error::TooLarge)));
    }
    let http = Http::new(
        runtime,
        Arc::new(|_| Err(Error::Denied)),
        Limits {
            bodies: Semaphore::MAX_PERMITS,
            ..Limits::default()
        },
    )
    .unwrap();
    http.shutdown().await.unwrap();
}
