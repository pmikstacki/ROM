use rom::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

static OLD_CODEC_CALLS: AtomicUsize = AtomicUsize::new(0);
fn uppercase(value: &str) -> String {
    value.to_uppercase()
}
fn observed(value: &str) -> String {
    OLD_CODEC_CALLS.fetch_add(1, Ordering::SeqCst);
    uppercase(value)
}
fn unchanged(value: &str) -> String {
    value.into()
}
macro_rules! record {
    ($name:ident, $kind:literal, $version:literal, $field:literal, $normalize:path) => {
        #[derive(Clone)]
        struct $name { owner: String, value: String }
        impl Resource for $name {
            const KIND: &'static str = $kind;
            fn descriptor() -> Descriptor {
                Descriptor { kind: Self::KIND.into(), version: $version, fields: vec![
                    FieldDescriptor { name: "owner".into(), shape: Shape::String },
                    FieldDescriptor { name: $field.into(), shape: Shape::String },
                ] }
            }
            fn normalize_field(name: &str, value: Value) -> Result<Value> {
                let text = value.as_str().ok_or_else(|| Error::invalid(Self::KIND, name))?;
                match name {
                    "owner" => Ok(json!(text)),
                    $field => Ok(json!($normalize(text))),
                    _ => Err(Error::invalid(Self::KIND, name)),
                }
            }
            fn encode(&self) -> Value { json!({"owner": self.owner, $field: self.value}) }
            fn decode(value: Value) -> Result<Self> {
                let map = value.as_object().filter(|m| m.len() == 2)
                    .ok_or_else(|| Error::invalid(Self::KIND, "object"))?;
                let owner = map.get("owner").and_then(Value::as_str)
                    .ok_or_else(|| Error::invalid(Self::KIND, "owner"))?;
                let text = map.get($field).and_then(Value::as_str)
                    .ok_or_else(|| Error::invalid(Self::KIND, $field))?;
                Ok(Self { owner: owner.into(), value: $normalize(text) })
            }
        }
    }
}
record!(Old, "replay-items", 1, "name", uppercase);
record!(ObservedOld, "replay-items", 1, "name", observed);
record!(OldSameName, "replay-items", 1, "title", uppercase);
record!(Current, "replay-items", 2, "title", unchanged);
record!(OtherKind, "other-items", 1, "title", unchanged);
record!(Future, "replay-items", 3, "title", unchanged);

#[derive(Default)]
struct Stored {
    rows: BTreeMap<Key, Row>,
    receipts: BTreeMap<String, Receipt>,
    commits: usize,
}
/// Runtime-only fixture: storage migrations are covered by the native adapter suites.
#[derive(Default)]
struct Memory(Mutex<Stored>, StorageOwnership);
impl Memory {
    fn rename(&self) {
        fn row(row: &mut Row) {
            if let Some(value) = &mut row.value {
                let map = value.as_object_mut().unwrap();
                if let Some(value) = map.remove("name") {
                    map.insert("title".into(), value);
                }
            }
        }
        let mut state = self.0.lock().unwrap();
        for value in state.rows.values_mut() {
            row(value);
        }
        for receipt in state.receipts.values_mut() {
            row(&mut receipt.row);
        }
    }
}
impl Storage for Memory {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.1.acquire()
    }
    fn register(&self, _: &[Descriptor]) -> Result<()> {
        Ok(())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        Ok(self.0.lock().unwrap().rows.get(key).cloned())
    }
    fn snapshot(&self, kind: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .rows
            .values()
            .filter(|r| r.key.kind == kind)
            .cloned()
            .collect())
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        Ok(self.0.lock().unwrap().receipts.get(id).cloned())
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        let mut state = self.0.lock().unwrap();
        if let Some(receipt) = state.receipts.get(&bundle.receipt.identity) {
            return if receipt.fingerprint == bundle.receipt.fingerprint {
                Ok(receipt.clone())
            } else {
                Err(Error::IdentityMismatch)
            };
        }
        if state.rows.get(&bundle.receipt.row.key).map(|r| r.revision) != bundle.expected {
            return Err(Error::Conflict);
        }
        state
            .rows
            .insert(bundle.receipt.row.key.clone(), bundle.receipt.row.clone());
        state
            .receipts
            .insert(bundle.receipt.identity.clone(), bundle.receipt.clone());
        state.commits += 1;
        Ok(bundle.receipt.clone())
    }
}
fn actor() -> Actor {
    Actor::trusted("test", "owner")
}
fn create(input: Value) -> Invocation {
    Invocation {
        retry_epoch: 0,
        kind: Current::KIND.into(),
        id: "one".into(),
        expected: None,
        idempotency: "create".into(),
        operation: Operation::Create(input),
    }
}
fn runtime<R: Resource>(store: Arc<Memory>, definition: Definition<R>) -> Runtime {
    Runtime::builder()
        .resource(definition)
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
async fn seed<R: Resource>(input: Value) -> Arc<Memory> {
    let store = Arc::new(Memory::default());
    let runtime = runtime(
        store.clone(),
        R::definition().policy(|_, _, _| true).allow_all_fields(),
    );
    runtime.invoke(&actor(), create(input)).await.unwrap();
    runtime.shutdown().await.unwrap();
    store
}
fn current() -> Definition<Current> {
    Current::definition()
        .policy(|actor, _, value| actor.subject == value.owner)
        .allow_all_fields()
}

#[tokio::test]
async fn renamed_create_replays_original_fingerprint_and_never_admits_legacy_fresh_writes() {
    let original = json!({"owner":"owner", "name":"mixed"});
    let store = seed::<Old>(original.clone()).await;
    assert_eq!(
        store
            .0
            .lock()
            .unwrap()
            .receipts
            .values()
            .next()
            .unwrap()
            .replay_version,
        Some(1)
    );
    store.rename();
    let runtime = runtime(store.clone(), current().replay_from::<Old>());
    let replay = runtime.invoke(&actor(), create(original)).await.unwrap();
    assert_eq!(replay.revision, 1);
    assert_eq!(
        replay.value.unwrap(),
        json!({"owner":"owner", "title":"MIXED"})
    );
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "name":"other"})))
            .await,
        Err(Error::IdentityMismatch)
    );
    let mut fresh = create(json!({"owner":"owner", "name":"mixed"}));
    fresh.id = "fresh".into();
    assert!(matches!(
        runtime.invoke(&actor(), fresh).await,
        Err(Error::Invalid { .. })
    ));
    assert_eq!(store.0.lock().unwrap().commits, 1);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn current_receipt_never_tries_a_legacy_codec_that_would_match() {
    let store = seed::<Current>(json!({"owner":"owner", "title":"UPPER"})).await;
    assert_eq!(
        store
            .0
            .lock()
            .unwrap()
            .receipts
            .values()
            .next()
            .unwrap()
            .replay_version,
        Some(2)
    );
    let runtime = runtime(store.clone(), current().replay_from::<OldSameName>());
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "title":"upper"})))
            .await,
        Err(Error::IdentityMismatch)
    );
    assert_eq!(store.0.lock().unwrap().commits, 1);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn exact_legacy_codec_is_used_even_when_the_current_codec_accepts_input() {
    let store = seed::<OldSameName>(json!({"owner":"owner", "title":"upper"})).await;
    let runtime = runtime(store, current().replay_from::<OldSameName>());
    let replay = runtime
        .invoke(&actor(), create(json!({"owner":"owner", "title":"upper"})))
        .await
        .unwrap();
    assert_eq!(replay.value.unwrap()["title"], "UPPER");
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn unavailable_legacy_codec_fails_closed_without_reexecution() {
    let store = seed::<Old>(json!({"owner":"owner", "name":"mixed"})).await;
    store.rename();
    let runtime = runtime(store.clone(), current());
    assert!(matches!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "name":"mixed"})))
            .await,
        Err(Error::Unsupported(_))
    ));
    assert_eq!(store.0.lock().unwrap().commits, 1);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn current_authority_denial_precedes_legacy_normalization() {
    let store = seed::<ObservedOld>(json!({"owner":"owner", "name":"mixed"})).await;
    store.rename();
    let runtime = runtime(
        store.clone(),
        Current::definition()
            .policy(|_, _, _| false)
            .allow_all_fields()
            .replay_from::<ObservedOld>(),
    );
    let before = OLD_CODEC_CALLS.load(Ordering::SeqCst);
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "name":"mixed"})))
            .await,
        Err(Error::Denied)
    );
    assert_eq!(OLD_CODEC_CALLS.load(Ordering::SeqCst), before);
    runtime.shutdown().await.unwrap();
}

#[test]
fn replay_registration_rejects_wrong_kind_current_future_and_duplicate_versions() {
    for definition in [
        current().replay_from::<OtherKind>(),
        current().replay_from::<Current>(),
        current().replay_from::<Future>(),
        current().replay_from::<Old>().replay_from::<OldSameName>(),
    ] {
        assert!(
            Runtime::builder()
                .resource(definition)
                .build(
                    Arc::new(Memory::default()),
                    Runtime::shared_cpu_pool(1).unwrap()
                )
                .is_err()
        );
    }
}

#[tokio::test]
async fn renamed_patch_replays_with_the_original_field_codec() {
    let store = seed::<Old>(json!({"owner":"owner", "name":"mixed"})).await;
    let old_runtime = runtime(
        store.clone(),
        Old::definition().policy(|_, _, _| true).allow_all_fields(),
    );
    let patch = Invocation {
        retry_epoch: 0,
        kind: Current::KIND.into(),
        id: "one".into(),
        expected: Some(1),
        idempotency: "patch".into(),
        operation: Operation::Patch(BTreeMap::from([(
            "name".into(),
            FieldUpdate::Set(json!("changed")),
        )])),
    };
    old_runtime.invoke(&actor(), patch.clone()).await.unwrap();
    old_runtime.shutdown().await.unwrap();
    drop(old_runtime);
    store.rename();
    let runtime = runtime(store.clone(), current().replay_from::<Old>());
    let replay = runtime.invoke(&actor(), patch.clone()).await.unwrap();
    assert_eq!(replay.revision, 2);
    assert_eq!(replay.value.unwrap()["title"], "CHANGED");
    let mut changed = patch;
    changed.operation = Operation::Patch(BTreeMap::from([(
        "name".into(),
        FieldUpdate::Set(json!("different")),
    )]));
    assert_eq!(
        runtime.invoke(&actor(), changed).await,
        Err(Error::IdentityMismatch)
    );
    assert_eq!(store.0.lock().unwrap().commits, 2);
    runtime.shutdown().await.unwrap();
}

fn panic_on_trigger(value: &str) -> String {
    assert_ne!(value, "panic", "legacy codec failed");
    uppercase(value)
}
record!(PanickingOld, "replay-items", 1, "name", panic_on_trigger);

#[tokio::test]
async fn legacy_codec_panic_does_not_poison_the_runtime_gate() {
    let store = seed::<Old>(json!({"owner":"owner", "name":"mixed"})).await;
    store.rename();
    let runtime = runtime(store, current().replay_from::<PanickingOld>());
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "name":"panic"})))
            .await,
        Err(Error::Panicked)
    );
    assert!(!runtime.status().unwrap().failed);
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "name":"mixed"})))
            .await
            .unwrap()
            .revision,
        1
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn unmarked_unmigrated_receipt_uses_the_current_catalog_version() {
    let store = seed::<Current>(json!({"owner":"owner", "title":"mixed"})).await;
    for receipt in store.0.lock().unwrap().receipts.values_mut() {
        receipt.replay_version = None;
    }
    let runtime = runtime(store, current().replay_from::<OldSameName>());
    assert_eq!(
        runtime
            .invoke(&actor(), create(json!({"owner":"owner", "title":"mixed"})))
            .await
            .unwrap()
            .value
            .unwrap()["title"],
        "mixed"
    );
    runtime.shutdown().await.unwrap();
}
