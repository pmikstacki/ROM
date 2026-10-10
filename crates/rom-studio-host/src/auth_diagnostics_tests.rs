//! The Host snapshot distinguishes rejected admission from accepted work failure.
use crate::{AuthOperation, HostConfig, StudioHost};
use rom::{Actor, Error, Runtime};
use std::sync::Arc;

async fn host() -> StudioHost {
    let runtime = Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let mut config = HostConfig::new(
        "https://studio.example",
        "/rom-studio/",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
        Actor::trusted("fixture", "bootstrap"),
    )
    .authentication_diagnostics(true);
    config.limits.authentication_jobs = 1;
    StudioHost::new(runtime, config).unwrap()
}

async fn released(host: &StudioHost, count: u64) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if host
                .authentication_diagnostics()
                .unwrap()
                .unwrap()
                .operation(AuthOperation::GenericHttpResolver)
                .released
                == count
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn host_admission_rejection_is_distinct_from_admitted_overload() {
    let host = host().await;
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released_work) = tokio::sync::oneshot::channel();
    let owned = host.clone();
    let accepted = tokio::spawn(async move {
        owned
            .shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async move {
                entered.send(()).unwrap();
                released_work.await.unwrap();
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    assert_eq!(
        host.shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async { Ok(()) })
            .await,
        Err(Error::Overloaded)
    );
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let counts = snapshot.operation(AuthOperation::GenericHttpResolver);
    assert_eq!(counts.attempts, 2);
    assert_eq!(counts.admitted, 1);
    assert_eq!(counts.rejected_capacity, 1);
    assert_eq!(counts.downstream_overloaded, 0);
    assert_eq!(counts.outstanding, 1);
    assert_eq!(counts.released, 0);
    release.send(()).unwrap();
    accepted.await.unwrap().unwrap();
    released(&host, 1).await;
    assert_eq!(
        host.shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async {
                Err::<(), _>(Error::Overloaded)
            })
            .await,
        Err(Error::Overloaded)
    );
    released(&host, 2).await;
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let counts = snapshot.operation(AuthOperation::GenericHttpResolver);
    assert_eq!(counts.attempts, 3);
    assert_eq!(counts.admitted, 2);
    assert_eq!(counts.rejected_capacity, 1);
    assert_eq!(counts.downstream_overloaded, 1);
    assert_eq!(counts.completed, 2);
    assert_eq!(counts.released, 2);
    assert_eq!(counts.outstanding, 0);
    assert_eq!(counts.maximum_outstanding, 1);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn disabled_capture_returns_none_without_changing_work_result() {
    let runtime = Runtime::builder()
        .build(
            Arc::new(rom_sqlite::Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let host = StudioHost::new(
        runtime,
        HostConfig::new(
            "https://studio.example",
            "/rom-studio/",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets"),
            Actor::trusted("fixture", "bootstrap"),
        ),
    )
    .unwrap();
    assert!(host.authentication_diagnostics().unwrap().is_none());
    assert_eq!(
        host.shared
            .auth
            .run_tagged(AuthOperation::Session, async { Ok(7_u32) })
            .await,
        Ok(7)
    );
    assert!(host.authentication_diagnostics().unwrap().is_none());
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_waiter_does_not_release_accepted_work_or_its_accounting() {
    let host = host().await;
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released_work) = tokio::sync::oneshot::channel();
    let owned = host.clone();
    let waiter = tokio::spawn(async move {
        owned
            .shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async {
                entered.send(()).unwrap();
                released_work.await.unwrap();
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let counts = snapshot.operation(AuthOperation::GenericHttpResolver);
    assert_eq!(counts.outstanding, 1);
    assert_eq!(counts.completed, 0);
    assert_eq!(counts.released, 0);
    assert_eq!(
        host.shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async { Ok(()) })
            .await,
        Err(Error::Overloaded)
    );
    release.send(()).unwrap();
    host.shutdown().await.unwrap();
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let counts = snapshot.operation(AuthOperation::GenericHttpResolver);
    assert_eq!(counts.completed, 1);
    assert_eq!(counts.released, 1);
    assert_eq!(counts.outstanding, 0);
}

#[tokio::test]
async fn accepted_panic_releases_capacity_and_still_reports_panicked_shutdown() {
    let host = host().await;
    assert_eq!(
        host.shared
            .auth
            .run_tagged::<(), _>(AuthOperation::GenericHttpResolver, async {
                panic!("fixed authentication panic control");
            })
            .await,
        Err(Error::Panicked)
    );
    released(&host, 1).await;
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    let counts = snapshot.operation(AuthOperation::GenericHttpResolver);
    assert_eq!(counts.completed, 1);
    assert_eq!(counts.panicked, 1);
    assert_eq!(counts.released, 1);
    assert_eq!(counts.outstanding, 0);
    assert_eq!(snapshot.failures.len(), 1);
    assert_eq!(snapshot.failures[0].outcome, crate::AuthOutcome::Panicked);
    assert_eq!(host.shutdown().await, Err(Error::Panicked));
}

#[tokio::test]
async fn earliest_failure_capacity_is_fixed_and_later_failures_are_counted() {
    let host = host().await;
    for count in 1..=33 {
        assert_eq!(
            host.shared
                .auth
                .run_tagged::<(), _>(AuthOperation::GenericHttpResolver, async {
                    Err(Error::Denied)
                })
                .await,
            Err(Error::Denied)
        );
        released(&host, count).await;
    }
    let snapshot = host.authentication_diagnostics().unwrap().unwrap();
    assert_eq!(snapshot.failures.len(), 32);
    assert_eq!(snapshot.failures[0].sequence, 1);
    assert_eq!(snapshot.failures[31].sequence, 32);
    assert!(
        snapshot
            .failures
            .iter()
            .all(|f| f.outcome == crate::AuthOutcome::Denied)
    );
    assert_eq!(snapshot.omitted_failures, 1);
    assert!(!snapshot.overflowed);
    assert_eq!(
        snapshot
            .operation(AuthOperation::GenericHttpResolver)
            .denied,
        33
    );
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn completed_panic_remains_sticky_after_a_later_accepted_run() {
    let host = host().await;
    assert_eq!(
        host.shared
            .auth
            .run_tagged::<(), _>(AuthOperation::GenericHttpResolver, async {
                panic!("fixed sticky authentication panic control");
            })
            .await,
        Err(Error::Panicked)
    );
    released(&host, 1).await;
    assert_eq!(
        host.shared
            .auth
            .run_tagged(AuthOperation::GenericHttpResolver, async { Ok(()) })
            .await,
        Ok(())
    );
    released(&host, 2).await;
    assert_eq!(host.shutdown().await, Err(Error::Panicked));
}
