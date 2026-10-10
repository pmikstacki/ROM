//! Physical callback ownership is distinct from a caller deadline or persisted result absence.
use super::*;
use std::{
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Condvar,
    task::Poll,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
#[path = "read_recovery/readiness.rs"]
mod readiness;
#[path = "read_recovery/readiness_tests.rs"]
mod readiness_tests;
// Setup and bookkeeping bounds do not change the real callback deadlines.
const FIXTURE_STARTUP: Duration = Duration::from_secs(10);
const FIXTURE_COMPLETION: Duration = Duration::from_secs(30);
const FIXTURE_CLEANUP: Duration = Duration::from_secs(30);
pub(super) struct ReadBlock {
    pub started: AtomicBool,
    release: Mutex<bool>,
    changed: Condvar,
    entered: Mutex<Option<oneshot::Sender<Instant>>>,
    entry: Mutex<Option<oneshot::Receiver<Instant>>>,
}
impl ReadBlock {
    pub fn new() -> Self {
        let (send, receive) = oneshot::channel();
        Self {
            started: AtomicBool::new(false),
            release: Mutex::new(false),
            changed: Condvar::new(),
            entered: Mutex::new(Some(send)),
            entry: Mutex::new(Some(receive)),
        }
    }
    fn take_entered(&self) -> oneshot::Receiver<Instant> {
        self.entry
            .lock()
            .unwrap()
            .take()
            .expect("one physical-entry observer")
    }
    pub fn wait(&self) {
        eprintln!("physical CPU descendant started");
        self.started.store(true, Ordering::SeqCst);
        if let Some(send) = self.entered.lock().unwrap().take() {
            let _ = send.send(Instant::now());
        }
        let (released, deadline) = self
            .changed
            .wait_timeout_while(
                self.release.lock().unwrap(),
                std::time::Duration::from_secs(45),
                |released| !*released,
            )
            .unwrap();
        assert!(
            *released && !deadline.timed_out(),
            "bounded fixture release was not reached"
        );
    }
    fn release(&self) {
        *self.release.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
pub(super) async fn active_cpu(
    runtime: Runtime,
    client: rom_ai::flow::FlowClient,
    adapter: Arc<Adapter>,
    storage: Arc<dyn Storage>,
    handle: rom_ai::flow::RunHandle,
) {
    eprintln!("active CPU fixture entered");
    let block = adapter.blocked_read.as_ref().unwrap().clone();
    let worker_runtime = runtime.clone();
    let worker_adapter = adapter.clone();
    let entered = block.take_entered();
    let mut worker = readiness::Worker::new(
        block.clone(),
        tokio::spawn(async move {
            for _ in 0..16 {
                eprintln!("active CPU fixture processing one durable work");
                worker_runtime.process_work(1).await?;
                if worker_adapter.reads.load(Ordering::SeqCst) > 0 {
                    break;
                }
            }
            Ok(())
        }),
    );
    let setup_started = Instant::now();
    let entered_at = match worker.ready(entered, FIXTURE_STARTUP).await {
        Ok(entered_at) => entered_at,
        Err(error) => {
            let cleanup = cleanup(&block, &mut worker, &runtime).await;
            panic!(
                "physical CPU setup failed after {:?}: {error:?}; cleanup: {cleanup:?}",
                setup_started.elapsed()
            );
        }
    };
    eprintln!(
        "physical CPU entered after {:?} of fixture setup",
        setup_started.elapsed()
    );
    let checked = capture_assertions(async {
        assert!(block.started.load(Ordering::SeqCst));
        eprintln!("active CPU fixture before public view");
        let before = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(
            before.read_progress().unwrap().status(),
            rom_ai::flow::ReadStatus::Active
        );
        assert_eq!(before.read_progress().unwrap().ordinal(), 1);
        eprintln!("active CPU fixture before public resume");
        assert_eq!(
            client
                .resume(&owner(), &handle, before.revision(), "retry-live-read")
                .await
                .unwrap_err(),
            rom_ai::AiError::Conflict
        );
        assert_eq!(adapter.reads.load(Ordering::SeqCst), 1);
        // The real execution deadline returns while the synchronous child still owns Runtime admission.
        worker.finish(FIXTURE_COMPLETION).await.unwrap();
        eprintln!("physical CPU callback wait returned: entry_to_observed_terminal={:?}, elapsed={:?}, terminal={:?}; child still owns work",
            worker.terminal().map(|terminal| terminal.observed_at.saturating_duration_since(entered_at)),
            entered_at.elapsed(), worker.terminal());
        assert!(runtime.status().unwrap().owned_work > 0);
        let second_runtime = Runtime::builder().build(storage, Runtime::shared_cpu_pool(1).unwrap());
        assert!(
            matches!(second_runtime, Err(rom::Error::Conflict)),
            "exclusive ownership rejects a second live Runtime"
        );
        let after_timeout = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(after_timeout.state(), &RunState::ToolsPending);
        assert_eq!(
            after_timeout.read_progress().unwrap().status(),
            rom_ai::flow::ReadStatus::Active
        );
        let raw = runtime.read::<AiRun>(&service(), &handle.0).await.unwrap();
        let pure = rom_ai::flow::RunView::project(
            &raw.value.unwrap().record().unwrap(),
            raw.revision,
            &owner(),
            |actor, identity| Authority.inspect(actor, identity),
        )
        .unwrap();
        assert_eq!(
            pure.read_progress().unwrap().status(),
            rom_ai::flow::ReadStatus::ActivityUnknown
        );
        assert_eq!(
            client
                .resume(
                    &owner(),
                    &handle,
                    after_timeout.revision(),
                    "retry-after-timeout"
                )
                .await
                .unwrap_err(),
            rom_ai::AiError::Conflict
        );
        assert_eq!(
            adapter.reads.load(Ordering::SeqCst),
            1,
            "a timeout does not grant physical takeover"
        );
        block.release();
        for _ in 0..100 {
            if runtime.status().unwrap().owned_work == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(runtime.status().unwrap().owned_work, 0);
        let before_retry = client.view(&owner(), &handle).await.unwrap();
        assert_eq!(
            before_retry.read_progress().unwrap().status(),
            rom_ai::flow::ReadStatus::AwaitingRecovery
        );
        client
            .resume(
                &owner(),
                &handle,
                before_retry.revision(),
                "retry-after-timeout",
            )
            .await
            .unwrap();
        for _ in 0..16 {
            runtime.process_work(1).await.unwrap();
            if client.view(&owner(), &handle).await.unwrap().state() == &RunState::Completed {
                break;
            }
        }
        assert_eq!(
            client.view(&owner(), &handle).await.unwrap().state(),
            &RunState::Completed
        );
        assert_eq!(adapter.reads.load(Ordering::SeqCst), 2);
        assert_eq!(adapter.attempts.lock().unwrap().len(), 2);
        assert_eq!(adapter.lookups.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime
                .read::<Task>(&owner(), "task")
                .await
                .unwrap()
                .revision,
            2
        );
    }).await;
    // Assertion failures release and join the physical descendant before resuming the panic.
    let cleanup = cleanup(&block, &mut worker, &runtime).await;
    if let Err(panic) = checked {
        eprintln!("physical CPU assertion failed; cleanup: {cleanup:?}");
        std::panic::resume_unwind(panic);
    }
    assert!(
        cleanup.drain_complete,
        "physical CPU cleanup is incomplete: {cleanup:?}"
    );
    cleanup.worker.unwrap();
    cleanup
        .runtime
        .expect("bounded fixture Runtime cleanup")
        .unwrap();
}

#[derive(Debug)]
struct Cleanup {
    drain_complete: bool,
    worker: Result<(), readiness::StartupError>,
    runtime: Result<rom::Result<()>, tokio::time::error::Elapsed>,
}
async fn cleanup(block: &ReadBlock, worker: &mut readiness::Worker, runtime: &Runtime) -> Cleanup {
    let started = Instant::now();
    block.release();
    let before = runtime.status();
    let joined = worker.finish(FIXTURE_CLEANUP).await;
    let drained = tokio::time::timeout(FIXTURE_CLEANUP, runtime.shutdown()).await;
    let after = runtime.status();
    let drain_complete = !worker.pending()
        && matches!(&drained, Ok(Ok(())))
        && matches!(&after, Ok(status) if status.owned_work == 0);
    eprintln!(
        "physical CPU cleanup: drain_complete={drain_complete}, pending_worker={}, terminal={:?}, worker_wait={joined:?}, runtime_wait={drained:?}, ownership_before={before:?}, ownership_after={after:?}, elapsed={:?}",
        worker.pending(),
        worker.terminal(),
        started.elapsed()
    );
    Cleanup {
        drain_complete,
        worker: joined,
        runtime: drained,
    }
}

// Catch only fixture assertions, so cleanup can await retained worker ownership.
async fn capture_assertions<F: Future>(
    future: F,
) -> Result<F::Output, Box<dyn std::any::Any + Send>> {
    tokio::pin!(future);
    poll_fn(
        |cx| match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
            Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(panic) => Poll::Ready(Err(panic)),
        },
    )
    .await
}

fn generated_future_bytes<F: std::future::Future, G: FnOnce() -> F>(_: G) -> usize {
    std::mem::size_of::<F>()
}
pub(super) async fn future_size() {
    eprintln!(
        "recovery fixture future bytes: {}",
        generated_future_bytes(|| super::recovery(false, 18))
    );
    let directory =
        std::env::temp_dir().join(format!("rom-ai-read-future-size-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let storage: Arc<dyn Storage> =
        Arc::new(rom_sqlite::Sqlite::open(directory.join("db")).unwrap());
    let adapter = Arc::new(Adapter {
        attempts: Mutex::new(vec![]),
        lookups: AtomicU64::new(0),
        repeat_call: false,
        reads: AtomicU64::new(0),
        blocked_read: None,
        now: Arc::new(AtomicU64::new(1000)),
        read_admission: None,
        slow_authority: Arc::new(AtomicBool::new(false)),
    });
    let (runtime, client) = recovery_host(
        adapter,
        Arc::new(AtomicBool::new(true)),
        storage,
        false,
        false,
        17,
    );
    eprintln!(
        "Runtime.process_work future bytes: {}",
        generated_future_bytes(|| runtime.process_work(1))
    );
    let actor = owner();
    let handle = rom_ai::flow::RunHandle::new("size-probe").unwrap();
    let future = client.resume(&actor, &handle, 1, "size-probe");
    eprintln!(
        "standalone public resume future bytes: {}",
        std::mem::size_of_val(&future)
    );
    drop(future);
    runtime.shutdown().await.unwrap();
}

/// The explicit wake admits an intention; it does not change a callback ordinal or outcome.
pub(super) fn assert_wake(before: &rom_ai::flow::RunRecord, after: &rom_ai::flow::RunRecord) {
    let mut expected = serde_json::to_value(before).unwrap();
    let actual = serde_json::to_value(after).unwrap();
    let old_ops = expected["operations"].as_array().unwrap();
    let new_ops = actual["operations"].as_array().unwrap();
    assert_eq!(new_ops.len(), old_ops.len() + 1);
    assert_eq!(&new_ops[..old_ops.len()], old_ops);
    assert_eq!(
        new_ops.last().unwrap()["phase"]["Finished"]["category"],
        "Unresolved"
    );
    expected["operations"] = actual["operations"].clone();
    expected["counters"]["ticks"] = serde_json::json!(before.counters().ticks() + 1);
    assert_eq!(
        expected, actual,
        "wake changed more than one immutable admission and one tick"
    );
}
