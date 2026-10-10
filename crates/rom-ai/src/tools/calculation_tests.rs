//! The restricted wrapper has its own real adapter lifetime/deadline evidence.
use super::ReadContext;
use crate::ExecutionDeadline;
use rom::{Actor, Error, Runtime, Storage};
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
struct Fixture {
    directory: std::path::PathBuf,
    runtime: Arc<Runtime>,
    _storage: Arc<dyn Storage>,
}
fn fixture(redb: bool) -> Fixture {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let directory = std::env::temp_dir().join(format!(
        "rom-ai-context-cpu-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(&path).unwrap())
    } else {
        Arc::new(rom_sqlite::Sqlite::open(&path).unwrap())
    };
    let runtime = Arc::new(
        Runtime::builder()
            .limits(rom::Limits {
                io_jobs: 1,
                ..Default::default()
            })
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap(),
    );
    Fixture {
        directory,
        runtime,
        _storage: storage,
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn context(runtime: &Arc<Runtime>, milliseconds: u64) -> ReadContext {
    ReadContext {
        runtime: runtime.clone(),
        actor: Actor::trusted("cpu-fixture", "owner"),
        lease: None,
        deadline: ExecutionDeadline::from_remaining(Duration::from_millis(milliseconds)).unwrap(),
    }
}
struct Block {
    released: Mutex<bool>,
    wake: Condvar,
    started: tokio::sync::Notify,
}
impl Block {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            released: Mutex::new(false),
            wake: Condvar::new(),
            started: tokio::sync::Notify::new(),
        })
    }
    fn run(&self) -> rom::Result<u64> {
        self.started.notify_one();
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.wake.wait(released).unwrap();
        }
        Ok(42)
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
struct Release(Arc<Block>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
async fn exercise(redb: bool, cancel: bool) {
    let fixture = fixture(redb);
    let block = Block::new();
    let release = Release(block.clone());
    let ctx = context(&fixture.runtime, if cancel { 1000 } else { 50 });
    let job = block.clone();
    let task = tokio::spawn(async move { ctx.calculate(move || job.run()).await });
    tokio::time::timeout(Duration::from_secs(1), block.started.notified())
        .await
        .unwrap();
    if cancel {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    } else {
        assert_eq!(task.await.unwrap(), Err(Error::Closed));
    }
    assert_eq!(fixture.runtime.status().unwrap().available_io_permits, 0);
    assert!(fixture.runtime.status().unwrap().owned_work > 0);
    assert_eq!(
        context(&fixture.runtime, 1000)
            .calculate(|| Ok(7_u64))
            .await,
        Err(Error::Overloaded)
    );
    let runtime = fixture.runtime.clone();
    let draining = tokio::spawn(async move { runtime.shutdown().await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(!draining.is_finished());
    release.0.release();
    tokio::time::timeout(Duration::from_secs(1), draining)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        fixture.runtime.status().unwrap().intake,
        rom::IntakeState::Stopped
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrapper_deadline_keeps_real_adapter_admission_and_shutdown_until_cpu_finishes() {
    for redb in [false, true] {
        exercise(redb, false).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrapper_cancellation_keeps_real_adapter_admission_and_shutdown_until_cpu_finishes() {
    for redb in [false, true] {
        exercise(redb, true).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_wrapper_deadline_submits_no_cpu_job() {
    for redb in [false, true] {
        let fixture = fixture(redb);
        let context = context(&fixture.runtime, 1);
        tokio::time::sleep(Duration::from_millis(5)).await;
        let called = Arc::new(AtomicBool::new(false));
        let witness = called.clone();
        assert_eq!(
            context
                .calculate(move || {
                    witness.store(true, Ordering::SeqCst);
                    Ok(7_u64)
                })
                .await,
            Err(Error::Closed)
        );
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(fixture.runtime.status().unwrap().available_io_permits, 1);
        fixture.runtime.shutdown().await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrapper_caught_cpu_panic_is_sanitized_and_does_not_close_runtime() {
    for redb in [false, true] {
        let fixture = fixture(redb);
        assert_eq!(
            context(&fixture.runtime, 1000)
                .calculate::<u64, _>(|| panic!("fixture CPU panic"))
                .await,
            Err(Error::Panicked)
        );
        assert!(fixture.runtime.status().unwrap().is_ready());
        assert_eq!(
            context(&fixture.runtime, 1000)
                .calculate(|| Ok(7_u64))
                .await
                .unwrap(),
            7
        );
        fixture.runtime.shutdown().await.unwrap();
    }
}
