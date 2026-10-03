use super::lifecycle::Supervisor;
use std::sync::Arc;

#[tokio::test]
async fn cancelled_caller_keeps_admission_until_owned_work_finishes() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let worker = supervisor.clone();
    let caller = tokio::spawn(async move {
        worker
            .run(async move {
                let _ = entered.send(());
                let _ = released.await;
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    caller.abort();
    assert_eq!(
        supervisor.run(async { Ok(()) }).await,
        Err(rom::Error::Overloaded)
    );
    release.send(()).unwrap();
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn cancelled_drain_waiter_cannot_abandon_owned_authentication_work() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let worker = supervisor.clone();
    let caller = tokio::spawn(async move {
        worker
            .run(async move {
                let _ = entered.send(());
                let _ = released.await;
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    let worker = supervisor.clone();
    let first = tokio::spawn(async move { worker.drain().await });
    tokio::task::yield_now().await;
    first.abort();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), supervisor.drain())
            .await
            .is_err()
    );
    release.send(()).unwrap();
    supervisor.drain().await.unwrap();
    caller.await.unwrap().unwrap();
}
