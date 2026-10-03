use super::{
    MemoryRecord,
    support::{BACKENDS, Backend, Process, Scratch, assert_saved, seed},
};
use rom::{
    Action, Actor, Channel, Command, Delivery, DeliveryOutcome, Error, PrincipalKind, Resource,
    Runtime, Storage, WorkPayload, WorkState,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

const NOTICE: Channel<String> = Channel::new("owner-recovery-notice", 1);
const NOTIFY: Action<MemoryRecord, ()> = Action::new("notify", |record, ()| {
    record.value = "committed before process exit".into();
    Ok(vec![NOTICE.intent(record.value.clone())])
});
type Deliveries = Arc<Mutex<Vec<(String, u32, String)>>>;

fn recovery_runtime(storage: Arc<dyn Storage>, delivered: Deliveries) -> Runtime {
    Runtime::builder()
        .resource(
            MemoryRecord::definition()
                .allow_all_fields()
                .policy(|_, _, _| true)
                .action(NOTIFY),
        )
        .channel(
            NOTICE,
            Actor::trusted("ownership", "notifier").with_kind(PrincipalKind::Service),
            move |delivery: Delivery<String>| {
                delivered
                    .lock()
                    .unwrap()
                    .push((delivery.id, delivery.attempt, delivery.payload));
                async { DeliveryOutcome::Accepted }
            },
        )
        .build(storage, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}

#[tokio::test]
async fn native_owner_child() {
    let Some(root) = std::env::var_os("ROM_OWNERSHIP_TEST_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let backend = if std::env::var("ROM_OWNERSHIP_TEST_BACKEND").unwrap() == "redb" {
        Backend::Redb
    } else {
        Backend::Sqlite
    };
    let storage: Arc<dyn Storage> = Arc::from(backend.open(&root.join("database")).unwrap());
    seed(&*storage, "before notification");
    let delivered = Deliveries::default();
    let runtime = recovery_runtime(storage.clone(), delivered.clone());
    runtime
        .execute(
            &Actor::trusted("ownership", "owner"),
            Command::action("saved", NOTIFY, ())
                .at_revision(1)
                .idempotency("notify"),
        )
        .await
        .unwrap();
    let pending = storage.reaction_records().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].state, WorkState::Pending);
    assert!(matches!(
        pending[0].pending.payload,
        WorkPayload::Notification { .. }
    ));
    assert!(delivered.lock().unwrap().is_empty());
    std::fs::write(root.join("ready"), b"committed with pending notification").unwrap();
    // Parent kills the owner without destructors, work draining or engine checkpoint-on-drop.
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::test]
async fn native_owner_process_exit_releases_lock_and_recovers_committed_data() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let mut child = Process::spawn("process::native_owner_child", &scratch.0, backend);
        child.wait_ready(&scratch.0.join("ready"));
        assert!(
            matches!(backend.open(&scratch.path()), Err(Error::Conflict)),
            "{backend:?}: live process owns the database"
        );
        let sidecar = scratch.0.join("database.rom-owner");
        assert!(sidecar.is_file());
        let status = child.kill();
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(status.signal(), Some(9));
        }
        assert!(sidecar.is_file(), "release must not delete the lock file");
        let reopened: Arc<dyn Storage> = Arc::from(backend.open(&scratch.path()).unwrap());
        assert_saved(&*reopened, "committed before process exit", 2);
        let pending = reopened.reaction_records().unwrap();
        assert_eq!(
            pending.len(),
            1,
            "process exit must leave durable work to recover"
        );
        assert_eq!(pending[0].state, WorkState::Pending);
        assert_eq!(pending[0].attempts, 0);
        assert!(matches!(
            pending[0].pending.payload,
            WorkPayload::Notification { .. }
        ));
        let work_id = pending[0].pending.id.clone();
        let delivered = Deliveries::default();
        let replacement = recovery_runtime(reopened.clone(), delivered.clone());
        assert_eq!(replacement.process_work(8).await.unwrap(), 1);
        assert_eq!(
            *delivered.lock().unwrap(),
            vec![(work_id.clone(), 1, "committed before process exit".into())]
        );
        let recovered = reopened.reaction_records().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].pending.id, work_id);
        assert_eq!(recovered[0].state, WorkState::Done);
        assert_eq!(recovered[0].attempts, 1);
        assert_eq!(recovered[0].delivery, Some(DeliveryOutcome::Accepted));
        assert_eq!(replacement.process_work(8).await.unwrap(), 0);
        replacement.shutdown().await.unwrap();
        drop(replacement);
        drop(reopened);

        // A second restart consumes the durable acknowledgement, not another delivery.
        let reopened: Arc<dyn Storage> = Arc::from(backend.open(&scratch.path()).unwrap());
        let replacement = recovery_runtime(reopened.clone(), delivered.clone());
        assert_eq!(replacement.process_work(8).await.unwrap(), 0);
        assert_eq!(delivered.lock().unwrap().len(), 1);
        assert_eq!(
            reopened.reaction_records().unwrap()[0].state,
            WorkState::Done
        );
        assert_saved(&*reopened, "committed before process exit", 2);
        replacement.shutdown().await.unwrap();
    }
}
