use rom::{
    Action, Actor, ActorGate, AuthorizationRead, Builder, Bundle, Capabilities, Command,
    DiscoveryTarget, Error, Field, Key, Limits, Receipt, Resource, ResourceRef, Result, Row,
    Runtime, Shape, Storage, Value, json,
};
use rom_consumer::{COMPLETE, Task};
use rom_sqlite::Sqlite;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

#[derive(Clone)]
struct NoDecode;
impl Field for NoDecode {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        panic!("discovery encoded a field")
    }
    fn decode(_: Value) -> Result<Self> {
        panic!("discovery decoded a field")
    }
}
#[derive(Clone, Resource)]
#[resource(name = "hidden-kind")]
struct Hidden {
    hidden_value: NoDecode,
}
#[derive(Clone, Resource)]
#[resource(name = "public-kind")]
struct Public {
    zeta: NoDecode,
    alpha: NoDecode,
    hidden_field: NoDecode,
    direct: ResourceRef<Hidden>,
    nested: Option<Vec<BTreeMap<String, ResourceRef<Hidden>>>>,
    visible_reference: ResourceRef<Public>,
}
const SAFE: Action<Public, ()> = Action::new("safe", |_, ()| panic!("discovery executed action"));
const HIDDEN_ACTION: Action<Public, ()> =
    Action::new("hidden-action", |_, ()| panic!("discovery executed action"));

// Independent review probe: accounting across multiple metadata arrays.
#[tokio::test]
async fn reviewer_catalog_budget_matches_wire_for_every_smaller_limit() {
    fn definitions() -> Builder {
        Runtime::builder()
            .resource(
                Public::definition()
                    .action(SAFE)
                    .action(HIDDEN_ACTION)
                    .discovery_policy(|_, _| true),
            )
            .resource(Hidden::definition().discovery_policy(|_, _| true))
    }
    let baseline = build(definitions());
    let catalog = baseline.discover(&actor()).await.unwrap();
    let wire_bytes = serde_json::to_vec(&catalog).unwrap().len();
    baseline.shutdown().await.unwrap();
    for limit in 1..=wire_bytes + 1 {
        let runtime = build(definitions().limits(Limits {
            snapshot_bytes: limit,
            ..Limits::default()
        }));
        let result = runtime.discover(&actor()).await;
        if limit < wire_bytes {
            assert!(
                matches!(result, Err(Error::TooLarge)),
                "accepted limit {limit} below {wire_bytes}"
            );
        } else {
            assert_eq!(result.unwrap(), catalog);
        }
        runtime.shutdown().await.unwrap();
    }
}

#[derive(Default)]
struct NoStorageReads(rom::StorageOwnership);
impl Storage for NoStorageReads {
    fn acquire_owner(&self) -> Result<rom::StorageOwner> {
        self.0.acquire()
    }
    // This fixture tests metadata-only discovery, not durable storage registration.
    fn register(&self, _: &[rom::Descriptor]) -> Result<()> {
        Ok(())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, _: &Key) -> Result<Option<Row>> {
        panic!("discovery loaded a row")
    }
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        panic!("discovery scanned rows")
    }
    fn receipt(&self, _: &str) -> Result<Option<Receipt>> {
        panic!("discovery loaded receipt")
    }
    fn commit(&self, _: &Bundle) -> Result<Receipt> {
        panic!("discovery committed")
    }
}
fn actor() -> Actor {
    Actor::trusted("test", "viewer")
}
fn build(builder: Builder) -> Runtime {
    builder
        .build(
            Arc::new(NoStorageReads::default()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap()
}
fn catalog() -> Builder {
    Runtime::builder().resource(Hidden::definition()).resource(
        Public::definition()
            .policy(|_, _, _| panic!("discovery evaluated row policy"))
            .field_policy(|_, _, _, _| panic!("discovery evaluated field policy"))
            .query_policy(|_, _| panic!("discovery evaluated query policy"))
            .source_owner("private-source-name")
            .action(HIDDEN_ACTION)
            .action(SAFE)
            .discovery_policy(|_, target| match target {
                DiscoveryTarget::Resource => true,
                DiscoveryTarget::Field(name) => name != "hidden_field",
                DiscoveryTarget::Action(name) => name == "safe",
            }),
    )
}
#[tokio::test]
async fn discovery_denies_by_default_and_requires_resource_grant() {
    let runtime = build(
        Runtime::builder().resource(Hidden::definition()).resource(
            Public::definition()
                .action(SAFE)
                .discovery_policy(|_, target| !matches!(target, DiscoveryTarget::Resource)),
        ),
    );
    assert_eq!(
        serde_json::to_value(runtime.discover(&actor()).await.unwrap()).unwrap(),
        json!({"version":1,"resources":[]})
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn catalog_omits_secrets_and_nested_hidden_references_without_running_domain_code() {
    let runtime = build(catalog());
    let discovery = runtime.discover(&actor()).await.unwrap();
    assert_eq!(
        serde_json::to_value(discovery).unwrap(),
        json!({"version":1,"resources":[{
            "kind":"public-kind","version":1,
            "fields":[
                {"name":"alpha","shape":{"type":"string"}},
                {"name":"visible_reference","shape":{"type":"reference","value":{"kind":"public-kind"}}},
                {"name":"zeta","shape":{"type":"string"}}
            ],"actions":["safe"],
            "action_inputs":[{"name":"safe","version":1,"input":{"type":"unit"}}]
        }]})
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn catalog_sorts_resources_fields_and_actions_and_serializes_recursive_shapes() {
    let runtime = build(
        Runtime::builder()
            .resource(
                Public::definition()
                    .action(SAFE)
                    .action(HIDDEN_ACTION)
                    .discovery_policy(|_, _| true),
            )
            .resource(Hidden::definition().discovery_policy(|_, _| true)),
    );
    let value = serde_json::to_value(runtime.discover(&actor()).await.unwrap()).unwrap();
    assert_eq!(value["resources"][0]["kind"], "hidden-kind");
    let public = &value["resources"][1];
    assert_eq!(public["actions"], json!(["hidden-action", "safe"]));
    assert_eq!(
        public["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "alpha",
            "direct",
            "hidden_field",
            "nested",
            "visible_reference",
            "zeta"
        ]
    );
    assert_eq!(
        public["fields"][3]["shape"],
        json!({"type":"nullable","value":{"type":"list","value":{"type":"map","value":{"type":"reference","value":{"kind":"hidden-kind"}}}}})
    );
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn metadata_visibility_does_not_grant_mutation_authority() {
    let runtime = Runtime::builder()
        .resource(
            Task::definition()
                .policy(|a, _, _| a.subject == "admin")
                .allow_all_fields()
                .action(COMPLETE)
                .discovery_policy(|_, _| true),
        )
        .build(
            Arc::new(Sqlite::open(":memory:").unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    runtime
        .execute(
            &Actor::trusted("test", "admin"),
            Command::create(
                "one",
                Task {
                    owner: "owner".into(),
                    title: "sensitive".into(),
                    done: false,
                    note: None,
                },
            )
            .idempotency("create"),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime.discover(&actor()).await.unwrap().resources[0].actions,
        ["complete"]
    );
    assert!(matches!(
        runtime
            .execute(
                &actor(),
                Command::action("one", COMPLETE, ())
                    .at_revision(1)
                    .idempotency("attempt")
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        runtime.read::<Task>(&actor(), "one").await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
struct CurrentGate(Arc<AtomicBool>);
impl ActorGate for CurrentGate {
    fn check(&self, _: &Actor, _: &mut dyn AuthorizationRead) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
}
#[tokio::test]
async fn current_authority_denial_precedes_metadata_callbacks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let runtime = build(
        Runtime::builder()
            .actor_gate(Arc::new(CurrentGate(Arc::new(AtomicBool::new(false)))))
            .resource(Hidden::definition().discovery_policy(move |_, _| {
                count.fetch_add(1, Ordering::SeqCst);
                true
            })),
    );
    assert!(matches!(
        runtime.discover(&actor()).await,
        Err(Error::Denied)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn authority_changed_during_metadata_work_denies_the_catalog() {
    let allowed = Arc::new(AtomicBool::new(true));
    let control = allowed.clone();
    let runtime = build(
        Runtime::builder()
            .actor_gate(Arc::new(CurrentGate(allowed)))
            .resource(Hidden::definition().discovery_policy(move |_, _| {
                control.store(false, Ordering::SeqCst);
                true
            })),
    );
    assert!(matches!(
        runtime.discover(&actor()).await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
struct Clock(Arc<AtomicU64>);
impl rom::Clock for Clock {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[tokio::test]
async fn expiry_during_discovery_does_not_return_metadata() {
    let now = Arc::new(AtomicU64::new(1));
    let control = now.clone();
    let runtime = build(Runtime::builder().clock(Arc::new(Clock(now))).resource(
        Hidden::definition().discovery_policy(move |_, _| {
            control.store(2, Ordering::SeqCst);
            true
        }),
    ));
    assert!(matches!(
        runtime.discover(&actor().expires_at(2)).await,
        Err(Error::Denied)
    ));
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn revocation_while_callback_is_blocked_prevents_disclosure() {
    let started = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let signal = started.clone();
    let wait = gate.clone();
    let runtime = build(
        Runtime::builder().resource(Hidden::definition().discovery_policy(move |_, target| {
            if matches!(target, DiscoveryTarget::Resource) {
                signal.notify_one();
                let (lock, wake) = &*wait;
                let mut open = lock.lock().unwrap();
                while !*open {
                    open = wake.wait(open).unwrap();
                }
            }
            true
        })),
    );
    let running = runtime.clone();
    let discovery = tokio::spawn(async move { running.discover(&actor()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), started.notified())
        .await
        .unwrap();
    runtime.revoke(&actor());
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    assert!(matches!(discovery.await.unwrap(), Err(Error::Denied)));
    runtime.shutdown().await.unwrap();
}
#[tokio::test]
async fn callback_panic_is_supervised_and_terminal() {
    let runtime = build(
        Runtime::builder()
            .resource(Hidden::definition().discovery_policy(|_, _| panic!("metadata callback"))),
    );
    assert!(matches!(
        runtime.discover(&actor()).await,
        Err(Error::Panicked)
    ));
    assert!(runtime.status().unwrap().failed);
    assert!(matches!(runtime.shutdown().await, Err(Error::Panicked)));
}
#[tokio::test]
async fn discovery_byte_budget_counts_the_complete_wire_envelope() {
    let runtime = build(catalog());
    let bytes = serde_json::to_vec(&runtime.discover(&actor()).await.unwrap())
        .unwrap()
        .len();
    runtime.shutdown().await.unwrap();
    let exact = build(catalog().limits(Limits {
        snapshot_bytes: bytes,
        ..Limits::default()
    }));
    assert_eq!(
        serde_json::to_vec(&exact.discover(&actor()).await.unwrap())
            .unwrap()
            .len(),
        bytes
    );
    exact.shutdown().await.unwrap();
    let short = build(catalog().limits(Limits {
        snapshot_bytes: bytes - 1,
        ..Limits::default()
    }));
    assert!(matches!(
        short.discover(&actor()).await,
        Err(Error::TooLarge)
    ));
    short.shutdown().await.unwrap();
    let empty = build(Runtime::builder().limits(Limits {
        snapshot_bytes: 1,
        ..Limits::default()
    }));
    assert!(matches!(
        empty.discover(&actor()).await,
        Err(Error::TooLarge)
    ));
    empty.shutdown().await.unwrap();
}
#[derive(Clone)]
struct Large;
impl Field for Large {
    fn shape() -> Shape {
        Shape::Enum(vec!["x".repeat(100_000)])
    }
    fn encode(&self) -> Value {
        panic!("encode")
    }
    fn decode(_: Value) -> Result<Self> {
        panic!("decode")
    }
}
#[derive(Clone, Resource)]
#[resource(name = "large")]
struct LargeResource {
    huge: Large,
}
#[tokio::test]
async fn discovery_stops_at_exhaustion_before_later_metadata_callbacks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let runtime = build(
        Runtime::builder()
            .limits(Limits {
                snapshot_bytes: 128,
                ..Limits::default()
            })
            .resource(
                LargeResource::definition()
                    .action(Action::new("later", |_, ()| Ok(vec![])))
                    .discovery_policy(move |_, target| {
                        if matches!(target, DiscoveryTarget::Action(_)) {
                            count.fetch_add(1, Ordering::SeqCst);
                        }
                        true
                    }),
            ),
    );
    assert!(matches!(
        runtime.discover(&actor()).await,
        Err(Error::TooLarge)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    runtime.shutdown().await.unwrap();
}
