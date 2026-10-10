//! Authority guarantees and measured callback cost of a projected 50-row query.
//! Removing per-row/final authority checks must not disclose after external changes.
use super::Runtime;
use super::overload_fixture::{MemoryStore, Record, actor, key};
use crate::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

const BOUND: Duration = Duration::from_secs(4);

struct Time(AtomicU64);
impl Clock for Time {
    fn now(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct Store {
    inner: Arc<MemoryStore>,
    clock: Arc<Time>,
    reads: AtomicUsize,
    snapshots: AtomicUsize,
    authority_current: AtomicBool,
    invalidate_after_reads: usize,
    expire_during_snapshot: bool,
}
impl Storage for Store {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.inner.acquire_owner()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.inner.register(descriptors)
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn load(&self, requested: &Key) -> Result<Option<Row>> {
        let count = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        let mut row = self.inner.load(requested)?;
        if requested == &key("authority")
            && !self.authority_current.load(Ordering::SeqCst)
            && let Some(row) = &mut row
        {
            row.revision = 2;
        }
        // The just-read value remains valid; the next authoritative read sees
        // an external change, without a Runtime generation notification.
        if count == self.invalidate_after_reads {
            self.authority_current.store(false, Ordering::SeqCst);
        }
        Ok(row)
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        if kind != Record::KIND {
            return Err(Error::Unregistered);
        }
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        let rows: Vec<_> = (0..50)
            .map(|index| Row {
                key: key(&format!("page-{index:02}")),
                revision: 1,
                value: Some(Record { writable: true }.encode()),
                protected: ProtectedMetadata::default(),
            })
            .collect();
        if rows.len() > max_rows
            || serde_json::to_vec(&rows).map_err(|_| Error::Storage)?.len() > max_bytes
        {
            return Err(Error::TooLarge);
        }
        if self.expire_during_snapshot {
            self.clock.0.store(100, Ordering::SeqCst);
        }
        Ok(rows)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        self.inner.receipt(identity)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.inner.commit(bundle)
    }
}

struct Gate(AtomicUsize);
impl ActorGate for Gate {
    fn check(&self, actor: &Actor, read: &mut dyn AuthorizationRead) -> Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let authority = read.load(&key("authority"))?.ok_or(Error::Denied)?;
        let session = read.load(&key("row-0"))?.ok_or(Error::Denied)?;
        if actor.authority != "overload-unit"
            || actor.subject != "owner"
            || authority.revision != 1
            || session.revision != 1
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}

struct Fixture {
    runtime: Runtime,
    store: Arc<Store>,
    gate: Arc<Gate>,
}
impl Fixture {
    fn new(invalidate_after_reads: usize, expire_during_snapshot: bool) -> Self {
        let clock = Arc::new(Time(AtomicU64::new(20)));
        let store = Arc::new(Store {
            inner: MemoryStore::seeded(),
            clock: clock.clone(),
            reads: AtomicUsize::new(0),
            snapshots: AtomicUsize::new(0),
            authority_current: AtomicBool::new(true),
            invalidate_after_reads,
            expire_during_snapshot,
        });
        let gate = Arc::new(Gate(AtomicUsize::new(0)));
        let runtime = Runtime::builder()
            .clock(clock)
            .actor_gate(gate.clone())
            .resource(
                Record::definition()
                    .policy(|actor, _, value| {
                        actor.authority == "overload-unit"
                            && actor.subject == "owner"
                            && value.writable
                    })
                    .allow_all_fields(),
            )
            .build(store.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self {
            runtime,
            store,
            gate,
        }
    }
    async fn query(&self, actor: &Actor) -> Result<Vec<ProjectedView>> {
        self.runtime
            .query_spec_projected(actor, Record::KIND, QuerySpec::all().limit(50))
            .await
    }
    async fn drain(&self) {
        tokio::time::timeout(BOUND, self.runtime.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(self.runtime.status().unwrap().owned_work, 0);
    }
}

#[tokio::test]
async fn authority_batch_fifty_projected_rows_measure_real_gate_reads() {
    let fixture = Fixture::new(usize::MAX, false);
    let result = tokio::time::timeout(BOUND, fixture.query(&actor())).await;
    fixture.drain().await;
    let views = result.unwrap().unwrap();
    assert_eq!(views.len(), 50);
    for (index, view) in views.iter().enumerate() {
        assert_eq!(view.key, key(&format!("page-{index:02}")));
        assert_eq!(view.revision, 1);
        assert_eq!(
            view.value.as_ref().unwrap().get("writable"),
            Some(&json!(true))
        );
    }
    // Characterization, not a minimum security contract: one precheck, fifty
    // disclosures and one final check; two actual keyed reads per gate call.
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 52);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 104);
    assert_eq!(fixture.store.snapshots.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn authority_batch_external_change_during_disclosure_returns_no_partial_page() {
    // Precheck plus four disclosed rows read ten keys. The next row must deny.
    let fixture = Fixture::new(10, false);
    let result = tokio::time::timeout(BOUND, fixture.query(&actor())).await;
    fixture.drain().await;
    assert_eq!(result.unwrap(), Err(Error::Denied));
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 6);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 12);
    assert_eq!(fixture.store.snapshots.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn authority_batch_external_change_after_last_row_denies_final_delivery() {
    // Precheck plus all fifty disclosures read102 keys. The final check must
    // observe the external change even though local generation is unchanged.
    let fixture = Fixture::new(102, false);
    let result = tokio::time::timeout(BOUND, fixture.query(&actor())).await;
    fixture.drain().await;
    assert_eq!(result.unwrap(), Err(Error::Denied));
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 52);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 104);
}

#[tokio::test]
async fn authority_batch_expiry_during_selection_prevents_disclosure() {
    let fixture = Fixture::new(usize::MAX, true);
    let result = tokio::time::timeout(BOUND, fixture.query(&actor())).await;
    fixture.drain().await;
    assert_eq!(result.unwrap(), Err(Error::Denied));
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.store.snapshots.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn authority_batch_local_revocation_denies_next_query_before_storage() {
    let fixture = Fixture::new(usize::MAX, false);
    let actor = actor();
    let first = tokio::time::timeout(BOUND, fixture.query(&actor)).await;
    fixture.runtime.revoke(&actor);
    let second = tokio::time::timeout(BOUND, fixture.query(&actor)).await;
    fixture.drain().await;
    assert_eq!(first.unwrap().unwrap().len(), 50);
    assert_eq!(second.unwrap(), Err(Error::Denied));
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 52);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 104);
    assert_eq!(fixture.store.snapshots.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn authority_batch_gate_denial_stops_selection_before_snapshot() {
    let fixture = Fixture::new(usize::MAX, false);
    let denied = Actor::trusted("overload-unit", "other-owner").expires_at(100);
    let result = tokio::time::timeout(BOUND, fixture.query(&denied)).await;
    fixture.drain().await;
    assert_eq!(result.unwrap(), Err(Error::Denied));
    assert_eq!(fixture.gate.0.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.reads.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.store.snapshots.load(Ordering::SeqCst), 0);
}
