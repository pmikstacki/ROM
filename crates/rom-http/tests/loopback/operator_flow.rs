//! Actual Resource actions create work; HTTP only binds the shared operator contract.
use super::*;
use rom::operator::*;
use rom::{Action, Channel, DeliveryOutcome, PrincipalKind, Resource, Storage};
use std::sync::atomic::{AtomicUsize, Ordering};

const NOTICE: Channel<String> = Channel::new("task-notice", 1);
const NOTIFY: Action<Task, ()> = Action::new("notify", |task, ()| {
    task.done = true;
    Ok(vec![NOTICE.intent("PRIVATE-PAYLOAD".into())])
});

struct Owner;
impl OperatorAuthorizer for Owner {
    fn authorize(
        &self,
        actor: &Actor,
        _: OperatorAccess,
        _: Option<&WorkScope>,
        _: &mut dyn rom::AuthorizationRead,
    ) -> rom::Result<()> {
        if actor.subject == "alice" {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}

#[tokio::test]
async fn operator_http_matches_embedded_inspection_and_atomic_retry_without_resending() {
    for service_access in [false, true] {
        operator_flow(service_access).await;
    }
}

fn service_policy(actor: &Actor, access: rom::Access, task: &Task) -> bool {
    rom_consumer::task_policy(actor, access, task)
        || (matches!(access, rom::Access::Read)
            && actor.authority == "local"
            && actor.subject == "PRIVATE-SERVICE"
            && actor.principal_kind() == PrincipalKind::Service)
}

async fn operator_flow(service_access: bool) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let sends = Arc::new(AtomicUsize::new(0));
    let sent = sends.clone();
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .policy(if service_access {
                    service_policy
                } else {
                    rom_consumer::task_policy
                })
                .allow_all_fields()
                .action(NOTIFY),
        )
        .operator_authorizer(Arc::new(Owner))
        .channel(
            NOTICE,
            Actor::trusted("local", "PRIVATE-SERVICE").with_kind(PrincipalKind::Service),
            move |_| {
                sent.fetch_add(1, Ordering::SeqCst);
                async { DeliveryOutcome::Accepted }
            },
        )
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    let server = Server::with_runtime(runtime, store, Limits::default()).await;
    let actor = Actor::trusted("local", "alice");
    server.runtime.invoke(&actor, create()).await.unwrap();
    server
        .runtime
        .execute(
            &actor,
            Command::action("one", NOTIFY, ())
                .at_revision(1)
                .idempotency("PRIVATE-ROOT"),
        )
        .await
        .unwrap();
    let query = WorkQuery {
        state: None,
        category: None,
        definition: None,
        limit: 8,
        cursor: None,
    };
    let embedded = server
        .runtime
        .work_list(&actor, query.clone())
        .await
        .unwrap();
    assert_eq!(embedded.records.len(), 1);
    let listed = server
        .post(
            "/work/list",
            "owner-secret",
            &serde_json::to_string(&query).unwrap(),
        )
        .await;
    assert_eq!(status(&listed), 200);
    assert_eq!(body(&listed), serde_json::to_value(&embedded).unwrap());
    for sentinel in ["PRIVATE-PAYLOAD", "PRIVATE-SERVICE", "PRIVATE-ROOT"] {
        assert!(!listed.contains(sentinel));
    }
    let view = &embedded.records[0];
    let read = json!({"handle": view.handle}).to_string();
    let shown = server.post("/work/read", "owner-secret", &read).await;
    assert_eq!(status(&shown), 200);
    assert_eq!(body(&shown), serde_json::to_value(view).unwrap());
    assert_eq!(
        status(&server.post("/work/read", "other-secret", &read).await),
        403
    );
    let before = server.store.counts().unwrap();
    let request = WorkControlRequest {
        handle: view.handle.clone(),
        expected: view.version.clone(),
        key: "operator-retry".into(),
        retry_epoch: 0,
        operation: WorkControlOperation::Retry,
    };
    let wire = serde_json::to_string(&request).unwrap();
    assert_eq!(
        status(&server.post("/work/control", "other-secret", &wire).await),
        403
    );
    // Operator authority alone cannot schedule work whose service cannot read the source.
    if !service_access {
        assert_eq!(
            status(&server.post("/work/control", "owner-secret", &wire).await),
            403
        );
        assert!(
            server
                .store
                .work_snapshot(2048, 4 * 1024 * 1024)
                .unwrap()
                .operator
                .receipts
                .is_empty()
        );
        assert_eq!(server.store.counts().unwrap(), before);
        assert_eq!(sends.load(Ordering::SeqCst), 0);
        server.finish().await;
        return;
    }
    let first = server.post("/work/control", "owner-secret", &wire).await;
    assert_eq!(status(&first), 200, "{first}");
    let first: WorkControlResult = serde_json::from_value(body(&first)).unwrap();
    assert!(!first.replayed);
    assert_eq!(first.outcome, WorkControlOutcome::Scheduled);
    let replay = server.runtime.work_control(&actor, request).await.unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.version, first.version);
    assert_eq!(server.store.counts().unwrap(), before);
    assert_eq!(
        server
            .store
            .work_snapshot(2048, 4 * 1024 * 1024)
            .unwrap()
            .operator
            .receipts
            .len(),
        1
    );
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    server.finish().await;
}
