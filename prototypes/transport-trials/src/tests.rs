use super::*;
use std::{future::Future, time::Duration};
use tower::{ServiceBuilder, ServiceExt, limit::ConcurrencyLimit, service_fn};

fn core() -> Core {
    Core::new(Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(3)
            .build()
            .unwrap(),
    ))
}
fn attempt(key: u64, expected: u64, value: u64) -> Attempt {
    Attempt {
        context: host_context(1),
        command: Command {
            key,
            expected,
            value,
            claimed_actor: 1,
        },
        gate: None,
        bytes: 64,
    }
}
fn pending<F: Future + Unpin>(future: &mut F) -> bool {
    Pin::new(future)
        .poll(&mut Context::from_waker(std::task::Waker::noop()))
        .is_pending()
}
async fn invoke(core: &Core, via_tower: bool, request: Attempt) -> Outcome {
    if via_tower {
        ServiceBuilder::new()
            .load_shed()
            .concurrency_limit(2)
            .service(TowerInvoke(core.clone()))
            .oneshot(request)
            .await
            .map_err(|e| *e.downcast::<Error>().unwrap())
    } else {
        core.invoke(request)?.await
    }
}

#[tokio::test]
async fn equivalent_semantics_conflict_forgery_and_current_disclosure() {
    for tower in [false, true] {
        let core = core();
        let first = invoke(&core, tower, attempt(1, 0, 7)).await.unwrap();
        assert_eq!(
            first,
            Resource {
                revision: 1,
                value: 7
            }
        );
        assert_eq!(invoke(&core, tower, attempt(1, 0, 7)).await, Ok(first));
        assert_eq!(
            invoke(&core, tower, attempt(1, 0, 8)).await,
            Err(Error::Conflict)
        );
        assert_eq!(
            invoke(&core, tower, attempt(2, 0, 8)).await,
            Err(Error::Conflict)
        );
        let mut forged = attempt(1, 0, 7);
        forged.context = host_context(2); // serialized claimed_actor remains 1
        assert_eq!(invoke(&core, tower, forged).await, Err(Error::Denied));
        let waiting = core.invoke(attempt(1, 0, 7)).unwrap();
        core.revoke();
        assert_eq!(waiting.await, Err(Error::Denied));
        assert_eq!(
            invoke(&core, tower, attempt(1, 0, 7)).await,
            Err(Error::Denied)
        );
        assert_eq!(core.resource(), first);
        core.begin_shutdown();
        core.drain().await;
    }
}

#[tokio::test]
async fn tower_only_negative_control_releases_permit_while_cpu_still_runs() {
    let pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(3)
            .build()
            .unwrap(),
    );
    let service = service_fn(move |gate: Arc<Gate>| {
        let (tx, rx) = oneshot::channel();
        pool.spawn(move || {
            gate.enter();
            let _ = tx.send(());
        });
        async move { rx.await.map_err(|_| Error::Unresolved) }
    });
    let mut service = ConcurrencyLimit::new(service, 1);
    let a = Arc::new(Gate::default());
    let b = Arc::new(Gate::default());
    let first = service.ready().await.unwrap().call(a.clone());
    a.started().await;
    drop(first);
    let mut readiness = Box::pin(service.ready());
    let incorrectly_available = !pending(&mut readiness);
    drop(readiness);
    let second = service.call(b.clone());
    b.started().await;
    // Both CPU gates are still closed even though advertised concurrency is one.
    a.release();
    b.release();
    second.await.unwrap();
    assert!(incorrectly_available);
}

#[tokio::test]
async fn rom_permits_supervision_and_shutdown_survive_tower_cancellation() {
    let core = core();
    let mut service = ConcurrencyLimit::new(TowerInvoke(core.clone()), 1);
    let a = Arc::new(Gate::default());
    let b = Arc::new(Gate::default());
    let mut request = attempt(1, 0, 1);
    request.gate = Some(a.clone());
    let first = service.ready().await.unwrap().call(request);
    a.started().await;
    drop(first);
    let mut request = attempt(2, 0, 2);
    request.gate = Some(b.clone());
    let second = service.ready().await.unwrap().call(request);
    b.started().await;
    drop(second);
    let occupied = core.available();
    let overload = service.ready().await.unwrap().call(attempt(3, 0, 3)).await;
    core.begin_shutdown();
    let closed = core.invoke(attempt(4, 0, 4)).err();
    let mut drain = Box::pin(core.drain());
    let blocked = pending(&mut drain);
    a.release();
    b.release();
    drain.await;
    assert_eq!(occupied, 0);
    assert_eq!(overload, Err(Error::Overloaded));
    assert_eq!(closed, Some(Error::Closed));
    assert!(blocked);
    assert_eq!(core.available(), 2);
    assert_eq!(core.retained_jobs(), 0);
    // Both requested revision zero; exactly one may commit after the gates open.
    assert_eq!(core.resource().revision, 1);
}

#[tokio::test]
async fn direct_cancel_and_inflight_duplicates_retain_one_job() {
    let core = core();
    let gate = Arc::new(Gate::default());
    let mut request = attempt(1, 0, 9);
    request.gate = Some(gate.clone());
    let first = core.invoke(request.clone()).unwrap();
    gate.started().await;
    drop(first);
    let duplicate = core.invoke(request).unwrap();
    let conflict = core.invoke(attempt(1, 0, 10)).err();
    let jobs = core.retained_jobs();
    let capacity = core.available();
    gate.release();
    let receipt = duplicate.await.unwrap();
    assert_eq!(conflict, Some(Error::Conflict));
    assert_eq!(jobs, 1);
    assert_eq!(capacity, 1);
    assert_eq!(
        receipt,
        Resource {
            revision: 1,
            value: 9
        }
    );
    assert_eq!(core.invoke(attempt(1, 0, 9)).unwrap().await, Ok(receipt));
    core.begin_shutdown();
    core.drain().await;
}

#[tokio::test(start_paused = true)]
async fn tower_timeout_is_unresolved_interest_loss_not_action_failure() {
    let core = core();
    let gate = Arc::new(Gate::default());
    let mut service = ServiceBuilder::new()
        .timeout(Duration::from_secs(5))
        .service(TowerInvoke(core.clone()));
    let mut request = attempt(1, 0, 42);
    request.gate = Some(gate.clone());
    let reply = service.ready().await.unwrap().call(request);
    gate.started().await;
    tokio::time::advance(Duration::from_secs(6)).await;
    let timed_out = reply
        .await
        .unwrap_err()
        .is::<tower::timeout::error::Elapsed>();
    let capacity = core.available();
    gate.release();
    let receipt = core.invoke(attempt(1, 0, 42)).unwrap().await.unwrap();
    assert!(timed_out);
    assert_eq!(capacity, 1);
    assert_eq!(receipt.revision, 1);
    core.begin_shutdown();
    core.drain().await;
}

#[tokio::test]
async fn admission_charges_bytes_before_spawn_and_bounds_retained_outcomes() {
    let core = core();
    let gate = Arc::new(Gate::default());
    let mut first = attempt(1, 0, 1);
    first.bytes = 100;
    first.gate = Some(gate.clone());
    let reply = core.invoke(first).unwrap();
    gate.started().await;
    let overload = core.invoke(attempt(2, 0, 2)).err();
    let mut oversized = attempt(3, 0, 3);
    oversized.bytes = 129;
    let too_large = core.invoke(oversized).err();
    let count = core.retained_jobs();
    gate.release();
    reply.await.unwrap();
    assert_eq!(overload, Some(Error::Overloaded));
    assert_eq!(too_large, Some(Error::TooLarge));
    assert_eq!(count, 1);
    for key in 2..=16 {
        core.invoke(attempt(key, key - 1, key))
            .unwrap()
            .await
            .unwrap();
    }
    assert_eq!(
        core.invoke(attempt(17, 16, 17)).err(),
        Some(Error::Overloaded)
    );
    assert_eq!(
        core.invoke(attempt(1, 0, 1))
            .unwrap()
            .await
            .unwrap()
            .revision,
        1
    );
    core.begin_shutdown();
    core.drain().await;
}

#[tokio::test]
async fn readiness_reserves_capacity_and_drop_releases_it_without_call() {
    let inner = service_fn(|_: ()| async { Ok::<_, Error>(()) });
    let mut first = ConcurrencyLimit::new(inner, 1);
    let mut second = first.clone();
    first.ready().await.unwrap();
    let mut waiting = Box::pin(second.ready());
    assert!(pending(&mut waiting));
    drop(waiting);
    drop(first);
    second.ready().await.unwrap().call(()).await.unwrap();
}

#[tokio::test]
async fn layer_order_changes_queued_plus_active_admission() {
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let inner_gate = gate.clone();
    let inner = service_fn(move |_: ()| {
        let gate = inner_gate.clone();
        async move {
            let _p = gate.acquire().await.unwrap();
            Ok::<_, Error>(())
        }
    });
    let mut buffered = ServiceBuilder::new()
        .buffer(1)
        .concurrency_limit(1)
        .service(inner.clone());
    let first = buffered.ready().await.unwrap().call(());
    // Polling the reply lets the Buffer worker forward the first request.
    let mut first = Box::pin(first);
    assert!(pending(&mut first));
    tokio::task::yield_now().await;
    let second = buffered.ready().await.unwrap().call(());
    drop(second);
    drop(first);
    drop(buffered);
    let mut limited = ServiceBuilder::new()
        .concurrency_limit(1)
        .buffer(1)
        .service(inner);
    let first = limited.ready().await.unwrap().call(());
    let mut ready = Box::pin(limited.ready());
    assert!(pending(&mut ready));
    drop(ready);
    drop(first);
    gate.add_permits(4);
}

#[tokio::test]
async fn live_coalesces_journal_overflows_and_old_cursor_reports_gap() {
    let core = core();
    let mut live = core.observe(host_context(1)).unwrap();
    let mut journal = core.subscribe(host_context(1), 0).unwrap();
    assert_eq!(live.next().await.unwrap().revision, 0);
    for key in 1..=3 {
        core.invoke(attempt(key, key - 1, key))
            .unwrap()
            .await
            .unwrap();
    }
    assert_eq!(
        live.next().await.unwrap(),
        Resource {
            revision: 3,
            value: 3
        }
    );
    assert_eq!(journal.queued(), 2);
    assert_eq!(journal.next().await, Err(Error::Lagged));
    drop(journal);
    assert_eq!(core.subscribe(host_context(1), 0).err(), Some(Error::Gap));
    let mut resumed = core.subscribe(host_context(1), 1).unwrap();
    assert_eq!(resumed.next().await.unwrap().revision, 2);
    assert_eq!(resumed.next().await.unwrap().revision, 3);
    core.begin_shutdown();
    assert_eq!(live.next().await, Err(Error::Closed));
    assert_eq!(resumed.next().await, Err(Error::Closed));
    core.drain().await;
}

#[tokio::test]
async fn buffered_streams_recheck_authorization_and_drop_returns_capacity() {
    let core = core();
    let mut live = core.observe(host_context(1)).unwrap();
    let mut journal = core.subscribe(host_context(1), 0).unwrap();
    assert!(matches!(
        core.observe(host_context(1)),
        Err(Error::Overloaded)
    ));
    core.invoke(attempt(1, 0, 9)).unwrap().await.unwrap();
    core.revoke();
    assert_eq!(live.next().await, Err(Error::Denied));
    assert_eq!(journal.next().await, Err(Error::Denied));
    drop(live);
    drop(journal);
    assert_eq!(core.stream_capacity(), 2);
    core.begin_shutdown();
    core.drain().await;
}

#[tokio::test]
async fn tower_response_limit_does_not_limit_open_stream_lifetime() {
    let core = core();
    let captured = core.clone();
    let service = service_fn(move |_: ()| {
        let stream = captured.observe(host_context(1));
        async { stream }
    });
    let mut service = ConcurrencyLimit::new(service, 1);
    let first = service.ready().await.unwrap().call(()).await.unwrap();
    let second = service.ready().await.unwrap().call(()).await.unwrap();
    assert!(matches!(
        service.ready().await.unwrap().call(()).await,
        Err(Error::Overloaded)
    ));
    assert_eq!(core.stream_capacity(), 0);
    drop(first);
    drop(second);
    assert_eq!(core.stream_capacity(), 2);
    core.begin_shutdown();
    core.drain().await;
}

#[test]
fn capability_support_does_not_create_authority_and_missing_profile_fails() {
    let core = core();
    let all = Profile {
        actions: true,
        live: true,
        journal: true,
    };
    assert_eq!(all.require(all), Ok(()));
    assert_eq!(
        Profile {
            actions: true,
            live: false,
            journal: false
        }
        .require(all),
        Err(Error::Unsupported)
    );
    assert!(matches!(core.observe(host_context(2)), Err(Error::Denied)));
    assert!(matches!(
        core.subscribe(host_context(2), 0),
        Err(Error::Denied)
    ));
}

#[tokio::test]
async fn authority_revoked_during_accepted_cpu_work_rejects_commit() {
    let core = core();
    let gate = Arc::new(Gate::default());
    let mut request = attempt(1, 0, 99);
    request.gate = Some(gate.clone());
    let reply = core.invoke(request).unwrap();
    gate.started().await;
    core.revoke();
    gate.release();
    assert_eq!(reply.await, Err(Error::Denied));
    core.begin_shutdown();
    core.drain().await;
    assert_eq!(core.resource().revision, 0);
    assert_eq!(core.available(), 2);
}

#[tokio::test]
async fn tower_load_shed_rejects_while_outer_response_permit_is_held() {
    let core = core();
    let gate = Arc::new(Gate::default());
    let mut service = ServiceBuilder::new()
        .load_shed()
        .concurrency_limit(1)
        .service(TowerInvoke(core.clone()));
    let mut request = attempt(1, 0, 1);
    request.gate = Some(gate.clone());
    let first = service.ready().await.unwrap().call(request);
    gate.started().await;
    let overloaded = service
        .ready()
        .await
        .unwrap()
        .call(attempt(2, 0, 2))
        .await
        .unwrap_err()
        .is::<tower::load_shed::error::Overloaded>();
    let still_unused_core_permit = core.available();
    gate.release();
    first.await.unwrap();
    assert!(overloaded);
    assert_eq!(still_unused_core_permit, 1);
    core.begin_shutdown();
    core.drain().await;
}

#[tokio::test]
async fn stream_terminal_reason_remains_available_on_repeated_next() {
    let core = core();
    let mut live = core.observe(host_context(1)).unwrap();
    live.next().await.unwrap();
    core.begin_shutdown();
    assert_eq!(live.next().await, Err(Error::Closed));
    assert_eq!(live.next().await, Err(Error::Closed));
    core.drain().await;
}
