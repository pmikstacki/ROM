use rom::{
    Actor, ActorGate, AuthorizationRead, Command, Error, PrincipalKind, Resource, Result, Runtime,
};
use rom_consumer::{Task, task_policy};
use rom_sqlite::Sqlite;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn actor(kind: PrincipalKind) -> Actor {
    Actor::trusted("local", "alice").with_kind(kind)
}
fn task() -> Task {
    Task {
        owner: "alice".into(),
        title: "one".into(),
        done: false,
        note: None,
    }
}

#[tokio::test]
async fn principal_kind_separates_receipt_and_revocation_identity() {
    let runtime = Runtime::builder()
        .resource(Task::definition().allow_all_fields().policy(task_policy))
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let human = actor(PrincipalKind::Human);
    let service = actor(PrincipalKind::Service);
    runtime
        .execute(&human, Command::create("task", task()).idempotency("same"))
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .execute(
                &service,
                Command::create("task", task()).idempotency("same")
            )
            .await,
        Err(Error::Conflict)
    ));
    runtime.revoke(&human);
    assert!(matches!(
        runtime.read::<Task>(&human, "task").await,
        Err(Error::Denied)
    ));
    assert!(runtime.read::<Task>(&service, "task").await.is_ok());
    runtime.shutdown().await.unwrap();
}

struct Gate {
    allowed: Arc<AtomicBool>,
    async_thread: std::thread::ThreadId,
}
impl ActorGate for Gate {
    fn check(&self, actor: &Actor, _storage: &mut dyn AuthorizationRead) -> Result<()> {
        assert_ne!(
            std::thread::current().id(),
            self.async_thread,
            "authoritative gate ran on async thread"
        );
        if self.allowed.load(Ordering::SeqCst) && actor.host_stamp() == Some("test-binding") {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
#[tokio::test(flavor = "current_thread")]
async fn gate_runs_on_bounded_io_and_denies_cached_receipt_and_live_delivery() {
    let allowed = Arc::new(AtomicBool::new(true));
    let runtime = Runtime::builder()
        .resource(Task::definition().allow_all_fields().policy(task_policy))
        .actor_gate(Arc::new(Gate {
            allowed: allowed.clone(),
            async_thread: std::thread::current().id(),
        }))
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    let a = actor(PrincipalKind::Human).with_host_stamp("test-binding");
    runtime
        .execute(&a, Command::create("task", task()).idempotency("same"))
        .await
        .unwrap();
    let mut live = runtime
        .live(&a, Task::done_field().equals(false))
        .await
        .unwrap();
    allowed.store(false, Ordering::SeqCst);
    assert!(matches!(
        runtime
            .execute(&a, Command::create("task", task()).idempotency("same"))
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        runtime.read::<Task>(&a, "missing").await,
        Err(Error::Denied)
    ));
    assert!(matches!(live.changed().await, Err(Error::Denied)));
    runtime.shutdown().await.unwrap();
}

struct BadGate(bool);
impl ActorGate for BadGate {
    fn check(&self, _: &Actor, storage: &mut dyn AuthorizationRead) -> Result<()> {
        assert!(!self.0, "synthetic policy failure");
        for _ in 0..9 {
            storage.load(&rom::Key {
                kind: "tasks".into(),
                id: "missing".into(),
            })?;
        }
        Ok(())
    }
}
#[tokio::test]
async fn gate_reads_are_bounded_and_policy_panic_fails_closed() {
    for panic in [false, true] {
        let runtime = Runtime::builder()
            .resource(Task::definition().allow_all_fields().policy(task_policy))
            .actor_gate(Arc::new(BadGate(panic)))
            .build(
                Arc::new(Sqlite::open(":memory:").unwrap()),
                Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        let result = runtime
            .execute(
                &actor(PrincipalKind::Human),
                Command::create("task", task()).idempotency("same"),
            )
            .await;
        assert!(
            matches!(result, Err(Error::Panicked)) && panic
                || matches!(result, Err(Error::TooLarge)) && !panic
        );
        let _ = runtime.shutdown().await;
    }
}
