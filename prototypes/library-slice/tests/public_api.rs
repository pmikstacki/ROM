use rom_library_slice::{
    json, resource, Access, Actor, Authorizer, Command, DenyAll, ErrorCode, Query, Resource,
    Runtime, Value,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
};
use tokio::sync::oneshot;

fn complete(data: &mut Value, input: &Value) -> std::result::Result<(), String> {
    if input != &json!({}) {
        return Err("complete accepts no arguments".into());
    }
    data["done"] = json!(true);
    Ok(())
}
fn reject(_: &mut Value, _: &Value) -> std::result::Result<(), String> {
    Err("business rule rejects this action".into())
}
resource! {Task("tasks"){title:String,done:bool} actions{"complete"=>complete,"reject"=>reject,"fail-event"=>complete}}
resource! {Switch("switches"){enabled:bool} actions{}}
#[derive(Clone)]
struct Policy {
    write: bool,
    read: bool,
    query: bool,
}
impl Authorizer for Policy {
    fn allows(&self, actor: &Actor, access: Access<'_>, _: Option<&Resource>) -> bool {
        actor.0 == "alice"
            && match access {
                Access::Action { .. } => self.write,
                Access::Read { .. } => self.read,
                Access::Query { .. } => self.query,
            }
    }
}
fn allowed() -> Policy {
    Policy {
        write: true,
        read: true,
        query: true,
    }
}
fn alice() -> Actor {
    Actor::new("alice")
}
fn open_tasks() -> Query {
    Query::equals(Task::KIND, "done", false)
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-library-slice-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn db(&self) -> PathBuf {
        self.0.join("PROTOTYPE-wipe-me.sqlite")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
async fn runtime(scratch: &Scratch) -> Runtime {
    Runtime::builder()
        .resource(Task::definition())
        .authorizer(allowed())
        .open_sqlite(scratch.db())
        .await
        .unwrap()
}
async fn create(runtime: &Runtime, id: &str) -> rom_library_slice::Outcome {
    runtime
        .execute(
            &alice(),
            Command::create(Task::KIND, id, json!({"title":id,"done":false})),
        )
        .await
        .unwrap()
}
async fn changed(
    subscription: &mut rom_library_slice::Subscription,
) -> rom_library_slice::Result<rom_library_slice::Snapshot> {
    tokio::time::timeout(std::time::Duration::from_secs(5), subscription.changed())
        .await
        .expect("live-query progress deadline")
}

#[tokio::test]
async fn declaration_only_second_kind_and_default_deny() {
    let scratch = Scratch::new();
    let runtime = Runtime::builder()
        .resource(Task::definition())
        .resource(Switch::definition())
        .open_sqlite(scratch.db())
        .await
        .unwrap();
    let err = runtime
        .execute(
            &alice(),
            Command::create(Switch::KIND, "one", json!({"enabled":false})),
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(
        runtime
            .get(&alice(), Task::KIND, "absent")
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    assert!(
        matches!(runtime.live(&alice(),open_tasks()).await,Err(e)if e.code==ErrorCode::Forbidden)
    );
    runtime.set_authorizer(allowed()).await.unwrap();
    let result = runtime
        .execute(
            &alice(),
            Command::create(Switch::KIND, "one", json!({"enabled":false})),
        )
        .await
        .unwrap();
    assert_eq!(result.resource.data["enabled"], false);
    assert_eq!(
        runtime
            .get(&alice(), Switch::KIND, "one")
            .await
            .unwrap()
            .revision,
        1
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn filtered_live_query_adds_removes_and_deletes_through_shared_pipeline() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    let mut live = runtime.live(&alice(), open_tasks()).await.unwrap();
    assert!(live.current().await.unwrap().rows.is_empty());
    create(&runtime, "one").await;
    assert_eq!(changed(&mut live).await.unwrap().rows.len(), 1);
    runtime
        .execute(
            &alice(),
            Command::action(Task::KIND, "one", "complete", 1, json!({})),
        )
        .await
        .unwrap();
    assert!(changed(&mut live).await.unwrap().rows.is_empty());
    runtime
        .execute(
            &alice(),
            Command::update(Task::KIND, "one", 2, json!({"done":false})),
        )
        .await
        .unwrap();
    assert_eq!(changed(&mut live).await.unwrap().rows.len(), 1);
    runtime
        .execute(&alice(), Command::delete(Task::KIND, "one", 3))
        .await
        .unwrap();
    assert!(changed(&mut live).await.unwrap().rows.is_empty());
    let events = runtime.journal(&alice()).await.unwrap();
    assert_eq!(events.len(), 4);
    assert!(events.last().unwrap().resource.deleted);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn custom_action_has_same_authorization_revision_and_validation_boundary() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    create(&runtime, "one").await;
    runtime
        .set_authorizer(Policy {
            write: false,
            ..allowed()
        })
        .await
        .unwrap();
    assert_eq!(
        runtime
            .execute(
                &alice(),
                Command::action(Task::KIND, "one", "complete", 1, json!({}))
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    runtime.set_authorizer(allowed()).await.unwrap();
    let stale = runtime
        .execute(
            &alice(),
            Command::action(Task::KIND, "one", "complete", 0, json!({})),
        )
        .await
        .unwrap_err();
    assert_eq!(stale.code, ErrorCode::Conflict);
    assert!(stale.message.contains("action complete"));
    let invalid = runtime
        .execute(
            &alice(),
            Command::action(Task::KIND, "one", "complete", 1, json!({"unexpected":true})),
        )
        .await
        .unwrap_err();
    assert_eq!(invalid.code, ErrorCode::Invalid);
    assert!(invalid.message.contains("tasks/one, action complete"));
    assert_eq!(
        runtime
            .get(&alice(), Task::KIND, "one")
            .await
            .unwrap()
            .revision,
        1
    );
    assert_eq!(runtime.journal(&alice()).await.unwrap().len(), 1);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn policy_replacement_without_mutation_terminates_subscription_and_blocks_buffered_delivery()
{
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    create(&runtime, "one").await;
    let mut live = runtime.live(&alice(), open_tasks()).await.unwrap();
    assert_eq!(live.current().await.unwrap().rows.len(), 1);
    runtime.set_authorizer(DenyAll).await.unwrap();
    assert_eq!(live.current().await.unwrap_err().code, ErrorCode::Forbidden);
    assert_eq!(
        changed(&mut live).await.unwrap_err().code,
        ErrorCode::Forbidden
    );
    assert_eq!(
        changed(&mut live).await.unwrap_err().code,
        ErrorCode::Closed
    );
    runtime.set_authorizer(allowed()).await.unwrap();
    assert_eq!(runtime.journal(&alice()).await.unwrap().len(), 1);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn row_policy_revocation_recomputes_membership_without_a_data_event() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    create(&runtime, "one").await;
    let mut live = runtime.live(&alice(), open_tasks()).await.unwrap();
    runtime
        .set_authorizer(Policy {
            read: false,
            ..allowed()
        })
        .await
        .unwrap();
    let result = changed(&mut live).await.unwrap();
    assert!(result.rows.is_empty());
    assert_eq!(result.journal_cursor, 1);
    assert_eq!(result.policy_epoch, 1);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn slow_consumers_coalesce_latest_snapshot_but_journal_retains_every_commit() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    let mut first = runtime.live(&alice(), open_tasks()).await.unwrap();
    let mut second = runtime.live(&alice(), open_tasks()).await.unwrap();
    assert_eq!(
        runtime.active_queries().await.unwrap(),
        1,
        "same actor/query must share one registration"
    );
    let mut latest = 0;
    for number in 0..24 {
        latest = create(&runtime, &number.to_string()).await.generation;
    }
    runtime.settle_queries().await.unwrap();
    let a = changed(&mut first).await.unwrap();
    let b = changed(&mut second).await.unwrap();
    assert_eq!(a, b);
    assert_eq!(a.generation, latest);
    assert_eq!(a.rows.len(), 24);
    assert!(!first.has_changed());
    assert_eq!(runtime.journal(&alice()).await.unwrap().len(), 24);
    drop(first);
    drop(second);
    assert_eq!(runtime.active_queries().await.unwrap(), 0);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn failed_action_and_sql_failure_publish_neither_snapshot_nor_event() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    create(&runtime, "one").await;
    let live = runtime.live(&alice(), open_tasks()).await.unwrap();
    let before = live.current().await.unwrap();
    assert_eq!(
        runtime
            .execute(
                &alice(),
                Command::action(Task::KIND, "one", "reject", 1, json!({}))
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Invalid
    );
    // Backend fault occurs after the resource UPDATE but at journal INSERT. No library-only test hook.
    let connection = rusqlite::Connection::open(scratch.db()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events WHEN NEW.action='fail-event' BEGIN SELECT RAISE(ABORT,'injected event write failure'); END;").unwrap();
    assert_eq!(
        runtime
            .execute(
                &alice(),
                Command::action(Task::KIND, "one", "fail-event", 1, json!({}))
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::Storage
    );
    runtime.settle_queries().await.unwrap();
    assert_eq!(live.current().await.unwrap(), before);
    assert!(!live.has_changed());
    assert_eq!(
        runtime.get(&alice(), Task::KIND, "one").await.unwrap().data["done"],
        false
    );
    assert_eq!(runtime.journal(&alice()).await.unwrap().len(), 1);
    runtime.shutdown().await.unwrap();
}

struct Gate {
    armed: AtomicBool,
    entered: Mutex<Option<oneshot::Sender<()>>>,
    released: Mutex<bool>,
    wake: Condvar,
}
struct GatedPolicy(Arc<Gate>);
impl Authorizer for GatedPolicy {
    fn allows(&self, actor: &Actor, access: Access<'_>, resource: Option<&Resource>) -> bool {
        if matches!(access, Access::Read { .. }) && self.0.armed.swap(false, Ordering::SeqCst) {
            if let Some(entered) = self.0.entered.lock().unwrap().take() {
                let _ = entered.send(());
            }
            let mut released = self.0.released.lock().unwrap();
            while !*released {
                released = self.0.wake.wait(released).unwrap();
            }
        }
        allowed().allows(actor, access, resource)
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_registration_race_is_serialized_with_a_deterministic_gate() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    create(&runtime, "one").await;
    let (entered, inside) = oneshot::channel();
    let gate = Arc::new(Gate {
        armed: AtomicBool::new(true),
        entered: Mutex::new(Some(entered)),
        released: Mutex::new(false),
        wake: Condvar::new(),
    });
    runtime
        .set_authorizer(GatedPolicy(gate.clone()))
        .await
        .unwrap();
    let subscriber = runtime.clone();
    let subscription = tokio::spawn(async move { subscriber.live(&alice(), open_tasks()).await });
    inside.await.unwrap(); // Rows have been loaded; authorizer pauses before query registration while lock held.
    let writer = runtime.clone();
    let (started, attempting) = oneshot::channel();
    let (done, mut completed) = oneshot::channel();
    let mutation = tokio::spawn(async move {
        let _ = started.send(());
        let result = writer
            .execute(
                &alice(),
                Command::update(Task::KIND, "one", 1, json!({"done":true})),
            )
            .await;
        let _ = done.send(());
        result
    });
    attempting.await.unwrap();
    assert!(matches!(
        completed.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    *gate.released.lock().unwrap() = true;
    gate.wake.notify_all();
    let mut live = subscription.await.unwrap().unwrap();
    let changed_resource = mutation.await.unwrap().unwrap();
    let mut snapshot = live.current().await.unwrap();
    while snapshot.generation < changed_resource.generation {
        snapshot = changed(&mut live).await.unwrap();
    }
    assert!(
        snapshot.rows.is_empty(),
        "mutation between initial snapshot and return cannot be missed"
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn explicit_shutdown_closes_streams_and_rejects_more_operations() {
    let scratch = Scratch::new();
    let runtime = runtime(&scratch).await;
    let mut live = runtime.live(&alice(), open_tasks()).await.unwrap();
    runtime.shutdown().await.unwrap();
    assert_eq!(
        changed(&mut live).await.unwrap_err().code,
        ErrorCode::Closed
    );
    assert_eq!(
        runtime
            .get(&alice(), Task::KIND, "one")
            .await
            .unwrap_err()
            .code,
        ErrorCode::Closed
    );
}
