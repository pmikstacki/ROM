use rom::{
    Action, Actor, Bundle, Capabilities, Command, Descriptor, Error, FieldDescriptor, Intent, Key,
    Receipt, Resource, Result, Row, Runtime, Shape, Storage, Value, json,
};
use rom_consumer::{COMPLETE, ENABLE, Setting, Task, declarations, setting_policy, task_policy};
use rom_sqlite::Sqlite;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
fn alice() -> Actor {
    Actor::trusted("local", "alice")
}
fn task() -> Task {
    Task {
        owner: "alice".into(),
        title: "private title".into(),
        done: false,
        note: None,
    }
}
fn setting() -> Setting {
    Setting {
        owner: "alice".into(),
        enabled: false,
        attempts: 0,
    }
}
fn setup() -> (Runtime, Arc<Sqlite>) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    (
        declarations()
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap(),
        store,
    )
}
async fn create(rom: &Runtime) {
    rom.execute(&alice(), Command::create("t", task()).idempotency("create"))
        .await
        .unwrap();
}

#[tokio::test]
async fn two_resources_share_actions_codec_and_live_membership() {
    let (rom, store) = setup();
    let a = alice();
    let mut tasks = rom
        .live(&a, Task::done_field().equals(false))
        .await
        .unwrap();
    let mut settings = rom
        .live(&a, Setting::enabled_field().equals(true))
        .await
        .unwrap();
    assert!(tasks.changed().await.unwrap().is_empty());
    assert!(settings.changed().await.unwrap().is_empty());
    create(&rom).await;
    assert_eq!(
        tasks.changed().await.unwrap()[0].value.as_ref(),
        Some(&task())
    );
    rom.execute(&a, Command::create("s", setting()).idempotency("create"))
        .await
        .unwrap();
    rom.execute(
        &a,
        Command::action("s", ENABLE, true)
            .at_revision(1)
            .idempotency("enable"),
    )
    .await
    .unwrap();
    assert_eq!(settings.changed().await.unwrap()[0].revision, 2);
    rom.execute(
        &a,
        Command::action("t", COMPLETE, ())
            .at_revision(1)
            .idempotency("complete"),
    )
    .await
    .unwrap();
    assert!(tasks.changed().await.unwrap().is_empty());
    assert_eq!(store.counts().unwrap(), [2, 4, 4, 1]);
    rom.shutdown().await.unwrap();
}
#[test]
fn descriptor_codec_rename_null_false_zero_and_unknown_input_agree() {
    let mut t = task();
    t.note = Some(String::new());
    let encoded = t.encode();
    assert_eq!(encoded["display-title"], t.title);
    assert!(encoded.get("title").is_none());
    assert_eq!(Task::decode(encoded).unwrap(), t);
    t.note = None;
    assert!(t.encode()["note"].is_null());
    assert_eq!(Setting::decode(setting().encode()).unwrap(), setting());
    let mut missing = t.encode();
    missing.as_object_mut().unwrap().remove("note");
    assert_eq!(Task::decode(missing), Err(Error::invalid("tasks", "note")));
    let mut wrong = t.encode();
    wrong["done"] = json!("false");
    assert_eq!(Task::decode(wrong), Err(Error::invalid("tasks", "done")));
    let mut unknown = t.encode();
    unknown["actor"] = json!("admin");
    assert_eq!(
        Task::decode(unknown),
        Err(Error::invalid("tasks", "unknown field"))
    );
    let d = Task::descriptor();
    assert!(
        d.fields
            .iter()
            .any(|f| f.name == "display-title" && f.shape == Shape::String)
    );
}
#[tokio::test]
async fn no_op_receipt_does_not_advance_revision_or_emit_event() {
    let (rom, store) = setup();
    create(&rom).await;
    let r = rom
        .execute(
            &alice(),
            Command::replace("t", task())
                .at_revision(1)
                .idempotency("no-op"),
        )
        .await
        .unwrap();
    assert_eq!(r.revision, 1);
    assert_eq!(store.counts().unwrap(), [1, 1, 2, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn revision_race_and_changed_input_identity_are_distinct() {
    let (rom, store) = setup();
    create(&rom).await;
    let one = Command::action("t", COMPLETE, ())
        .at_revision(1)
        .idempotency("one");
    let mut changed = task();
    changed.title = "changed".into();
    let two = Command::replace("t", changed)
        .at_revision(1)
        .idempotency("two");
    let a = alice();
    let (x, y) = tokio::join!(rom.execute(&a, one), rom.execute(&a, two));
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    assert!(matches!(x, Err(Error::Conflict)) || matches!(y, Err(Error::Conflict)));
    assert_eq!(store.counts().unwrap()[1], 2);
    let mut changed = task();
    changed.done = true;
    assert!(matches!(
        rom.execute(&a, Command::create("t", changed).idempotency("create"))
            .await,
        Err(Error::IdentityMismatch)
    ));
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn concurrent_same_identity_replays_one_durable_transition() {
    let (rom, store) = setup();
    create(&rom).await;
    let a = alice();
    let cmd = Command::action("t", COMPLETE, ())
        .at_revision(1)
        .idempotency("done");
    let (x, y) = tokio::join!(rom.execute(&a, cmd.clone()), rom.execute(&a, cmd));
    assert_eq!(x.unwrap().revision, 2);
    assert_eq!(y.unwrap().revision, 2);
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn rollback_after_each_real_sql_write_leaves_full_bundle_absent() {
    for point in 1..=4 {
        let (rom, store) = setup();
        create(&rom).await;
        store.inject_fault(point);
        let cmd = Command::action("t", COMPLETE, ())
            .at_revision(1)
            .idempotency("done");
        assert!(matches!(
            rom.execute(&alice(), cmd.clone()).await,
            Err(Error::NotCommitted)
        ));
        assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
        assert!(
            !rom.read::<Task>(&alice(), "t")
                .await
                .unwrap()
                .value
                .unwrap()
                .done
        );
        assert_eq!(rom.execute(&alice(), cmd).await.unwrap().revision, 2);
        assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
        rom.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn unknown_after_actual_commit_reopens_and_deduplicates() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "PROTOTYPE-wipe-me-integrated-{}-{}.sqlite",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let cmd = Command::action("t", COMPLETE, ())
        .at_revision(1)
        .idempotency("done");
    {
        let store = Arc::new(Sqlite::open(&path).unwrap());
        let rom = declarations()
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        create(&rom).await;
        store.inject_fault(5);
        assert!(matches!(
            rom.execute(&alice(), cmd.clone()).await,
            Err(Error::Unknown)
        ));
        assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
        rom.shutdown().await.unwrap();
    }
    {
        let store = Arc::new(Sqlite::open(&path).unwrap());
        let rom = declarations()
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        assert_eq!(rom.execute(&alice(), cmd).await.unwrap().revision, 2);
        assert_eq!(store.counts().unwrap(), [1, 2, 2, 1]);
        assert_eq!(store.intentions().unwrap().len(), 1);
        rom.shutdown().await.unwrap();
    }
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn revoke_buffered_live_signal_and_durable_replay() {
    let (rom, store) = setup();
    create(&rom).await;
    let a = alice();
    let mut live = rom
        .live(&a, Task::done_field().equals(false))
        .await
        .unwrap();
    // Initial delivery has not happened yet; policy changes before the consumer polls.
    rom.revoke(&a);
    assert!(matches!(live.changed().await, Err(Error::Denied)));
    assert!(matches!(
        rom.execute(&a, Command::create("t", task()).idempotency("create"))
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        rom.read::<Task>(&a, "t").await,
        Err(Error::Denied)
    ));
    assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn owner_change_removes_live_membership_and_denies_old_outcome() {
    let (rom, store) = setup();
    create(&rom).await;
    let a = alice();
    let mut live = rom
        .live(&a, Task::done_field().equals(false))
        .await
        .unwrap();
    assert_eq!(live.changed().await.unwrap().len(), 1);
    let mut moved = task();
    moved.owner = "bob".into();
    rom.execute(
        &Actor::trusted("local", "admin"),
        Command::replace("t", moved)
            .at_revision(1)
            .idempotency("move"),
    )
    .await
    .unwrap();
    assert!(live.changed().await.unwrap().is_empty());
    assert!(matches!(
        rom.execute(&a, Command::create("t", task()).idempotency("create"))
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn live_initialization_coalescing_delete_and_tombstone_retry() {
    let (rom, store) = setup();
    let a = alice();
    let mut live = rom
        .live(&a, Task::done_field().equals(false))
        .await
        .unwrap();
    create(&rom).await;
    assert_eq!(live.changed().await.unwrap().len(), 1);
    for i in 1..=20 {
        let mut t = task();
        t.title = format!("title {i}");
        rom.execute(
            &a,
            Command::replace("t", t)
                .at_revision(i)
                .idempotency(&format!("r{i}")),
        )
        .await
        .unwrap();
    }
    let rows = live.changed().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].revision, 21);
    let cmd = Command::<Task>::delete("t")
        .at_revision(21)
        .idempotency("delete");
    rom.execute(&a, cmd.clone()).await.unwrap();
    assert!(live.changed().await.unwrap().is_empty());
    assert!(rom.execute(&a, cmd).await.unwrap().value.is_none());
    assert_eq!(store.counts().unwrap(), [1, 22, 22, 0]);
    assert!(matches!(
        rom.execute(&a, Command::create("t", task()).idempotency("create"))
            .await,
        Err(Error::Denied)
    ));
    rom.shutdown().await.unwrap();
}

struct Gate {
    started: tokio::sync::Notify,
    open: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            started: tokio::sync::Notify::new(),
            open: Mutex::new(false),
            wake: Condvar::new(),
        })
    }
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
fn gates() -> &'static Mutex<BTreeMap<String, Arc<Gate>>> {
    static GATES: OnceLock<Mutex<BTreeMap<String, Arc<Gate>>>> = OnceLock::new();
    GATES.get_or_init(Default::default)
}
fn blocked(task: &mut Task, _: ()) -> Result<Vec<Intent>> {
    let gate = gates().lock().unwrap().get(&task.title).unwrap().clone();
    gate.started.notify_one();
    let mut open = gate.open.lock().unwrap();
    while !*open {
        open = gate.wake.wait(open).unwrap();
    }
    task.done = true;
    Ok(vec![])
}
const BLOCKED: Action<Task, ()> = Action::new("blocked", blocked);
fn blocked_runtime(capacity: usize) -> (Runtime, Arc<Sqlite>) {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .capacity(capacity)
        .resource(Task::definition().policy(task_policy).action(BLOCKED))
        .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
        .unwrap();
    (rom, store)
}
async fn blocked_task(rom: &Runtime, name: &str) -> Arc<Gate> {
    let g = Gate::new();
    gates().lock().unwrap().insert(name.into(), g.clone());
    let mut t = task();
    t.title = name.into();
    rom.execute(&alice(), Command::create("t", t).idempotency("create"))
        .await
        .unwrap();
    g
}
#[tokio::test]
async fn caller_cancellation_keeps_capacity_and_shutdown_drains_real_work() {
    let (rom, store) = blocked_runtime(1);
    let gate = blocked_task(&rom, "cancel").await;
    let r = rom.clone();
    let waiter = tokio::spawn(async move {
        r.execute(
            &alice(),
            Command::action("t", BLOCKED, ())
                .at_revision(1)
                .idempotency("blocked"),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), gate.started.notified())
        .await
        .unwrap();
    waiter.abort();
    let _ = waiter.await;
    assert_eq!(rom.available_capacity(), 0);
    assert!(matches!(
        rom.execute(
            &alice(),
            Command::action("t", BLOCKED, ())
                .at_revision(1)
                .idempotency("other")
        )
        .await,
        Err(Error::Overloaded)
    ));
    let r = rom.clone();
    let mut drain = tokio::spawn(async move { r.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut drain)
            .await
            .is_err()
    );
    assert_eq!(store.counts().unwrap()[1], 1);
    gate.release();
    drain.await.unwrap().unwrap();
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 0]);
    assert_eq!(rom.available_capacity(), 1);
    assert!(matches!(
        rom.execute(
            &alice(),
            Command::action("t", BLOCKED, ())
                .at_revision(1)
                .idempotency("blocked")
        )
        .await,
        Err(Error::Closed)
    ));
}
#[tokio::test]
async fn authority_and_revision_are_rechecked_after_cpu_proposal() {
    let (rom, store) = blocked_runtime(2);
    let gate = blocked_task(&rom, "auth-race").await;
    let r = rom.clone();
    let waiter = tokio::spawn(async move {
        r.execute(
            &alice(),
            Command::action("t", BLOCKED, ())
                .at_revision(1)
                .idempotency("blocked"),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), gate.started.notified())
        .await
        .unwrap();
    let mut moved = task();
    moved.owner = "bob".into();
    rom.execute(
        &Actor::trusted("local", "admin"),
        Command::replace("t", moved)
            .at_revision(1)
            .idempotency("move"),
    )
    .await
    .unwrap();
    gate.release();
    assert!(matches!(waiter.await.unwrap(), Err(Error::Denied)));
    assert_eq!(store.counts().unwrap(), [1, 2, 2, 0]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn pure_action_panic_releases_permit_without_committing() {
    fn panic_action(_: &mut Task, _: ()) -> Result<Vec<Intent>> {
        panic!("intentional CPU panic")
    }
    let action = Action::new("panic", panic_action);
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .capacity(1)
        .resource(Task::definition().policy(task_policy).action(action))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    create(&rom).await;
    assert!(matches!(
        rom.execute(
            &alice(),
            Command::action("t", action, ())
                .at_revision(1)
                .idempotency("panic")
        )
        .await,
        Err(Error::Panicked)
    ));
    rom.shutdown().await.unwrap();
    assert_eq!(store.counts().unwrap(), [1, 1, 1, 0]);
    assert_eq!(rom.available_capacity(), 1);
}
#[tokio::test]
async fn downstream_failure_preserves_upstream_and_recovery_retries_same_action_identity() {
    let (rom, store) = setup();
    create(&rom).await;
    rom.execute(
        &alice(),
        Command::create("s", setting()).idempotency("create-setting"),
    )
    .await
    .unwrap();
    rom.execute(
        &alice(),
        Command::action("t", COMPLETE, ())
            .at_revision(1)
            .idempotency("complete"),
    )
    .await
    .unwrap();
    let (intent_identity, _) = store.intentions().unwrap().pop().unwrap();
    let continuation = Command::action("s", ENABLE, true)
        .at_revision(1)
        .idempotency(&intent_identity);
    store.inject_fault(2);
    assert!(matches!(
        rom.execute(&alice(), continuation.clone()).await,
        Err(Error::NotCommitted)
    ));
    assert!(
        rom.read::<Task>(&alice(), "t")
            .await
            .unwrap()
            .value
            .unwrap()
            .done
    );
    assert!(
        !rom.read::<Setting>(&alice(), "s")
            .await
            .unwrap()
            .value
            .unwrap()
            .enabled
    );
    assert_eq!(
        rom.execute(&alice(), continuation.clone())
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        rom.execute(&alice(), continuation).await.unwrap().revision,
        2
    );
    assert_eq!(
        rom.read::<Setting>(&alice(), "s")
            .await
            .unwrap()
            .value
            .unwrap()
            .attempts,
        1
    );
    assert_eq!(store.counts().unwrap(), [2, 4, 4, 1]);
    rom.shutdown().await.unwrap();
}

#[derive(Clone)]
struct Manual {
    flag: bool,
}
impl Resource for Manual {
    const KIND: &'static str = "manual";
    fn descriptor() -> Descriptor {
        Descriptor {
            kind: "manual".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "flag".into(),
                shape: Shape::Bool,
            }],
        }
    }
    fn encode(&self) -> Value {
        json!({"flag":self.flag})
    }
    fn decode(v: Value) -> Result<Self> {
        Ok(Self {
            flag: v
                .get("flag")
                .and_then(Value::as_bool)
                .ok_or_else(|| Error::invalid(Self::KIND, "flag"))?,
        })
    }
}
struct Unsupported;
impl Storage for Unsupported {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: false,
            effects: false,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        unreachable!()
    }
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        unreachable!()
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        unreachable!()
    }
    fn commit(&self, _: &Bundle) -> Result<Receipt> {
        unreachable!()
    }
}
#[test]
fn derived_and_manual_definitions_share_registration_and_capability_checks() {
    for builder in [
        Runtime::builder().resource(Manual::definition()),
        Runtime::builder().resource(Task::definition()),
    ] {
        assert!(matches!(
            builder.build(Arc::new(Unsupported), Runtime::shared_cpu_pool(1).unwrap()),
            Err(Error::Unsupported(_))
        ));
    }
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    assert!(
        matches!(declarations().resource(Task::definition()).build(store,Runtime::shared_cpu_pool(1).unwrap()),Err(Error::Duplicate(k)) if k=="tasks")
    );
}
#[tokio::test]
async fn missing_policy_is_default_deny_and_authorities_are_distinct() {
    let store = Arc::new(Sqlite::open(":memory:").unwrap());
    let rom = Runtime::builder()
        .resource(Task::definition())
        .resource(Setting::definition().policy(setting_policy))
        .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
        .unwrap();
    assert!(matches!(
        rom.execute(&alice(), Command::create("t", task()).idempotency("create"))
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        rom.execute(
            &Actor::trusted("forged", "alice"),
            Command::create("s", setting()).idempotency("create")
        )
        .await,
        Err(Error::Denied)
    ));
    assert_eq!(store.counts().unwrap(), [0; 4]);
    rom.shutdown().await.unwrap();
}
#[tokio::test]
async fn negative_control_cached_payload_would_leak_after_revoke() {
    // Deliberately flawed control: storing authorized rows alone cannot enforce future policy.
    let (rom, _) = setup();
    let a = alice();
    create(&rom).await;
    let previously_authorized = rom
        .query(&a, &Task::done_field().equals(false))
        .await
        .unwrap();
    rom.revoke(&a);
    assert_eq!(
        previously_authorized[0].value.as_ref().unwrap().title,
        "private title"
    );
    assert!(
        rom.query(&a, &Task::done_field().equals(false))
            .await
            .is_err()
    );
    rom.shutdown().await.unwrap();
}
