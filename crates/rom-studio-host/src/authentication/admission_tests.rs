//! Queue admission must preserve the shared physical capacity and identity result contract.
use crate::{AuthOperation, auth_diagnostics::Capture, lifecycle::Supervisor};
use futures_util::FutureExt;
use rom::{Error, Result};
use std::{future::Future, sync::Arc, time::Duration};

// The dedicated queued path preserves the existing fail-fast API and its overflow contract.
async fn queued<T, F>(supervisor: &Supervisor, operation: AuthOperation, work: F) -> Result<T>
where
    T: Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
{
    supervisor
        .run_queued_tagged(operation, Duration::from_secs(5), work)
        .await
}

async fn blocked(
    supervisor: &Arc<Supervisor>,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<()>>,
) {
    blocked_operation(supervisor, AuthOperation::ProtectedMiddleware).await
}

async fn blocked_operation(
    supervisor: &Arc<Supervisor>,
    operation: AuthOperation,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let worker = supervisor.clone();
    let task = tokio::spawn(async move {
        worker
            .run_tagged(operation, async move {
                entered.send(()).unwrap();
                released.await.unwrap();
                Ok(())
            })
            .await
    });
    ready.await.unwrap();
    (release, task)
}

async fn full_mixed_profile(stream_first: bool) {
    let capture = Capture::new();
    let supervisor = Arc::new(Supervisor::observed(8, Some(capture.clone())));
    let active_streams = if stream_first { 5 } else { 0 };
    let active_requests = 8 - active_streams;
    let mut active = Vec::new();
    for _ in 0..active_streams {
        active.push(blocked_operation(&supervisor, AuthOperation::CurrentStream).await);
    }
    for _ in 0..active_requests {
        active.push(blocked(&supervisor).await);
    }
    let operations = std::iter::repeat_n(AuthOperation::ProtectedMiddleware, 32 - active_requests)
        .chain(std::iter::repeat_n(AuthOperation::Session, 4))
        .chain(std::iter::repeat_n(
            AuthOperation::CurrentStream,
            5 - active_streams,
        ));
    let mut waiters: Vec<_> = operations
        .map(|operation| Box::pin(queued(&supervisor, operation, async { Ok(()) })))
        .collect();
    // Each actual future registers once while the eight physical jobs remain blocked.
    // Capture rejections instead of panicking before releasing the owned callbacks.
    let premature: Vec<_> = waiters
        .iter_mut()
        .map(|waiter| waiter.as_mut().now_or_never())
        .collect();
    let counts = supervisor.queued_counts();
    let pending = capture.snapshot().unwrap();
    let outstanding: u64 = pending.operations.iter().map(|row| row.outstanding).sum();
    let mut active_tasks = Vec::new();
    for (release, task) in active {
        release.send(()).unwrap();
        active_tasks.push(task);
    }
    let mut final_results = Vec::new();
    for (waiter, early) in waiters.into_iter().zip(premature.iter()) {
        final_results.push(match early {
            Some(result) => result.clone(),
            None => waiter.await,
        });
    }
    for task in active_tasks {
        task.await.unwrap().unwrap();
    }
    supervisor.drain().await.unwrap();
    let final_snapshot = capture.snapshot().unwrap();
    assert_eq!(
        outstanding, 8,
        "only eight physical callbacks may be accepted while blocked"
    );
    assert!(
        premature.iter().all(Option::is_none),
        "all32 HTTP callers plus4 Session reacquisitions and5 streams must fit bounded waiting positions in either arrival order"
    );
    assert_eq!(counts, (32 - active_requests + 4, 5 - active_streams));
    assert!(final_results.iter().all(Result::is_ok));
    for (operation, expected) in [
        (AuthOperation::ProtectedMiddleware, 32),
        (AuthOperation::Session, 4),
        (AuthOperation::CurrentStream, 5),
    ] {
        let row = final_snapshot.operation(operation);
        assert_eq!(row.attempts, expected);
        assert_eq!(row.admitted, expected);
        assert_eq!(row.completed, expected);
        assert_eq!(row.released, expected);
        assert_eq!(row.rejected_capacity, 0);
        assert_eq!(row.outstanding, 0);
    }
    assert_eq!(supervisor.queued_counts(), (0, 0));
}

#[tokio::test]
async fn actual_thirty_two_request_profile_with_session_reacquisitions_request_first() {
    full_mixed_profile(false).await;
}

#[tokio::test]
async fn actual_thirty_two_request_profile_with_session_reacquisitions_stream_first() {
    full_mixed_profile(true).await;
}

#[tokio::test]
async fn stream_first_active_checks_do_not_overload_sixteen_request_callers() {
    let capture = Capture::new();
    let supervisor = Arc::new(Supervisor::observed(8, Some(capture.clone())));
    let mut active = Vec::new();
    for _ in 0..4 {
        active.push(blocked_operation(&supervisor, AuthOperation::CurrentStream).await);
    }
    for _ in 0..4 {
        active.push(blocked(&supervisor).await);
    }
    let mut requests: Vec<_> = (0..12)
        .map(|_| {
            Box::pin(queued(
                &supervisor,
                AuthOperation::ProtectedMiddleware,
                async { Ok(()) },
            ))
        })
        .collect();
    for request in &mut requests {
        assert!(
            request.as_mut().now_or_never().is_none(),
            "four active streams must not make a sixteen-request profile overload"
        );
    }
    assert_eq!(supervisor.queued_counts(), (12, 0));
    let pending = capture.snapshot().unwrap();
    assert_eq!(
        pending
            .operation(AuthOperation::ProtectedMiddleware)
            .admitted,
        4
    );
    assert_eq!(pending.operation(AuthOperation::CurrentStream).admitted, 4);
    let mut stream = Box::pin(queued(&supervisor, AuthOperation::CurrentStream, async {
        Ok(())
    }));
    assert!(stream.as_mut().now_or_never().is_none());
    assert_eq!(supervisor.queued_counts(), (12, 1));
    let mut tasks = Vec::new();
    for (release, task) in active {
        release.send(()).unwrap();
        tasks.push(task);
    }
    for result in futures_util::future::join_all(requests).await {
        result.unwrap();
    }
    stream.await.unwrap();
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    supervisor.drain().await.unwrap();
    let snapshot = capture.snapshot().unwrap();
    let requests = snapshot.operation(AuthOperation::ProtectedMiddleware);
    assert_eq!(requests.admitted, 16);
    assert_eq!(requests.rejected_capacity, 0);
    assert_eq!(requests.completed, requests.admitted);
    assert_eq!(requests.released, requests.admitted);
    assert_eq!(supervisor.queued_counts(), (0, 0));
}

#[tokio::test]
async fn forty_waiting_requests_keep_a_finite_typed_overflow_boundary() {
    let supervisor = Arc::new(Supervisor::new(8));
    let mut active = Vec::new();
    for _ in 0..8 {
        active.push(blocked(&supervisor).await);
    }
    let mut requests: Vec<_> = (0..40)
        .map(|_| {
            Box::pin(queued(
                &supervisor,
                AuthOperation::ProtectedMiddleware,
                async { Ok(()) },
            ))
        })
        .collect();
    for request in &mut requests {
        assert!(request.as_mut().now_or_never().is_none());
    }
    assert_eq!(supervisor.queued_counts(), (40, 0));
    assert_eq!(
        queued(&supervisor, AuthOperation::ProtectedMiddleware, async {
            Ok(())
        })
        .await,
        Err(Error::Overloaded)
    );
    supervisor.close();
    for request in requests {
        assert_eq!(request.await, Err(Error::Closed));
    }
    for (release, task) in active {
        release.send(()).unwrap();
        task.await.unwrap().unwrap();
    }
    supervisor.drain().await.unwrap();
    assert_eq!(supervisor.queued_counts(), (0, 0));
}

#[tokio::test]
async fn sixteen_request_callers_and_four_stream_checks_share_only_eight_active_jobs() {
    let capture = Capture::new();
    let supervisor = Arc::new(Supervisor::observed(8, Some(capture.clone())));
    let mut active = Vec::new();
    for _ in 0..8 {
        active.push(blocked(&supervisor).await);
    }
    let mut requests: Vec<_> = (0..8)
        .map(|_| {
            Box::pin(queued(
                &supervisor,
                AuthOperation::ProtectedMiddleware,
                async { Ok(()) },
            ))
        })
        .collect();
    for request in &mut requests {
        assert!(
            request.as_mut().now_or_never().is_none(),
            "request burst must wait instead of immediate overload"
        );
    }
    assert_eq!(supervisor.queued_counts(), (8, 0));
    let mut streams: Vec<_> = (0..4)
        .map(|_| {
            Box::pin(queued(&supervisor, AuthOperation::CurrentStream, async {
                Ok(())
            }))
        })
        .collect();
    for stream in &mut streams {
        assert!(
            stream.as_mut().now_or_never().is_none(),
            "request saturation must leave bounded stream waiting positions"
        );
    }
    assert_eq!(supervisor.queued_counts(), (8, 4));
    let pending = capture.snapshot().unwrap();
    assert_eq!(
        pending
            .operation(AuthOperation::ProtectedMiddleware)
            .admitted,
        8
    );
    assert_eq!(
        pending
            .operation(AuthOperation::ProtectedMiddleware)
            .outstanding,
        8
    );
    assert_eq!(pending.operation(AuthOperation::CurrentStream).admitted, 0);
    let mut tasks = Vec::new();
    for (release, task) in active {
        release.send(()).unwrap();
        tasks.push(task);
    }
    for outcome in futures_util::future::join_all(requests).await {
        outcome.unwrap();
    }
    for outcome in futures_util::future::join_all(streams).await {
        outcome.unwrap();
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    supervisor.drain().await.unwrap();
    let snapshot = capture.snapshot().unwrap();
    let admitted: u64 = snapshot.operations.iter().map(|row| row.admitted).sum();
    let released: u64 = snapshot.operations.iter().map(|row| row.released).sum();
    assert_eq!(admitted, 20);
    assert_eq!(released, admitted);
    assert!(snapshot.operations.iter().all(|row| row.outstanding == 0));
    assert_eq!(supervisor.queued_counts(), (0, 0));
}

#[tokio::test]
async fn later_requests_cannot_take_the_only_slot_a_registered_stream_is_waiting_for() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (release, active) = blocked(&supervisor).await;
    let mut stream = Box::pin(queued(&supervisor, AuthOperation::CurrentStream, async {
        Ok(7)
    }));
    let mut later = Box::pin(queued(
        &supervisor,
        AuthOperation::ProtectedMiddleware,
        async { Ok(9) },
    ));
    assert!(stream.as_mut().now_or_never().is_none());
    assert!(later.as_mut().now_or_never().is_none());
    release.send(()).unwrap();
    assert_eq!(stream.await, Ok(7));
    assert_eq!(later.await, Ok(9));
    active.await.unwrap().unwrap();
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn cancelled_waiting_caller_releases_queue_position_without_spawning_work() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (release, active) = blocked(&supervisor).await;
    let (ran, work_ran) = tokio::sync::oneshot::channel();
    let mut waiting = Box::pin(queued(
        &supervisor,
        AuthOperation::ProtectedMiddleware,
        async move {
            let _ = ran.send(());
            Ok(())
        },
    ));
    assert!(waiting.as_mut().now_or_never().is_none());
    let mut others: Vec<_> = (0..4)
        .map(|_| {
            Box::pin(queued(
                &supervisor,
                AuthOperation::ProtectedMiddleware,
                async { Ok(()) },
            ))
        })
        .collect();
    for other in &mut others {
        assert!(other.as_mut().now_or_never().is_none());
    }
    assert_eq!(supervisor.queued_counts(), (5, 0));
    assert_eq!(
        queued(&supervisor, AuthOperation::ProtectedMiddleware, async {
            Ok(())
        })
        .await,
        Err(Error::Overloaded)
    );
    drop(waiting);
    assert_eq!(supervisor.queued_counts(), (4, 0));
    assert!(work_ran.await.is_err());
    let mut replacement = Box::pin(queued(
        &supervisor,
        AuthOperation::ProtectedMiddleware,
        async { Ok(()) },
    ));
    assert!(replacement.as_mut().now_or_never().is_none());
    assert_eq!(supervisor.queued_counts(), (5, 0));
    supervisor.close();
    for other in others {
        assert_eq!(other.await, Err(Error::Closed));
    }
    assert_eq!(replacement.await, Err(Error::Closed));
    release.send(()).unwrap();
    active.await.unwrap().unwrap();
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn completed_result_cannot_arrive_before_its_physical_capacity_is_released() {
    let supervisor = Supervisor::new(1);
    assert_eq!(
        queued(&supervisor, AuthOperation::Session, async { Ok(1) }).await,
        Ok(1)
    );
    assert_eq!(supervisor.run(async { Ok(2) }).await, Ok(2));
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn queued_authoritative_denial_and_downstream_overload_are_preserved() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (release, active) = blocked(&supervisor).await;
    let mut denial = Box::pin(queued::<(), _>(
        &supervisor,
        AuthOperation::CurrentStream,
        async { Err(Error::Denied) },
    ));
    assert!(denial.as_mut().now_or_never().is_none());
    release.send(()).unwrap();
    assert_eq!(denial.await, Err(Error::Denied));
    active.await.unwrap().unwrap();
    assert_eq!(
        queued::<(), _>(&supervisor, AuthOperation::Session, async {
            Err(Error::Overloaded)
        })
        .await,
        Err(Error::Overloaded)
    );
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn stream_waiting_positions_are_separate_and_finite_and_close_wakes_both_classes() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (release, active) = blocked(&supervisor).await;
    let mut stream = Box::pin(queued(&supervisor, AuthOperation::CurrentStream, async {
        Ok(())
    }));
    assert!(stream.as_mut().now_or_never().is_none());
    assert_eq!(
        queued(&supervisor, AuthOperation::CurrentStream, async { Ok(()) }).await,
        Err(Error::Overloaded)
    );
    let mut requests: Vec<_> = (0..5)
        .map(|_| {
            Box::pin(queued(
                &supervisor,
                AuthOperation::ProtectedMiddleware,
                async { Ok(()) },
            ))
        })
        .collect();
    for request in &mut requests {
        assert!(request.as_mut().now_or_never().is_none());
    }
    assert_eq!(
        queued(&supervisor, AuthOperation::ProtectedMiddleware, async {
            Ok(())
        })
        .await,
        Err(Error::Overloaded)
    );
    assert_eq!(supervisor.queued_counts(), (5, 1));
    supervisor.close();
    assert_eq!(stream.await, Err(Error::Closed));
    for request in requests {
        assert_eq!(request.await, Err(Error::Closed));
    }
    assert_eq!(supervisor.queued_counts(), (0, 0));
    release.send(()).unwrap();
    active.await.unwrap().unwrap();
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn queue_wait_deadline_releases_position_without_polling_authentication_work() {
    let supervisor = Arc::new(Supervisor::new(1));
    let (release, active) = blocked(&supervisor).await;
    let (ran, work_ran) = tokio::sync::oneshot::channel();
    assert_eq!(
        supervisor
            .run_queued_tagged(
                AuthOperation::CurrentStream,
                Duration::from_millis(1),
                async move {
                    let _ = ran.send(());
                    Ok(())
                },
            )
            .await,
        Err(Error::Overloaded)
    );
    assert!(work_ran.await.is_err());
    assert_eq!(supervisor.queued_counts(), (0, 0));
    assert_eq!(
        supervisor.run(async { Ok(()) }).await,
        Err(Error::Overloaded)
    );
    release.send(()).unwrap();
    active.await.unwrap().unwrap();
    supervisor.drain().await.unwrap();
}

#[tokio::test]
async fn cancelled_accepted_queued_caller_keeps_the_physical_permit_until_completion() {
    let capture = Capture::new();
    let supervisor = Arc::new(Supervisor::observed(1, Some(capture.clone())));
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let worker = supervisor.clone();
    let caller = tokio::spawn(async move {
        queued(&worker, AuthOperation::CurrentStream, async move {
            entered.send(()).unwrap();
            released.await.unwrap();
            Ok(())
        })
        .await
    });
    ready.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(
        supervisor.run(async { Ok(()) }).await,
        Err(Error::Overloaded)
    );
    let counts = *capture
        .snapshot()
        .unwrap()
        .operation(AuthOperation::CurrentStream);
    assert_eq!(counts.outstanding, 1);
    assert_eq!(counts.released, 0);
    release.send(()).unwrap();
    supervisor.drain().await.unwrap();
    let counts = *capture
        .snapshot()
        .unwrap()
        .operation(AuthOperation::CurrentStream);
    assert_eq!(counts.admitted, 1);
    assert_eq!(counts.completed, 1);
    assert_eq!(counts.released, 1);
    assert_eq!(counts.outstanding, 0);
}
