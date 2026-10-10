//! Readiness is actual child entry, independent from its observer's setup bound.
use super::{
    FIXTURE_CLEANUP, FIXTURE_STARTUP, ReadBlock,
    readiness::{StartupError, Worker},
};
use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Notify, oneshot};

#[tokio::test]
async fn physical_entry_signal_sent_before_observation_is_retained() {
    let block = Arc::new(ReadBlock::new());
    let entered = block.take_entered();
    let child_block = block.clone();
    let mut worker = Worker::new(
        block.clone(),
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || child_block.wait())
                .await
                .unwrap();
            Ok(())
        }),
    );
    // Receive the physical signal before constructing the supervisor's observer.
    let physical_entry = tokio::time::timeout(FIXTURE_STARTUP, entered)
        .await
        .unwrap()
        .unwrap();
    let (send, receive) = oneshot::channel();
    send.send(physical_entry).unwrap();
    assert_eq!(
        worker.ready(receive, FIXTURE_STARTUP).await.unwrap(),
        physical_entry
    );
    block.release();
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
}

#[tokio::test]
async fn worker_error_before_entry_is_reported_without_waiting_for_setup_timeout() {
    let block = Arc::new(ReadBlock::new());
    let entered = block.take_entered();
    let mut worker = Worker::new(block, tokio::spawn(async { Err(rom::Error::Denied) }));
    let error = worker.ready(entered, FIXTURE_STARTUP).await.unwrap_err();
    assert_eq!(error, StartupError::Worker(rom::Error::Denied));
    assert_eq!(worker.finish(FIXTURE_CLEANUP).await.unwrap_err(), error);
    assert_eq!(worker.terminal().unwrap().outcome, Err(error));
    assert!(!worker.pending());
}

#[tokio::test]
async fn completed_worker_without_child_entry_is_not_readiness() {
    let block = Arc::new(ReadBlock::new());
    let entered = block.take_entered();
    let mut worker = Worker::new(block, tokio::spawn(async { Ok(()) }));
    assert_eq!(
        worker.ready(entered, FIXTURE_STARTUP).await.unwrap_err(),
        StartupError::ExitedBeforeEntry
    );
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
    assert!(worker.terminal().unwrap().outcome.is_ok());
}

#[tokio::test]
async fn worker_panic_and_readiness_channel_closure_are_distinct() {
    let block = Arc::new(ReadBlock::new());
    let entered = block.take_entered();
    let mut worker = Worker::new(
        block,
        tokio::spawn(async { panic!("fixture worker panic") }),
    );
    let error = worker.ready(entered, FIXTURE_STARTUP).await.unwrap_err();
    assert!(matches!(&error, StartupError::Panicked(_)));
    assert_eq!(worker.finish(FIXTURE_CLEANUP).await.unwrap_err(), error);
    assert_eq!(worker.terminal().unwrap().outcome, Err(error));
    assert!(!worker.pending());

    let block = Arc::new(ReadBlock::new());
    let release = Arc::new(Notify::new());
    let observed = release.clone();
    let mut worker = Worker::new(
        block,
        tokio::spawn(async move {
            observed.notified().await;
            Ok(())
        }),
    );
    let (send, receive) = oneshot::channel::<Instant>();
    drop(send);
    assert_eq!(
        worker.ready(receive, FIXTURE_STARTUP).await.unwrap_err(),
        StartupError::SignalClosed
    );
    release.notify_one();
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
}

#[tokio::test]
async fn expired_setup_wait_retains_worker_for_physical_release_and_join() {
    let block = Arc::new(ReadBlock::new());
    let physical_entry = block.take_entered();
    let child_block = block.clone();
    let mut worker = Worker::new(
        block.clone(),
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || child_block.wait())
                .await
                .unwrap();
            Ok(())
        }),
    );
    tokio::time::timeout(FIXTURE_STARTUP, physical_entry)
        .await
        .unwrap()
        .unwrap();
    let (_send, never_entered) = oneshot::channel();
    assert_eq!(
        worker
            .ready(never_entered, Duration::ZERO)
            .await
            .unwrap_err(),
        StartupError::SetupTimeout
    );
    block.release();
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
}

#[tokio::test]
async fn cancelled_readiness_observer_retains_worker_for_cleanup() {
    let block = Arc::new(ReadBlock::new());
    let physical_entry = block.take_entered();
    let child_block = block.clone();
    let mut worker = Worker::new(
        block.clone(),
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || child_block.wait())
                .await
                .unwrap();
            Ok(())
        }),
    );
    tokio::time::timeout(FIXTURE_STARTUP, physical_entry)
        .await
        .unwrap()
        .unwrap();
    let (_send, receive) = oneshot::channel();
    {
        let ready = worker.ready(receive, FIXTURE_STARTUP);
        tokio::pin!(ready);
        std::future::poll_fn(|cx| {
            assert!(ready.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
    }
    block.release();
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
}

#[tokio::test]
async fn dropping_fixture_owner_releases_its_already_started_physical_child() {
    let block = Arc::new(ReadBlock::new());
    let physical_entry = block.take_entered();
    let child_block = block.clone();
    let (done, finished) = oneshot::channel();
    let worker = Worker::new(
        block,
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || child_block.wait())
                .await
                .unwrap();
            done.send(()).unwrap();
            Ok(())
        }),
    );
    tokio::time::timeout(FIXTURE_STARTUP, physical_entry)
        .await
        .unwrap()
        .unwrap();
    drop(worker);
    tokio::time::timeout(FIXTURE_CLEANUP, finished)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn assertion_panic_cleanup_retains_consumed_worker_error() {
    let block = Arc::new(ReadBlock::new());
    let mut worker = Worker::new(
        block.clone(),
        tokio::spawn(async { Err(rom::Error::Denied) }),
    );
    let assertion = super::capture_assertions(async {
        worker.finish(FIXTURE_CLEANUP).await.unwrap();
    })
    .await;
    assert!(assertion.is_err());
    block.release();
    assert_eq!(
        worker.finish(FIXTURE_CLEANUP).await.unwrap_err(),
        StartupError::Worker(rom::Error::Denied)
    );
    assert_eq!(
        worker.terminal().unwrap().outcome,
        Err(StartupError::Worker(rom::Error::Denied))
    );
    assert!(!worker.pending());
}

#[tokio::test]
async fn completion_timeout_records_pending_instead_of_successful_worker_terminal() {
    let block = Arc::new(ReadBlock::new());
    let release = Arc::new(Notify::new());
    let observed = release.clone();
    let mut worker = Worker::new(
        block,
        tokio::spawn(async move {
            observed.notified().await;
            Ok(())
        }),
    );
    assert_eq!(
        worker.finish(Duration::ZERO).await.unwrap_err(),
        StartupError::CompletionTimeout
    );
    assert!(worker.pending());
    assert!(worker.terminal().is_none());
    release.notify_one();
    worker.finish(FIXTURE_CLEANUP).await.unwrap();
    assert!(!worker.pending());
    assert!(worker.terminal().unwrap().outcome.is_ok());
}
