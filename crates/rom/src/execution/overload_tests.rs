//! Genuine boundary regressions for fixed overload counters.
use super::Runtime;
use super::overload_fixture::*;
use super::overload_generation::{Boundary, GenerationHook};
use crate::*;
use std::collections::BTreeSet;
use std::future::Future;
use std::sync::atomic::Ordering;
use std::task::Poll;

async fn drain(runtime: &Runtime) -> std::result::Result<Result<()>, tokio::time::error::Elapsed> {
    tokio::time::timeout(BOUND, runtime.shutdown()).await
}
fn zero_other_counts(runtime: &Runtime, io: u64, action: u64, observation: u64, actor: u64) {
    let counts = runtime.core_overload_stats();
    assert_eq!(counts.io_no_permits, io);
    assert_eq!(counts.action_no_permits, action);
    assert_eq!(counts.observation_generation_exhausted, observation);
    assert_eq!(counts.actor_generation_exhausted, actor);
    assert!(!counts.overflowed);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn io_32_native_entries_survive_waiter_cancel_and_shutdown_waits() {
    let store = MemoryStore::seeded();
    let runtime = runtime(store.clone());
    let (pause, mut entered) = NativePause::new(32);
    let release_guard = Release(pause.clone());
    let (finished, mut bodies_finished) = tokio::sync::mpsc::channel(32);
    let mut tasks = vec![];
    for index in 0..32 {
        let clone = runtime.clone();
        let pause = pause.clone();
        let finished = finished.clone();
        tasks.push(tokio::spawn(async move {
            clone
                .io(move |rt| {
                    // Before any managed gate/storage lock, independently of Rayon2.
                    let result = pause
                        .wait(index)
                        .and_then(|()| rt.0.storage.load(&key("row-0")));
                    let _ = finished.try_send(index);
                    result
                })
                .await
        }));
    }
    drop(finished);
    let all_entered = tokio::time::timeout(BOUND, async {
        let mut seen = BTreeSet::new();
        for _ in 0..32 {
            if let Some(index) = entered.recv().await {
                seen.insert(index);
            }
        }
        seen
    })
    .await;
    let cancelled = tasks.remove(0);
    cancelled.abort();
    let cancellation = cancelled.await;
    let status_while_blocked = runtime.status();
    let capacity_while_blocked = runtime.available_io_capacity();
    let reads_before_extra = store.reads.load(Ordering::SeqCst);
    let extra = runtime.io(|rt| rt.0.storage.load(&key("row-0"))).await;
    let reads_after_extra = store.reads.load(Ordering::SeqCst);
    let counts_while_blocked = runtime.core_overload_stats();
    let mut shutdown = Box::pin(runtime.shutdown());
    // Actually poll once, unlike is_finished immediately after spawn.
    let pending_before_release =
        std::future::poll_fn(|cx| Poll::Ready(shutdown.as_mut().poll(cx).is_pending())).await;
    pause.release();
    drop(release_guard);
    let body_receipts = tokio::time::timeout(BOUND, async {
        let mut seen = BTreeSet::new();
        for _ in 0..32 {
            if let Some(index) = bodies_finished.recv().await {
                seen.insert(index);
            }
        }
        seen
    })
    .await;
    let mut caller_results = vec![];
    for task in tasks {
        caller_results.push(tokio::time::timeout(BOUND, task).await);
    }
    let terminal = tokio::time::timeout(BOUND, shutdown).await;
    // Only assert after explicit release, receipts, joins and shutdown.
    assert_eq!(all_entered.unwrap().len(), 32);
    assert!(cancellation.unwrap_err().is_cancelled());
    assert_eq!(capacity_while_blocked, 0);
    assert_eq!(status_while_blocked.unwrap().owned_work, 32);
    assert_eq!(extra, Err(Error::Overloaded));
    assert_eq!(reads_after_extra, reads_before_extra);
    assert_eq!(counts_while_blocked.io_no_permits, 1);
    assert!(pending_before_release);
    assert_eq!(body_receipts.unwrap().len(), 32);
    for result in caller_results {
        assert!(result.unwrap().unwrap().unwrap().is_some());
    }
    terminal.unwrap().unwrap();
    assert_eq!(runtime.status().unwrap().owned_work, 0);
    assert_eq!(runtime.status().unwrap().intake, IntakeState::Stopped);
    assert_eq!(runtime.available_io_capacity(), 32);
    zero_other_counts(&runtime, 1, 0, 0, 0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_and_nested_overloaded_do_not_count_acquisition_failure() {
    let runtime = runtime(MemoryStore::seeded());
    let nested = runtime.io::<(), _>(|_| Err(Error::Overloaded)).await;
    let terminal = drain(&runtime).await;
    let closed = runtime.io::<(), _>(|_| Ok(())).await;
    terminal.unwrap().unwrap();
    assert_eq!(nested, Err(Error::Overloaded));
    assert_eq!(closed, Err(Error::Closed));
    zero_other_counts(&runtime, 0, 0, 0, 0);
}

async fn generation_case(boundary: Boundary, invalidations: usize) {
    let store = MemoryStore::seeded();
    let runtime = runtime(store.clone());
    let (hook, mut entered) = GenerationHook::new(boundary);
    *runtime.0.generation_test_hook.lock().unwrap() = Some(hook);
    let clone = runtime.clone();
    let pending = tokio::spawn(async move {
        match boundary {
            Boundary::Observation => {
                clone
                    .observe(&actor(), |rt| {
                        rt.0.storage
                            .load(&key("row-0"))?
                            .ok_or(Error::Missing)
                            .map(|_| ())
                    })
                    .await
            }
            Boundary::Actor => clone
                .establish_actor(|read| {
                    let row = read.load(&key("authority"))?.ok_or(Error::Denied)?;
                    if row.revision != 1 {
                        return Err(Error::Denied);
                    }
                    Ok(actor())
                })
                .await
                .map(|_| ()),
        }
    });
    let driving = tokio::time::timeout(BOUND, async {
        let mut observed = vec![];
        for index in 0..8 {
            let Some(seen) = entered.recv().await else {
                break;
            };
            observed.push(seen.boundary);
            // Invalidate AFTER native result captured its generation.
            if index < invalidations {
                runtime.invalidate();
            }
            let _ = seen.release.send(());
        }
        observed
    })
    .await;
    let result = tokio::time::timeout(BOUND, pending).await;
    *runtime.0.generation_test_hook.lock().unwrap() = None;
    let terminal = drain(&runtime).await;
    terminal.unwrap().unwrap();
    assert_eq!(driving.unwrap(), vec![boundary; 8]);
    let result = result.unwrap().unwrap();
    if invalidations == 8 {
        assert_eq!(result, Err(Error::Overloaded));
    } else {
        assert_eq!(result, Ok(()));
    }
    assert!(store.reads.load(Ordering::SeqCst) >= 16);
    zero_other_counts(
        &runtime,
        0,
        0,
        u64::from(boundary == Boundary::Observation && invalidations == 8),
        u64::from(boundary == Boundary::Actor && invalidations == 8),
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observation_eight_generation_changes_count_one_exhaustion() {
    generation_case(Boundary::Observation, 8).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actor_eight_generation_changes_count_one_exhaustion() {
    generation_case(Boundary::Actor, 8).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn observation_stable_eighth_attempt_does_not_count_exhaustion() {
    generation_case(Boundary::Observation, 7).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actor_stable_eighth_attempt_does_not_count_exhaustion() {
    generation_case(Boundary::Actor, 7).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolver_and_observation_errors_are_not_generation_exhaustion() {
    let runtime = runtime(MemoryStore::seeded());
    let nested_observation = runtime
        .observe(&actor(), |_| Err::<(), _>(Error::Overloaded))
        .await;
    let nested_actor = runtime.establish_actor(|_| Err(Error::Overloaded)).await;
    let denied_actor = runtime.establish_actor(|_| Err(Error::Denied)).await;
    let expired = actor().expires_at(20);
    let expired_observation = runtime.observe(&expired, |_| Ok(())).await;
    runtime.revoke(&actor());
    let revoked = runtime.observe(&actor(), |_| Ok(())).await;
    let terminal = drain(&runtime).await;
    terminal.unwrap().unwrap();
    assert_eq!(nested_observation, Err(Error::Overloaded));
    assert_eq!(nested_actor, Err(Error::Overloaded));
    assert_eq!(denied_actor, Err(Error::Denied));
    assert_eq!(expired_observation, Err(Error::Denied));
    assert_eq!(revoked, Err(Error::Denied));
    zero_other_counts(&runtime, 0, 0, 0, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn action_32_owned_commands_and_reaction_rejection_survive_caller_cancel() {
    let store = MemoryStore::seeded();
    let runtime = runtime(store.clone());
    let (pause, mut entered) = NativePause::new(1);
    let release_guard = Release(pause.clone());
    *store.first_load_pause.lock().unwrap() = Some(pause.clone());
    let mut tasks = vec![];
    let mut first_entry = None;
    for index in 0..32 {
        let clone = runtime.clone();
        tasks.push(tokio::spawn(async move {
            clone
                .execute(
                    &actor(),
                    Command::replace(&format!("row-{index}"), Record { writable: true })
                        .at_revision(1)
                        .idempotency(&format!("replace-{index}")),
                )
                .await
        }));
        if index == 0 {
            // Record failure as data; finite native guard always releases.
            first_entry = Some(tokio::time::timeout(BOUND, entered.recv()).await);
        }
    }
    // Only one adapter-entered job; 31 are owned queued/gate-waiting.
    let saturated = until(|| {
        runtime.available_capacity() == 0
            && runtime.available_io_capacity() == 0
            && runtime.status().is_ok_and(|s| s.owned_work == 32)
    })
    .await;
    let cancelled = tasks.remove(0);
    cancelled.abort();
    let cancellation = cancelled.await;
    let blocked_status = runtime.status();
    let blocked_actions = runtime.available_capacity();
    let command_rejection = runtime
        .execute(
            &actor(),
            Command::replace("extra", Record { writable: true })
                .at_revision(1)
                .idempotency("extra"),
        )
        .await
        .map(|_| ());
    let reaction_rejection = runtime.process_reactions(1).await;
    let blocked_counts = runtime.core_overload_stats();
    let mut shutdown = Box::pin(runtime.shutdown());
    let shutdown_pending =
        std::future::poll_fn(|cx| Poll::Ready(shutdown.as_mut().poll(cx).is_pending())).await;
    // Closed intake must be published BEFORE release; otherwise concurrent
    // command projection batches could create unrelated IO-pressure events.
    pause.release();
    drop(release_guard);
    let terminal = tokio::time::timeout(BOUND, shutdown).await;
    let mut joined = vec![];
    for task in tasks {
        joined.push(tokio::time::timeout(BOUND, task).await);
    }
    terminal.unwrap().unwrap();
    assert_eq!(first_entry.unwrap().unwrap(), Some(0));
    assert!(saturated.is_ok());
    assert!(shutdown_pending);
    assert!(cancellation.unwrap_err().is_cancelled());
    assert_eq!(blocked_status.unwrap().owned_work, 32);
    assert_eq!(blocked_actions, 0);
    assert_eq!(command_rejection, Err(Error::Overloaded));
    assert_eq!(reaction_rejection, Err(Error::Overloaded));
    assert_eq!(blocked_counts.action_no_permits, 2);
    assert_eq!(blocked_counts.io_no_permits, 0);
    for result in joined {
        let result = result.unwrap().unwrap().map(|_| ());
        assert!(result == Ok(()) || result == Err(Error::Closed));
    }
    assert_eq!(runtime.status().unwrap().owned_work, 0);
    assert_eq!(runtime.available_capacity(), 32);
    zero_other_counts(&runtime, 0, 2, 0, 0);
}
