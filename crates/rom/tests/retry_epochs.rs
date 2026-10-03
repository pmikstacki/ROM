use rom::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Resource)]
#[resource(name = "epoch-records")]
struct Record {
    value: String,
}
#[derive(Default)]
struct State {
    epochs: RetryEpochs,
    reads: usize,
    advance_at: Option<usize>,
    rows: BTreeMap<Key, Row>,
    receipts: BTreeMap<String, Receipt>,
}
#[derive(Default)]
struct Memory(Mutex<State>);
impl Storage for Memory {
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        let mut state = self.0.lock().unwrap();
        state.reads += 1;
        if state.advance_at == Some(state.reads) {
            state.epochs = epochs(1, 1, 1);
        }
        Ok(state.epochs)
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
    fn snapshot(&self, _: &str, _: usize, _: usize) -> Result<Vec<Row>> {
        Ok(self.0.lock().unwrap().rows.values().cloned().collect())
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        Ok(self.0.lock().unwrap().receipts.get(id).cloned())
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        let mut state = self.0.lock().unwrap();
        if let Some(receipt) = state.receipts.get(&bundle.receipt.identity) {
            return Ok(receipt.clone());
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
        Ok(bundle.receipt.clone())
    }
}
fn actor() -> Actor {
    Actor::trusted("epoch-tests", "owner")
}
fn builder() -> Builder {
    Runtime::builder().resource(
        Record::definition()
            .policy(|a, _, _| a.subject == "owner")
            .allow_all_fields(),
    )
}
fn runtime(store: Arc<Memory>) -> Runtime {
    builder()
        .build(store, Runtime::shared_cpu_pool(1).unwrap())
        .unwrap()
}
fn create(id: &str) -> Command<Record> {
    Command::create(
        id,
        Record {
            value: "original".into(),
        },
    )
    .idempotency("create")
}
fn epochs(current: u64, admission_floor: u64, replay_floor: u64) -> RetryEpochs {
    RetryEpochs {
        current,
        admission_floor,
        replay_floor,
    }
}

#[test]
fn epochs_validate_order_and_builder_fence_detects_rollback() {
    assert!(epochs(1, 2, 0).validate().is_err());
    assert!(epochs(2, 0, 1).validate().is_err());
    let store = Arc::new(Memory::default());
    store.0.lock().unwrap().epochs = epochs(3, 2, 1);
    for fence in [epochs(4, 2, 1), epochs(3, 3, 1), epochs(3, 2, 2)] {
        assert!(
            builder()
                .retry_fence(fence)
                .build(store.clone(), Runtime::shared_cpu_pool(1).unwrap())
                .is_err()
        );
    }
    assert!(
        builder()
            .retry_fence(epochs(2, 1, 1))
            .build(store, Runtime::shared_cpu_pool(1).unwrap())
            .is_ok()
    );
}
#[tokio::test]
async fn sealed_epochs_replay_but_never_admit_fresh_input_and_expired_receipts_fail_closed() {
    let store = Arc::new(Memory::default());
    let runtime = runtime(store.clone());
    runtime.execute(&actor(), create("one")).await.unwrap();
    store.0.lock().unwrap().epochs = epochs(1, 1, 0);
    assert_eq!(
        runtime.retry_epochs(&actor()).await.unwrap(),
        epochs(1, 1, 0)
    );
    runtime.execute(&actor(), create("one")).await.unwrap();
    let mut invalid: Invocation = create("two").into();
    invalid.operation = Operation::Create(Value::Null);
    assert_eq!(
        runtime.invoke(&actor(), invalid.clone()).await,
        Err(Error::IdentityExpired)
    );
    invalid.retry_epoch = 2;
    assert!(
        matches!(runtime.invoke(&actor(), invalid).await, Err(Error::Invalid { field, .. }) if field == "retry epoch")
    );
    store.0.lock().unwrap().epochs = epochs(1, 1, 1);
    assert_eq!(
        runtime.invoke(&actor(), create("one").into()).await,
        Err(Error::IdentityExpired)
    );
    runtime
        .execute(&actor(), create("two").retry_epoch(1))
        .await
        .unwrap();
    assert_eq!(store.0.lock().unwrap().receipts.len(), 2);
}
#[tokio::test]
async fn epochs_separate_identity_and_zero_preserves_the_legacy_identity() {
    let store = Arc::new(Memory::default());
    store.0.lock().unwrap().epochs = epochs(1, 0, 0);
    let runtime = runtime(store.clone());
    runtime.execute(&actor(), create("one")).await.unwrap();
    let legacy = json!([
        "epoch-tests",
        "embedded",
        "owner",
        Record::KIND,
        "one",
        ["standard", "create"],
        "create"
    ])
    .to_string();
    // Actor PrincipalKind serialization is part of the legacy identity contract.
    assert!(store.0.lock().unwrap().receipts.contains_key(&legacy));
    let patch = || {
        Command::<Record>::patch("one", Patch::new())
            .at_revision(1)
            .idempotency("noop")
    };
    runtime.execute(&actor(), patch()).await.unwrap();
    runtime
        .execute(&actor(), patch().retry_epoch(1))
        .await
        .unwrap();
    runtime
        .execute(&actor(), patch().retry_epoch(1))
        .await
        .unwrap();
    assert_eq!(store.0.lock().unwrap().receipts.len(), 3);
}
#[test]
fn omitted_wire_epoch_stays_zero() {
    let original = json!({"kind":Record::KIND,"id":"one","expected":null,"idempotency":"create","operation":{"type":"create","input":{"value":"original"}}});
    let invocation: Invocation = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(invocation.retry_epoch, 0);
    assert_eq!(serde_json::to_value(&invocation).unwrap(), original);
}

#[tokio::test]
async fn floors_are_rechecked_after_normalization_and_after_proposal_work() {
    for advance_at in [2, 3] {
        let store = Arc::new(Memory::default());
        let runtime = runtime(store.clone());
        {
            let mut state = store.0.lock().unwrap();
            state.reads = 0;
            state.advance_at = Some(advance_at);
        }
        assert_eq!(
            runtime.invoke(&actor(), create("one").into()).await,
            Err(Error::IdentityExpired)
        );
        assert!(store.0.lock().unwrap().rows.is_empty());
        assert!(store.0.lock().unwrap().receipts.is_empty());
    }
}

#[tokio::test]
async fn epoch_query_checks_current_actor_authority() {
    let runtime = runtime(Arc::new(Memory::default()));
    runtime.revoke(&actor());
    assert_eq!(runtime.retry_epochs(&actor()).await, Err(Error::Denied));
}
