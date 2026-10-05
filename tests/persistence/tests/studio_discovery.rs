//! Discovery is metadata-only and preserves existing native catalog identity.
use rom::*;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Debug, PartialEq)]
struct Money(String);
impl Field for Money {
    fn shape() -> Shape {
        Shape::String
    }
    fn codec_identity() -> Option<CodecIdentity> {
        Some(CodecIdentity {
            name: "money".into(),
            version: 1,
        })
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_str()
            .map(|s| Self(s.into()))
            .ok_or_else(|| Error::invalid("money", "$"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "studio-items")]
struct Item {
    amount: Money,
    enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "studio-targets")]
struct Target {
    enabled: bool,
}
#[derive(Clone, Input)]
struct Named {
    #[input(rename = "wire-name")]
    amount: Money,
    value: Option<u64>,
}
#[derive(Clone, Input)]
struct Linked {
    targets: Vec<BTreeMap<String, Option<ResourceRef<Target>>>>,
}
#[derive(Clone)]
struct Opaque;
impl Input for Opaque {
    fn encode(&self) -> Value {
        json!({"private": true})
    }
    fn decode(_: Value) -> Result<Self> {
        Ok(Self)
    }
}
const UNIT: Action<Item, ()> = Action::new("unit", |_, _| Ok(vec![]));
const SCALAR: Action<Item, Money> = Action::new("scalar", |_, _| Ok(vec![]));
const OBJECT: Action<Item, Named> = Action::new("object", |_, _| Ok(vec![]));
const LINKS: Action<Item, Linked> = Action::new("links", |_, _| Ok(vec![]));
const OPAQUE: Action<Item, Opaque> = Action::new("opaque", |_, _| Ok(vec![]));
fn definition() -> Definition<Item> {
    Item::definition()
        .action(UNIT)
        .action(SCALAR)
        .action(OBJECT)
        .action(LINKS)
        .action(OPAQUE)
        .discovery_policy(|actor, target| {
            actor.subject == "reader" && !matches!(target, DiscoveryTarget::Action("unit"))
        })
}
fn build(storage: Arc<dyn Storage>, target_visible: bool, limit: usize) -> Result<Runtime> {
    Runtime::builder()
        .limits(Limits {
            snapshot_bytes: limit,
            ..Limits::default()
        })
        .resource(definition())
        .resource(Target::definition().discovery_policy(move |_, _| target_visible))
        .build(storage, Runtime::shared_cpu_pool(1)?)
}
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-studio-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn open(&self, redb: bool) -> Arc<dyn Storage> {
        if redb {
            Arc::new(rom_redb::Redb::open(self.0.join("db")).unwrap())
        } else {
            Arc::new(rom_sqlite::Sqlite::open(self.0.join("db")).unwrap())
        }
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn authorized_metadata_preserves_codec_bindings_and_hides_nested_targets() {
    for redb in [false, true] {
        let files = Files::new();
        let runtime = build(files.open(redb), false, 100_000).unwrap();
        let actor = Actor::trusted("test", "reader");
        let discovery = runtime.discover(&actor).await.unwrap();
        assert_eq!(discovery.version, 1);
        let item = &discovery.resources[0];
        assert_eq!(item.actions, ["links", "object", "opaque", "scalar"]);
        assert_eq!(
            item.action_inputs
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            ["object", "opaque", "scalar"]
        );
        assert_eq!(item.fields[0].codec, Money::codec_identity());
        assert_eq!(item.action_inputs[0].input, Named::descriptor());
        assert_eq!(item.action_inputs[1].input, None);
        assert_eq!(item.action_inputs[2].input, Money::descriptor());
        assert!(item.action_inputs.iter().all(|a| a.version == 1));
        let bytes = serde_json::to_string(&discovery).unwrap();
        assert!(!bytes.contains("studio-targets"));
        assert!(matches!(
            runtime
                .execute(
                    &actor,
                    Command::create(
                        "one",
                        Item {
                            amount: Money("1".into()),
                            enabled: false
                        }
                    )
                    .idempotency("create")
                )
                .await,
            Err(Error::Denied)
        ));
        assert!(
            runtime
                .discover(&Actor::trusted("test", "other"))
                .await
                .unwrap()
                .resources
                .is_empty()
        );
        runtime.shutdown().await.unwrap();
        drop(runtime);
        let visible = build(files.open(redb), true, 100_000).unwrap();
        let disclosed = visible.discover(&actor).await.unwrap();
        assert_eq!(
            disclosed.resources[0].action_inputs[0].input,
            Linked::descriptor()
        );
        let encoded = serde_json::to_vec(&disclosed).unwrap();
        visible.shutdown().await.unwrap();
        drop(visible);
        let exact = build(files.open(redb), true, encoded.len()).unwrap();
        assert_eq!(
            serde_json::to_vec(&exact.discover(&actor).await.unwrap()).unwrap(),
            encoded
        );
        exact.shutdown().await.unwrap();
        drop(exact);
        let short = build(files.open(redb), true, encoded.len() - 1).unwrap();
        assert!(matches!(short.discover(&actor).await, Err(Error::TooLarge)));
        short.revoke(&actor);
        assert!(matches!(short.discover(&actor).await, Err(Error::Denied)));
        short.shutdown().await.unwrap();
    }
}

#[derive(Clone)]
struct Legacy(Item);
impl Resource for Legacy {
    const KIND: &'static str = Item::KIND;
    fn descriptor() -> Descriptor {
        Item::descriptor()
    }
    fn normalize_field(name: &str, value: Value) -> Result<Value> {
        Item::normalize_field(name, value)
    }
    fn encode(&self) -> Value {
        self.0.encode()
    }
    fn decode(value: Value) -> Result<Self> {
        Item::decode(value).map(Self)
    }
}
#[tokio::test]
async fn enabling_presentation_metadata_does_not_change_native_catalog_or_values() {
    for redb in [false, true] {
        let files = Files::new();
        let actor = Actor::trusted("test", "owner");
        let old = Runtime::builder()
            .resource(
                Legacy::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields(),
            )
            .build(files.open(redb), Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
        old.execute(
            &actor,
            Command::create(
                "one",
                Legacy(Item {
                    amount: Money("9007199254740993".into()),
                    enabled: false,
                }),
            )
            .idempotency("create"),
        )
        .await
        .unwrap();
        old.shutdown().await.unwrap();
        drop(old);
        let storage = files.open(redb);
        let new = Runtime::builder()
            .resource(
                Item::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields()
                    .discovery_policy(|_, _| true),
            )
            .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
            .unwrap();
        assert_eq!(
            new.read::<Item>(&actor, "one")
                .await
                .unwrap()
                .value
                .unwrap()
                .amount,
            Money("9007199254740993".into())
        );
        assert_eq!(
            new.discover(&actor).await.unwrap().resources[0].fields[0].codec,
            Money::codec_identity()
        );
        assert_eq!(Item::descriptor(), Legacy::descriptor());
        let head = storage.journal_head(Item::KIND).unwrap();
        let rows = storage.snapshot(Item::KIND, 100, 100_000).unwrap();
        let replay = new
            .execute(
                &actor,
                Command::create(
                    "one",
                    Item {
                        amount: Money("9007199254740993".into()),
                        enabled: false,
                    },
                )
                .idempotency("create"),
            )
            .await
            .unwrap();
        assert_eq!(replay.revision, 1);
        assert_eq!(storage.journal_head(Item::KIND).unwrap(), head);
        assert_eq!(storage.snapshot(Item::KIND, 100, 100_000).unwrap(), rows);
        new.shutdown().await.unwrap();
    }
}

#[derive(Clone)]
struct InvalidInput;
impl Input for InvalidInput {
    fn descriptor() -> Option<InputDescriptor> {
        Some(InputDescriptor::Object(vec![InputFieldDescriptor {
            enum_labels: Default::default(),
            codec_wrappers: vec![],
            name: "".into(),
            shape: Shape::String,
            codec: None,
        }]))
    }
    fn encode(&self) -> Value {
        Value::Null
    }
    fn decode(_: Value) -> Result<Self> {
        Ok(Self)
    }
}
#[test]
fn invalid_input_metadata_and_unregistered_targets_fail_before_runtime() {
    let files = Files::new();
    let storage = files.open(false);
    let bad = Action::new("bad", |_: &mut Item, _: InvalidInput| Ok(vec![]));
    assert!(
        Runtime::builder()
            .resource(Item::definition().action(bad))
            .build(storage.clone(), Runtime::shared_cpu_pool(1).unwrap())
            .is_err()
    );
    assert!(
        Runtime::builder()
            .resource(Item::definition().action(LINKS))
            .build(storage, Runtime::shared_cpu_pool(1).unwrap())
            .is_err()
    );
}
