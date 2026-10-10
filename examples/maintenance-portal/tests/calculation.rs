use rom::{Error, Limits, Runtime, Storage};
use std::{sync::Arc, time::Duration};

fn stores() -> (tempfile::TempDir, Vec<Arc<dyn Storage>>) {
    let directory = tempfile::tempdir().unwrap();
    let stores: Vec<Arc<dyn Storage>> = vec![
        Arc::new(rom_sqlite::Sqlite::open(directory.path().join("calculation.sqlite")).unwrap()),
        Arc::new(rom_redb::Redb::open(directory.path().join("calculation.redb")).unwrap()),
    ];
    (directory, stores)
}

async fn cancellation_journey(store: Arc<dyn Storage>, abort: bool) {
    let pool = Runtime::shared_cpu_pool(1).unwrap();
    let expected_thread = pool.install(|| std::thread::current().id());
    let runtime = Runtime::builder()
        .limits(Limits {
            io_jobs: 1,
            ..Limits::default()
        })
        .build(store, pool)
        .unwrap();
    let (started, beginning) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = runtime.clone();
    let mut task = tokio::spawn(async move {
        worker
            .calculate(move || {
                assert_eq!(std::thread::current().id(), expected_thread);
                started.send(()).unwrap();
                released.recv_timeout(Duration::from_secs(3)).unwrap();
                Ok(42_u64)
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(1), beginning)
        .await
        .unwrap()
        .unwrap();
    if abort {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    } else {
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut task)
                .await
                .is_err()
        );
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
    assert_eq!(runtime.status().unwrap().owned_work, 1);
    assert_eq!(runtime.status().unwrap().available_io_permits, 0);
    assert_eq!(
        runtime.calculate(|| Ok(7_u64)).await,
        Err(Error::Overloaded)
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), runtime.shutdown())
            .await
            .is_err()
    );
    assert_eq!(runtime.status().unwrap().owned_work, 1);
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(1), runtime.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(runtime.status().unwrap().owned_work, 0);
    assert_eq!(runtime.calculate(|| Ok(7_u64)).await, Err(Error::Closed));
}

#[tokio::test]
async fn cancelled_calculation_retains_capacity_and_shutdown_ownership() {
    let (_directory, stores) = stores();
    for store in stores {
        cancellation_journey(store, true).await;
    }
}

#[tokio::test]
async fn timed_out_calculation_retains_capacity_and_shutdown_ownership() {
    let (_directory, stores) = stores();
    for store in stores {
        cancellation_journey(store, false).await;
    }
}

#[tokio::test]
async fn calculation_panic_is_sanitized_and_does_not_close_runtime() {
    let (_directory, stores) = stores();
    for store in stores {
        let runtime = Runtime::builder()
            .build(store, Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
        assert_eq!(
            runtime
                .calculate::<u64, _>(|| panic!("private calculation detail"))
                .await,
            Err(Error::Panicked)
        );
        assert!(runtime.status().unwrap().is_ready());
        assert_eq!(runtime.calculate(|| Ok(7_u64)).await.unwrap(), 7);
        runtime.shutdown().await.unwrap();
    }
}
