//! Runtime protocol conformance using real SQLite persistence and fault responses.
//! Synthetic replies test protocol faults; Reply::Sqlite delegates unchanged to
//! the maintained native index and counts its actual candidate responses.
use rom::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Clone, Debug, Resource)]
#[resource(name = "query-read-items")]
struct Item {
    amount: u64,
    title: String,
}
fn actor() -> Actor {
    Actor::trusted("tests", "reader")
}
fn uniform() -> Definition<Item> {
    Item::definition().read_policy(|_| true).allow_all_fields()
}
#[derive(Clone, Copy, Debug)]
enum Reply {
    Sqlite,
    Reference,
    Native,
    WrongRequest,
    EmptyStore,
    WrongProfile,
    WrongKind,
    Duplicate,
    ImpossibleRows,
    ImpossibleBytes,
    ExcessRows,
    ExcessPersistedBytes,
    ExcessCanonicalBytes,
    Failure,
}
struct Observed {
    database: rom_sqlite::Sqlite,
    reply: Reply,
    snapshots: AtomicUsize,
    native_reads: AtomicUsize,
    requests: Mutex<Vec<StorageQuery>>,
    at_read: Option<fn()>,
}
impl Storage for Observed {
    fn acquire_owner(&self) -> Result<StorageOwner> {
        self.database.acquire_owner()
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.database.register(descriptors)
    }
    fn capabilities(&self) -> Capabilities {
        self.database.capabilities()
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.database.load(key)
    }
    fn snapshot(&self, kind: &str, rows: usize, bytes: usize) -> Result<Vec<Row>> {
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        self.database.snapshot(kind, rows, bytes)
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>> {
        self.database.receipt(identity)
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.database.commit(bundle)
    }
    fn query_read(&self, request: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        self.requests.lock().unwrap().push(request.clone());
        if matches!(self.reply, Reply::Sqlite) {
            let result = self.database.query_read(request, bounds)?;
            if matches!(result, QueryRead::NativeCandidates { .. }) {
                self.native_reads.fetch_add(1, Ordering::SeqCst);
            }
            // The adapter has released its native guard before external code.
            if let Some(callback) = self.at_read {
                callback();
            }
            return Ok(result);
        }
        if let Some(callback) = self.at_read {
            callback();
        }
        if matches!(self.reply, Reply::Failure) {
            return Err(Error::Storage);
        }
        let mut rows =
            self.database
                .snapshot(&request.descriptor.kind, bounds.max_rows, bounds.max_bytes)?;
        if matches!(self.reply, Reply::Reference) {
            return Ok(QueryRead::Reference { rows });
        }
        let bytes = rows
            .iter()
            .map(|row| serde_json::to_vec(row).unwrap().len())
            .sum();
        let mut admission = KindAdmission {
            rows: rows.len(),
            persisted_bytes: bytes,
            canonical_bytes: bytes,
        };
        // This fixture implements only equality preselection. Other predicates,
        // sorting, anchors and page limits remain entirely in the shared evaluator.
        rows.retain(|row| {
            row.value.as_ref().is_some_and(|value| {
                request.spec.filters.iter().all(|filter| {
                    if filter.absent {
                        value.get(&filter.field).is_none()
                    } else {
                        value.get(&filter.field) == Some(&filter.value)
                    }
                })
            })
        });
        rows.reverse();
        let mut binding = ReadBinding {
            request: request.clone(),
            snapshot: QuerySnapshot {
                store: "SQLite conformance wrapper".into(),
                generation: 1,
                profile_version: 1,
                encoding_version: 1,
            },
        };
        match self.reply {
            Reply::WrongRequest => binding.request.spec.limit = Some(99),
            Reply::EmptyStore => binding.snapshot.store.clear(),
            Reply::WrongProfile => binding.snapshot.profile_version += 1,
            Reply::WrongKind => rows[0].key.kind = "other-kind".into(),
            Reply::Duplicate => {
                rows.push(rows[0].clone());
                admission.rows += 1;
                admission.canonical_bytes *= 2;
                admission.persisted_bytes *= 2;
            }
            Reply::ImpossibleRows => admission.rows = 0,
            Reply::ImpossibleBytes => admission.canonical_bytes = 0,
            Reply::ExcessRows => {
                admission.rows = bounds.max_rows + 1;
                admission.persisted_bytes = admission.persisted_bytes.max(admission.rows);
                admission.canonical_bytes = admission.canonical_bytes.max(admission.rows);
            }
            Reply::ExcessPersistedBytes => admission.persisted_bytes = bounds.max_bytes + 1,
            Reply::ExcessCanonicalBytes => admission.canonical_bytes = bounds.max_bytes + 1,
            _ => {}
        }
        Ok(QueryRead::NativeCandidates {
            rows,
            admission,
            binding: Box::new(binding),
        })
    }
}
struct Fixture {
    runtime: Runtime,
    storage: Arc<Observed>,
    directory: std::path::PathBuf,
}
impl Fixture {
    fn new<R: Resource>(definition: Definition<R>, reply: Reply) -> Self {
        Self::configured(definition, reply, Limits::default(), None)
    }
    fn configured<R: Resource>(
        definition: Definition<R>,
        reply: Reply,
        limits: Limits,
        at_read: Option<fn()>,
    ) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "rom-query-read-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let storage = Arc::new(Observed {
            database: rom_sqlite::Sqlite::open(directory.join("database")).unwrap(),
            reply,
            snapshots: AtomicUsize::new(0),
            native_reads: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
            at_read,
        });
        let runtime = Runtime::builder()
            .limits(limits)
            .resource(definition)
            .build(storage.clone(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        Self {
            runtime,
            storage,
            directory,
        }
    }
    // Seed through the real adapter's atomic bundle API, without invoking a
    // custom application decoder before the observation under test.
    fn seed(&self, kind: &str, id: &str, value: Value) {
        self.storage
            .commit(&Bundle {
                expected: None,
                receipt: Receipt {
                    identity: format!("seed-{id}"),
                    fingerprint: format!("seed-{id}"),
                    retry_epoch: 0,
                    replay_version: None,
                    row: Row {
                        key: Key {
                            kind: kind.into(),
                            id: id.into(),
                        },
                        revision: 1,
                        value: Some(value),
                        protected: ProtectedMetadata::default(),
                    },
                },
                changed: true,
                effects: vec![],
                reactions: vec![],
                reaction_limits: None,
                completed_work: None,
            })
            .unwrap();
    }
    fn items(&self) {
        for (id, amount) in [("a", 9), ("b", 1), ("c", 1)] {
            self.seed(Item::KIND, id, json!({"amount":amount,"title":id}));
        }
    }
    async fn ids(&self, spec: QuerySpec) -> Result<Vec<String>> {
        Ok(self
            .runtime
            .query_spec_projected(&actor(), Item::KIND, spec)
            .await?
            .into_iter()
            .map(|view| view.key.id)
            .collect())
    }
    fn assert_requests(&self, count: usize, selection: SelectionMode) {
        let requests = self.storage.requests.lock().unwrap();
        assert_eq!(requests.len(), count);
        assert!(
            requests
                .iter()
                .all(|request| request.selection == selection)
        );
        assert_eq!(self.storage.snapshots.load(Ordering::SeqCst), 0);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn reference_and_complete_candidates_share_typed_projected_and_live_results() {
    for reply in [Reply::Reference, Reply::Native] {
        let f = Fixture::new(uniform(), reply);
        f.items();
        let query = Item::amount_field()
            .equals(1)
            .order_by(Item::title_field(), Direction::Desc)
            .limit(1);
        let first = f.runtime.query(&actor(), &query).await.unwrap();
        assert_eq!(first[0].id, "c");
        let after = query.clone().after_snapshot(&first[0]).unwrap();
        assert_eq!(f.ids(after.spec().clone()).await.unwrap(), ["b"]);
        let mut typed = f.runtime.live(&actor(), query.clone()).await.unwrap();
        assert_eq!(typed.changed().await.unwrap()[0].id, "c");
        let mut projected = f
            .runtime
            .live_spec_projected(&actor(), Item::KIND, query.spec().clone())
            .await
            .unwrap();
        assert_eq!(projected.changed().await.unwrap()[0].key.id, "c");
        assert_eq!(
            f.ids(QuerySpec::equal("amount", json!(77))).await.unwrap(),
            Vec::<String>::new()
        );
        f.assert_requests(7, SelectionMode::UniformReadAndFields);
    }
}

#[tokio::test]
async fn explicit_denial_precedes_empty_or_overbound_storage_reads() {
    for seeded in [false, true] {
        let f = Fixture::configured(
            Item::definition().read_policy(|_| false).allow_all_fields(),
            Reply::Failure,
            Limits {
                snapshot_rows: 1,
                ..Limits::default()
            },
            None,
        );
        if seeded {
            f.items();
        }
        assert_eq!(f.ids(QuerySpec::all()).await.unwrap_err(), Error::Denied);
        f.assert_requests(0, SelectionMode::UniformReadAndFields);
    }
}

#[tokio::test]
async fn builder_overrides_keep_opaque_selection_and_field_preflight() {
    let empty = Fixture::new(
        Item::definition()
            .policy(|_, _, _| false)
            .allow_all_fields(),
        Reply::Reference,
    );
    assert!(empty.ids(QuerySpec::all()).await.unwrap().is_empty());
    empty.assert_requests(1, SelectionMode::ReferenceOnly);

    let opaque = uniform().policy(|_, _, row| row.amount == 1);
    let f = Fixture::new(opaque, Reply::Reference);
    f.items();
    assert_eq!(f.ids(QuerySpec::all()).await.unwrap(), ["b", "c"]);
    f.assert_requests(1, SelectionMode::ReferenceOnly);

    let mixed = uniform().field_policy(|_, _, field, row| field != "amount" || row.amount == 1);
    let f = Fixture::new(mixed, Reply::Reference);
    f.items();
    assert_eq!(
        f.ids(QuerySpec::equal("amount", json!(1)).order_by("amount", Direction::Asc))
            .await
            .unwrap_err(),
        Error::Denied
    );
    f.assert_requests(1, SelectionMode::ReferenceOnly);

    let f = Fixture::new(
        Item::definition()
            .policy(|_, _, _| false)
            .read_policy(|_| true)
            .allow_all_fields(),
        Reply::Native,
    );
    f.items();
    assert_eq!(f.ids(QuerySpec::all()).await.unwrap(), ["a", "b", "c"]);
    f.assert_requests(1, SelectionMode::UniformReadAndFields);
}

#[tokio::test]
async fn malformed_native_responses_and_execution_failures_never_fall_back() {
    for reply in [
        Reply::WrongRequest,
        Reply::EmptyStore,
        Reply::WrongProfile,
        Reply::WrongKind,
        Reply::Duplicate,
        Reply::ImpossibleRows,
        Reply::ImpossibleBytes,
        Reply::ExcessRows,
        Reply::ExcessPersistedBytes,
        Reply::ExcessCanonicalBytes,
        Reply::Failure,
    ] {
        let f = Fixture::new(uniform(), reply);
        f.items();
        let expected = match reply {
            Reply::ExcessRows | Reply::ExcessPersistedBytes | Reply::ExcessCanonicalBytes => {
                Error::TooLarge
            }
            _ => Error::Storage,
        };
        assert_eq!(
            f.ids(QuerySpec::all()).await.unwrap_err(),
            expected,
            "{reply:?}"
        );
        f.assert_requests(1, SelectionMode::UniformReadAndFields);
    }
    let f = Fixture::new(
        Item::definition().policy(|_, _, _| true).allow_all_fields(),
        Reply::Native,
    );
    f.items();
    assert_eq!(f.ids(QuerySpec::all()).await.unwrap_err(), Error::Storage);
    f.assert_requests(1, SelectionMode::ReferenceOnly);
}

static SELECTION_CALLS: AtomicUsize = AtomicUsize::new(0);
static SELECTION_ALLOWED: AtomicBool = AtomicBool::new(true);
fn counted_read(_: &Actor) -> bool {
    SELECTION_CALLS.fetch_add(1, Ordering::SeqCst);
    SELECTION_ALLOWED.load(Ordering::SeqCst)
}
fn revoke_after_selection() {
    assert_eq!(SELECTION_CALLS.load(Ordering::SeqCst), 1);
    SELECTION_ALLOWED.store(false, Ordering::SeqCst);
}
#[tokio::test]
async fn read_grant_runs_once_after_query_validation_and_disclosure_rechecks_authority() {
    SELECTION_CALLS.store(0, Ordering::SeqCst);
    SELECTION_ALLOWED.store(true, Ordering::SeqCst);
    let f = Fixture::new(
        Item::definition()
            .read_policy(counted_read)
            .allow_all_fields()
            .query_policy(|_, field| field != "title")
            .sort_policy(|_, field| field != "title"),
        Reply::Native,
    );
    assert_eq!(
        f.ids(QuerySpec::equal("title", json!("no")))
            .await
            .unwrap_err(),
        Error::Denied
    );
    assert_eq!(
        f.ids(QuerySpec::all().order_by("title", Direction::Asc))
            .await
            .unwrap_err(),
        Error::Denied
    );
    assert!(matches!(
        f.ids(QuerySpec::equal("amount", json!("wrong shape")))
            .await,
        Err(Error::Invalid { .. })
    ));
    assert_eq!(SELECTION_CALLS.load(Ordering::SeqCst), 0);
    f.assert_requests(0, SelectionMode::UniformReadAndFields);
    // Empty results have no disclosure callbacks: exactly one selection grant.
    assert!(f.ids(QuerySpec::all()).await.unwrap().is_empty());
    assert_eq!(SELECTION_CALLS.load(Ordering::SeqCst), 1);
    f.assert_requests(1, SelectionMode::UniformReadAndFields);

    SELECTION_CALLS.store(0, Ordering::SeqCst);
    let f = Fixture::configured(
        Item::definition()
            .read_policy(counted_read)
            .allow_all_fields(),
        Reply::Native,
        Limits::default(),
        Some(revoke_after_selection),
    );
    f.items();
    assert_eq!(f.ids(QuerySpec::all()).await.unwrap_err(), Error::Denied);
    assert!(SELECTION_CALLS.load(Ordering::SeqCst) > 1);
    f.assert_requests(1, SelectionMode::UniformReadAndFields);
}

#[derive(Clone, Debug)]
struct Fragile(String);
impl Field for Fragile {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        let text = value
            .as_str()
            .ok_or_else(|| Error::invalid("codec", "text"))?;
        assert_ne!(text, "poison", "custom decoder reached an excluded row");
        Ok(Self(text.trim().into()))
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "query-read-fragile")]
struct FragileItem {
    amount: u64,
    text: Fragile,
}
#[tokio::test]
async fn excluded_decoder_is_skipped_only_under_explicit_uniform_contract() {
    for reply in [Reply::Reference, Reply::Native] {
        let f = Fixture::new(
            FragileItem::definition()
                .read_policy(|_| true)
                .allow_all_fields(),
            reply,
        );
        f.seed(FragileItem::KIND, "a", json!({"amount":9,"text":"poison"}));
        f.seed(FragileItem::KIND, "b", json!({"amount":1,"text":"ok"}));
        let query = QuerySpec::equal("amount", json!(1)).order_by("amount", Direction::Asc);
        let rows = f
            .runtime
            .query_spec_projected(&actor(), FragileItem::KIND, query)
            .await
            .unwrap();
        assert_eq!(rows[0].key.id, "b");
        f.assert_requests(1, SelectionMode::UniformReadAndFields);
        assert_eq!(
            f.runtime
                .read::<FragileItem>(&actor(), "a")
                .await
                .unwrap_err(),
            Error::Panicked
        );
    }
    let f = Fixture::new(
        FragileItem::definition()
            .policy(|_, _, _| true)
            .allow_all_fields(),
        Reply::Reference,
    );
    f.seed(FragileItem::KIND, "a", json!({"amount":9,"text":"poison"}));
    assert_eq!(
        f.runtime
            .query_spec_projected(
                &actor(),
                FragileItem::KIND,
                QuerySpec::equal("amount", json!(1))
            )
            .await
            .unwrap_err(),
        Error::Panicked
    );
    f.assert_requests(1, SelectionMode::ReferenceOnly);
}

#[tokio::test]
async fn normalized_predicates_reach_storage_and_returned_rows_still_decode() {
    for reply in [Reply::Reference, Reply::Native] {
        let f = Fixture::new(
            FragileItem::definition()
                .read_policy(|_| true)
                .allow_all_fields(),
            reply,
        );
        f.seed(FragileItem::KIND, "b", json!({"amount":1,"text":"ok"}));
        let rows = f
            .runtime
            .query_spec_projected(
                &actor(),
                FragileItem::KIND,
                QuerySpec::equal("text", json!(" ok ")),
            )
            .await
            .unwrap();
        assert_eq!(rows[0].key.id, "b");
        assert_eq!(
            f.storage.requests.lock().unwrap()[0].spec.filters[0].value,
            json!("ok")
        );
        f.seed(FragileItem::KIND, "a", json!({"amount":9,"text":"poison"}));
        assert_eq!(
            f.runtime
                .query_spec_projected(&actor(), FragileItem::KIND, QuerySpec::all())
                .await
                .unwrap_err(),
            Error::Panicked
        );
        f.assert_requests(2, SelectionMode::UniformReadAndFields);
    }
}

#[tokio::test]
async fn native_candidates_cannot_bypass_whole_kind_admission_with_a_small_page() {
    for reply in [Reply::Reference, Reply::Native] {
        let f = Fixture::configured(
            uniform(),
            reply,
            Limits {
                snapshot_rows: 1,
                ..Limits::default()
            },
            None,
        );
        f.items();
        assert_eq!(
            f.ids(QuerySpec::equal("amount", json!(1)).limit(1))
                .await
                .unwrap_err(),
            Error::TooLarge
        );
        f.assert_requests(1, SelectionMode::UniformReadAndFields);
    }
}

fn many_items(f: &Fixture) {
    for amount in 0u64..128 {
        let id = format!("{amount:03}");
        f.seed(Item::KIND, &id, json!({"amount":amount,"title":id}));
    }
}
fn many_fragile(f: &Fixture) {
    for amount in 0u64..128 {
        let text = if amount == 0 { "poison" } else { "ok" };
        f.seed(
            FragileItem::KIND,
            &format!("{amount:03}"),
            json!({"amount":amount,"text":text}),
        );
    }
}
#[tokio::test]
async fn actual_native_index_skips_excluded_decoder_only_with_uniform_authority() {
    for reply in [Reply::Reference, Reply::Sqlite] {
        let f = Fixture::new(
            FragileItem::definition()
                .read_policy(|_| true)
                .allow_all_fields(),
            reply,
        );
        many_fragile(&f);
        let query = QuerySpec::equal("amount", json!(1)).order_by("amount", Direction::Asc);
        let rows = f
            .runtime
            .query_spec_projected(&actor(), FragileItem::KIND, query)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key.id, "001");
        assert_eq!(
            f.storage.native_reads.load(Ordering::SeqCst),
            usize::from(matches!(reply, Reply::Sqlite))
        );
        // A selected row still executes its custom decoder after materialization.
        assert_eq!(
            f.runtime
                .query_spec_projected(
                    &actor(),
                    FragileItem::KIND,
                    QuerySpec::equal("amount", json!(0))
                )
                .await
                .unwrap_err(),
            Error::Panicked
        );
        assert_eq!(
            f.storage.native_reads.load(Ordering::SeqCst),
            2 * usize::from(matches!(reply, Reply::Sqlite))
        );
        f.assert_requests(2, SelectionMode::UniformReadAndFields);
    }
    let f = Fixture::new(
        FragileItem::definition()
            .policy(|_, _, _| true)
            .allow_all_fields(),
        Reply::Sqlite,
    );
    many_fragile(&f);
    assert_eq!(
        f.runtime
            .query_spec_projected(
                &actor(),
                FragileItem::KIND,
                QuerySpec::equal("amount", json!(1))
            )
            .await
            .unwrap_err(),
        Error::Panicked
    );
    assert_eq!(f.storage.native_reads.load(Ordering::SeqCst), 0);
    f.assert_requests(1, SelectionMode::ReferenceOnly);
}

#[tokio::test]
async fn actual_native_storage_preserves_denial_before_limits_and_opaque_field_preflight() {
    let f = Fixture::configured(
        Item::definition().read_policy(|_| false).allow_all_fields(),
        Reply::Sqlite,
        Limits {
            snapshot_rows: 1,
            ..Limits::default()
        },
        None,
    );
    many_items(&f);
    assert_eq!(
        f.ids(QuerySpec::equal("amount", json!(1)).limit(1))
            .await
            .unwrap_err(),
        Error::Denied
    );
    f.assert_requests(0, SelectionMode::UniformReadAndFields);
    let f = Fixture::configured(
        uniform(),
        Reply::Sqlite,
        Limits {
            snapshot_rows: 1,
            ..Limits::default()
        },
        None,
    );
    many_items(&f);
    assert_eq!(
        f.ids(QuerySpec::equal("amount", json!(1)).limit(1))
            .await
            .unwrap_err(),
        Error::TooLarge
    );
    f.assert_requests(1, SelectionMode::UniformReadAndFields);
    assert_eq!(f.storage.native_reads.load(Ordering::SeqCst), 0);

    let f = Fixture::new(
        uniform().field_policy(|_, _, field, row| field != "amount" || row.amount != 0),
        Reply::Sqlite,
    );
    many_items(&f);
    assert_eq!(
        f.ids(QuerySpec::equal("amount", json!(1)).order_by("amount", Direction::Asc))
            .await
            .unwrap_err(),
        Error::Denied
    );
    f.assert_requests(1, SelectionMode::ReferenceOnly);
    assert_eq!(f.storage.native_reads.load(Ordering::SeqCst), 0);
}

static NATIVE_ALLOWED: AtomicBool = AtomicBool::new(true);
fn native_read_grant(_: &Actor) -> bool {
    NATIVE_ALLOWED.load(Ordering::SeqCst)
}
fn native_revoke() {
    NATIVE_ALLOWED.store(false, Ordering::SeqCst);
}
#[tokio::test]
async fn actual_native_candidates_still_require_current_disclosure_authority() {
    NATIVE_ALLOWED.store(true, Ordering::SeqCst);
    let f = Fixture::configured(
        Item::definition()
            .read_policy(native_read_grant)
            .allow_all_fields(),
        Reply::Sqlite,
        Limits::default(),
        Some(native_revoke),
    );
    many_items(&f);
    assert_eq!(
        f.ids(QuerySpec::equal("amount", json!(1)))
            .await
            .unwrap_err(),
        Error::Denied
    );
    f.assert_requests(1, SelectionMode::UniformReadAndFields);
    assert_eq!(
        f.storage.native_reads.load(Ordering::SeqCst),
        1,
        "denial followed actual native selection"
    );
}
