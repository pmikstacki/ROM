//! THROWAWAY PROTOTYPE: executable observations, not a ROM implementation.
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    future::{poll_fn, Future},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    task::Poll,
    time::{Duration, Instant},
};
use tokio::{
    runtime::{Builder, Handle},
    sync::{oneshot, OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
};

type CpuResult = Result<u64, &'static str>;

#[derive(Clone)]
struct Host {
    runtime: Handle,
    pool: Arc<ThreadPool>,
    slots: Arc<Semaphore>,
}

struct Supervisor {
    host: Host,
    accepting: bool,
    tasks: JoinSet<(CpuResult, bool)>,
    submitted: usize,
}

impl Supervisor {
    fn new(host: Host) -> Self {
        Self {
            host,
            accepting: true,
            tasks: JoinSet::new(),
            submitted: 0,
        }
    }

    fn try_submit(
        &mut self,
        work: impl FnOnce() -> u64 + Send + 'static,
    ) -> Result<oneshot::Receiver<CpuResult>, &'static str> {
        if !self.accepting {
            return Err("closed");
        }
        let permit = self
            .host
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "full")?;
        let (done_tx, done_rx) = oneshot::channel();
        let (reply_tx, reply_rx) = oneshot::channel();
        self.host.pool.spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(work)).map_err(|_| "cpu panic");
            // The closure, not its async waiter, owns capacity until work finishes.
            drop(permit);
            let _ = done_tx.send(result);
        });
        self.tasks.spawn_on(
            async move {
                let result = done_rx.await.unwrap_or(Err("completion channel closed"));
                let delivered = reply_tx.send(result).is_ok();
                (result, delivered)
            },
            &self.host.runtime,
        );
        self.submitted += 1;
        Ok(reply_rx)
    }

    async fn drain(&mut self) -> Vec<(CpuResult, bool)> {
        self.accepting = false;
        let mut completed = Vec::new();
        while let Some(result) = self.tasks.join_next().await {
            completed.push(result.expect("supervisor task must not panic"));
        }
        completed
    }
}

// Channels establish causality. No timer or scheduler-yield assertion is used.
fn gated_job(
    value: u64,
) -> (
    impl FnOnce() -> u64 + Send,
    oneshot::Receiver<()>,
    mpsc::Sender<()>,
) {
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let work = move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        value
    };
    (work, started_rx, release_tx)
}

async fn correct_bridge(host: Host) {
    let another_handle = host.clone();
    assert!(Arc::ptr_eq(&host.pool, &another_handle.pool));
    assert!(Arc::ptr_eq(&host.slots, &another_handle.slots));
    assert_eq!(host.pool.current_num_threads(), 3);
    let mut supervisor = Supervisor::new(another_handle);
    let (job_a, started_a, release_a) = gated_job(10);
    let (job_b, started_b, release_b) = gated_job(20);
    let waiter_a = tokio::spawn(supervisor.try_submit(job_a).unwrap());
    let waiter_b = tokio::spawn(supervisor.try_submit(job_b).unwrap());
    started_a.await.unwrap();
    started_b.await.unwrap();
    assert_eq!(host.slots.available_permits(), 0);
    assert!(matches!(
        supervisor.try_submit(|| panic!("overflow must never spawn")),
        Err("full")
    ));
    assert_eq!(supervisor.submitted, 2);
    println!("PASS shared host pool; capacity=2; overflow rejected before spawn");

    waiter_a.abort();
    waiter_b.abort();
    assert!(waiter_a.await.unwrap_err().is_cancelled());
    assert!(waiter_b.await.unwrap_err().is_cancelled());
    assert_eq!(host.slots.available_permits(), 0);
    assert!(matches!(supervisor.try_submit(|| 30), Err("full")));
    println!("PASS cancelling both reply waiters retains both running CPU permits");

    supervisor.accepting = false;
    assert!(matches!(supervisor.try_submit(|| 30), Err("closed")));
    let drain = supervisor.drain();
    tokio::pin!(drain);
    let pending = poll_fn(|cx| Poll::Ready(drain.as_mut().poll(cx).is_pending())).await;
    assert!(
        pending,
        "shutdown must wait for actual blocked CPU closures"
    );
    release_a.send(()).unwrap();
    release_b.send(()).unwrap();
    let completed = drain.await;
    assert_eq!(completed.len(), 2);
    assert!(completed
        .iter()
        .all(|(result, delivered)| result.is_ok() && !delivered));
    assert_eq!(host.slots.available_permits(), 2);
    println!("PASS shutdown remains pending until CPU release; abandoned replies do not panic");
}

async fn panic_bridge(host: Host) {
    let mut supervisor = Supervisor::new(host.clone());
    // resume_unwind simulates an unwinding panic without the global panic hook's noise.
    let reply = supervisor
        .try_submit(|| std::panic::resume_unwind(Box::new("injected")))
        .unwrap();
    assert_eq!(reply.await.unwrap(), Err("cpu panic"));
    let next = supervisor.try_submit(|| 42).unwrap();
    assert_eq!(next.await.unwrap(), Ok(42));
    let outcomes = supervisor.drain().await;
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|(_, delivered)| *delivered));
    assert_eq!(host.slots.available_permits(), 2);
    println!("PASS CPU unwind reported as error; permit restored; subsequent job succeeds");
}

// Deliberately WRONG: the async waiter owns capacity for independently running CPU work.
async fn naive_waiter(
    permit: OwnedSemaphorePermit,
    reply: oneshot::Receiver<u64>,
    entered: oneshot::Sender<()>,
) -> u64 {
    let _wrong_owner = permit;
    entered.send(()).unwrap();
    reply.await.unwrap()
}

async fn negative_control(host: Host) {
    let mut releases = Vec::new();
    let mut completions = Vec::new();
    for value in 0..2 {
        let permit = host.slots.clone().try_acquire_owned().unwrap();
        let (job, started, release) = gated_job(value);
        let (reply_tx, reply_rx) = oneshot::channel();
        let (done_tx, done_rx) = oneshot::channel();
        host.pool.spawn(move || {
            let result = job();
            let _ = reply_tx.send(result);
            done_tx.send(()).unwrap();
        });
        let (entered_tx, entered_rx) = oneshot::channel();
        let waiter = tokio::spawn(naive_waiter(permit, reply_rx, entered_tx));
        started.await.unwrap();
        entered_rx.await.unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        releases.push(release);
        completions.push(done_rx);
    }
    // Both original CPU closures are still waiting for their release signals.
    assert_eq!(host.slots.available_permits(), 2);
    let third_permit = host.slots.clone().try_acquire_owned().unwrap();
    let (third, started_third, release_third) = gated_job(3);
    let (done_tx, done_rx) = oneshot::channel();
    host.pool.spawn(move || {
        third();
        drop(third_permit);
        done_tx.send(()).unwrap();
    });
    started_third.await.unwrap();
    assert!(completions
        .iter_mut()
        .all(|rx| matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Empty))));
    println!("PASS negative control exposes violation: 3 CPU closures started/unreleased with advertised capacity=2");
    for release in releases {
        release.send(()).unwrap();
    }
    release_third.send(()).unwrap();
    for completion in completions {
        completion.await.unwrap();
    }
    done_rx.await.unwrap();
    assert_eq!(host.slots.available_permits(), 2);
}

fn checksum(seed: u64) -> u64 {
    let mut value = seed;
    for i in 0..120_000_000_u64 {
        value = std::hint::black_box(value.rotate_left(7).wrapping_add(i) ^ 0x9e3779b97f4a7c15);
    }
    value
}

async fn timer_observation(host: Host) {
    let expected_a = checksum(1);
    let expected_b = checksum(2);
    let mut supervisor = Supervisor::new(host);
    let (release_a, wait_a) = mpsc::channel();
    let (release_b, wait_b) = mpsc::channel();
    let (started_a, observed_a) = oneshot::channel();
    let (started_b, observed_b) = oneshot::channel();
    let running = Arc::new(AtomicUsize::new(2));
    let running_a = running.clone();
    let running_b = running.clone();
    let a = supervisor
        .try_submit(move || {
            started_a.send(()).unwrap();
            wait_a.recv().unwrap();
            let value = checksum(1);
            running_a.fetch_sub(1, Ordering::SeqCst);
            value
        })
        .unwrap();
    let b = supervisor
        .try_submit(move || {
            started_b.send(()).unwrap();
            wait_b.recv().unwrap();
            let value = checksum(2);
            running_b.fetch_sub(1, Ordering::SeqCst);
            value
        })
        .unwrap();
    observed_a.await.unwrap();
    observed_b.await.unwrap();
    let start = Instant::now();
    release_a.send(()).unwrap();
    release_b.send(()).unwrap();
    let timer = async {
        let mut lateness = Vec::new();
        let mut samples_with_cpu_running = 0;
        for i in 1..=20 {
            let deadline = start + Duration::from_millis(i * 2);
            tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
            if running.load(Ordering::SeqCst) > 0 {
                samples_with_cpu_running += 1;
            }
            lateness.push(
                Instant::now()
                    .saturating_duration_since(deadline)
                    .as_micros(),
            );
        }
        lateness.sort_unstable();
        (lateness[18], lateness[19], samples_with_cpu_running)
    };
    let jobs = async {
        (
            a.await.unwrap().unwrap(),
            b.await.unwrap().unwrap(),
            start.elapsed(),
        )
    };
    let ((p95_us, max_us, overlap), (actual_a, actual_b, cpu_elapsed)) = tokio::join!(timer, jobs);
    assert_eq!((actual_a, actual_b), (expected_a, expected_b));
    assert_eq!(supervisor.drain().await.len(), 2);
    println!("PASS CPU outputs match sequential reference; 20 async timer deadlines observed");
    println!("OBS timing only: two CPU jobs elapsed={}ms; timer lateness p95={}us max={}us; samples with CPU running={}/20 (no performance threshold)", cpu_elapsed.as_millis(), p95_us, max_us, overlap);
}

fn main() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let host = Host {
        runtime: runtime.handle().clone(),
        pool: Arc::new(ThreadPoolBuilder::new().num_threads(3).build().unwrap()),
        slots: Arc::new(Semaphore::new(2)),
    };
    runtime.block_on(async {
        correct_bridge(host.clone()).await;
        panic_bridge(host.clone()).await;
        negative_control(host.clone()).await;
        timer_observation(host.clone()).await;
    });
    println!("PASS all supervised probe work completed before host shutdown");
}
