//! Disposable integrated Resource experiment. No concrete persistence or transport dependency.
#![forbid(unsafe_code)]
pub use rom_probe_derive::Resource;
use serde::{Deserialize, Serialize};
pub use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
};
use tokio::sync::{Semaphore, oneshot, watch};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid { kind: String, field: String },
    Unsupported(String),
    Duplicate(String),
    Unregistered,
    Denied,
    Conflict,
    IdentityMismatch,
    Missing,
    Overloaded,
    Closed,
    NotCommitted,
    Unknown,
    Panicked,
    Storage,
    TooLarge,
}
impl Error {
    pub fn invalid(kind: &str, field: &str) -> Self {
        Self::Invalid {
            kind: kind.into(),
            field: field.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    String,
    Bool,
    U64,
    Nullable(Box<Shape>),
}
pub trait Field: Clone + Send + Sync + 'static {
    fn shape() -> Shape;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
}
macro_rules! scalar {
    ($ty:ty,$shape:ident,$get:ident,$map:expr) => {
        impl Field for $ty {
            fn shape() -> Shape {
                Shape::$shape
            }
            fn encode(&self) -> Value {
                json!(self)
            }
            fn decode(value: Value) -> Result<Self> {
                value
                    .$get()
                    .map($map)
                    .ok_or_else(|| Error::invalid("input", "$"))
            }
        }
    };
}
scalar!(String, String, as_str, |s: &str| s.to_owned());
scalar!(bool, Bool, as_bool, |v| v);
scalar!(u64, U64, as_u64, |v| v);
impl<T: Field> Field for Option<T> {
    fn shape() -> Shape {
        Shape::Nullable(Box::new(T::shape()))
    }
    fn encode(&self) -> Value {
        self.as_ref().map_or(Value::Null, Field::encode)
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(None)
        } else {
            T::decode(v).map(Some)
        }
    }
}
pub trait Input: Clone + Send + Sync + 'static {
    fn encode(&self) -> Value;
    fn decode(v: Value) -> Result<Self>;
}
impl<T: Field> Input for T {
    fn encode(&self) -> Value {
        Field::encode(self)
    }
    fn decode(v: Value) -> Result<Self> {
        T::decode(v)
    }
}
impl Input for () {
    fn encode(&self) -> Value {
        Value::Null
    }
    fn decode(v: Value) -> Result<Self> {
        if v.is_null() {
            Ok(())
        } else {
            Err(Error::invalid("input", "$"))
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: String,
    pub shape: Shape,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub kind: String,
    pub version: u32,
    pub fields: Vec<FieldDescriptor>,
}
pub trait Resource: Clone + Send + Sync + 'static {
    const KIND: &'static str;
    fn descriptor() -> Descriptor;
    fn encode(&self) -> Value;
    fn decode(value: Value) -> Result<Self>;
    fn definition() -> Definition<Self> {
        Definition::new()
    }
}
#[derive(Clone)]
pub struct FieldRef<R, T> {
    name: &'static str,
    marker: PhantomData<fn() -> (R, T)>,
}
impl<R: Resource, T: Field> FieldRef<R, T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
    pub fn equals(self, value: T) -> Query<R> {
        Query {
            field: self.name.into(),
            value: Field::encode(&value),
            marker: PhantomData,
        }
    }
}
#[derive(Clone)]
pub struct Query<R> {
    field: String,
    value: Value,
    marker: PhantomData<fn() -> R>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub authority: String,
    pub subject: String,
}
impl Actor {
    /// Trusted host construction; not a credential verifier.
    pub fn trusted(authority: &str, subject: &str) -> Self {
        Self {
            authority: authority.into(),
            subject: subject.into(),
        }
    }
    fn key(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Access {
    Read,
    Write,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub channel: String,
    pub payload: Value,
}
impl Intent {
    pub fn new(channel: &str, payload: Value) -> Self {
        Self {
            channel: channel.into(),
            payload,
        }
    }
}

pub struct Action<R, I> {
    name: &'static str,
    function: fn(&mut R, I) -> Result<Vec<Intent>>,
}
impl<R, I> Copy for Action<R, I> {}
impl<R, I> Clone for Action<R, I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R: Resource, I: Input> Action<R, I> {
    pub const fn new(name: &'static str, function: fn(&mut R, I) -> Result<Vec<Intent>>) -> Self {
        Self { name, function }
    }
}
type ErasedAction = Arc<dyn Fn(Value, Value) -> Result<(Value, Vec<Intent>)> + Send + Sync>;
type Policy<R> = fn(&Actor, Access, &R) -> bool;
pub struct Definition<R: Resource> {
    descriptor: Descriptor,
    actions: BTreeMap<String, ErasedAction>,
    policy: Option<Policy<R>>,
    duplicate: bool,
}
impl<R: Resource> Default for Definition<R> {
    fn default() -> Self {
        Self::new()
    }
}
impl<R: Resource> Definition<R> {
    pub fn new() -> Self {
        Self {
            descriptor: R::descriptor(),
            actions: BTreeMap::new(),
            policy: None,
            duplicate: false,
        }
    }
    pub fn policy(mut self, policy: Policy<R>) -> Self {
        self.policy = Some(policy);
        self
    }
    pub fn action<I: Input>(mut self, action: Action<R, I>) -> Self {
        let f: ErasedAction = Arc::new(move |state, input| {
            let mut r = R::decode(state)?;
            let i = I::decode(input).map_err(|_| Error::invalid(R::KIND, action.name))?;
            let effects = (action.function)(&mut r, i)?;
            Ok((r.encode(), effects))
        });
        self.duplicate |= self.actions.insert(action.name.into(), f).is_some();
        self
    }
}
trait Registered: Send + Sync {
    fn descriptor(&self) -> Descriptor;
    fn normalize(&self, v: Value) -> Result<Value>;
    fn allows(&self, actor: &Actor, access: Access, v: &Value) -> bool;
    fn action(&self, name: &str) -> Result<ErasedAction>;
}
impl<R: Resource> Registered for Definition<R> {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    fn normalize(&self, v: Value) -> Result<Value> {
        let value = R::decode(v)?.encode();
        let descriptor = &self.descriptor;
        let map = value
            .as_object()
            .ok_or_else(|| Error::invalid(R::KIND, "codec object"))?;
        if map.len() != descriptor.fields.len() {
            return Err(Error::invalid(R::KIND, "codec fields"));
        }
        for field in &descriptor.fields {
            if !map
                .get(&field.name)
                .is_some_and(|v| matches_shape(v, &field.shape))
            {
                return Err(Error::invalid(R::KIND, &field.name));
            }
        }
        Ok(value)
    }
    fn allows(&self, a: &Actor, access: Access, v: &Value) -> bool {
        R::decode(v.clone())
            .ok()
            .is_some_and(|r| self.policy.is_some_and(|p| p(a, access, &r)))
    }
    fn action(&self, name: &str) -> Result<ErasedAction> {
        self.actions
            .get(name)
            .cloned()
            .ok_or_else(|| Error::invalid(R::KIND, name))
    }
}

fn matches_shape(value: &Value, shape: &Shape) -> bool {
    match shape {
        Shape::String => value.is_string(),
        Shape::Bool => value.is_boolean(),
        Shape::U64 => value.as_u64().is_some(),
        Shape::Nullable(inner) => value.is_null() || matches_shape(value, inner),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    pub kind: String,
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub key: Key,
    pub revision: u64,
    pub value: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub identity: String,
    pub fingerprint: String,
    pub row: Row,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub expected: Option<u64>,
    pub receipt: Receipt,
    pub changed: bool,
    pub effects: Vec<Intent>,
}
#[derive(Clone, Copy)]
pub struct Capabilities {
    pub atomic_bundle: bool,
    pub snapshots: bool,
    pub effects: bool,
}
/// Single ROM owner per adapter. load/snapshot are authoritative; commit MUST conditionally
/// arbitrate expected revision and identity and atomically persist row/event/receipt/effects.
/// Implementations must not claim rollback for uncertain acknowledgment.
pub trait Storage: Send + Sync + 'static {
    fn capabilities(&self) -> Capabilities;
    fn load(&self, key: &Key) -> Result<Option<Row>>;
    fn snapshot(&self, kind: &str) -> Result<Vec<Row>>;
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>>;
    fn commit(&self, bundle: &Bundle) -> Result<Receipt>;
}
#[derive(Clone, Debug)]
enum Mutation {
    Create(Value),
    Replace(Value),
    Delete,
    Action(String, Value),
}
#[derive(Clone, Debug)]
pub struct Command<R> {
    id: String,
    expected: Option<u64>,
    identity: String,
    mutation: Mutation,
    marker: PhantomData<fn() -> R>,
}
impl<R: Resource> Command<R> {
    pub fn create(id: &str, value: R) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Create(value.encode()),
            marker: PhantomData,
        }
    }
    pub fn replace(id: &str, value: R) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Replace(value.encode()),
            marker: PhantomData,
        }
    }
    pub fn delete(id: &str) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Delete,
            marker: PhantomData,
        }
    }
    pub fn action<I: Input>(id: &str, action: Action<R, I>, input: I) -> Self {
        Self {
            id: id.into(),
            expected: None,
            identity: String::new(),
            mutation: Mutation::Action(action.name.into(), input.encode()),
            marker: PhantomData,
        }
    }
    pub fn at_revision(mut self, revision: u64) -> Self {
        self.expected = Some(revision);
        self
    }
    pub fn idempotency(mut self, key: &str) -> Self {
        self.identity = key.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct Snapshot<R> {
    pub id: String,
    pub revision: u64,
    pub value: Option<R>,
}
fn typed<R: Resource>(row: Row) -> Result<Snapshot<R>> {
    Ok(Snapshot {
        id: row.key.id,
        revision: row.revision,
        value: row.value.map(R::decode).transpose()?,
    })
}

pub struct Builder {
    registry: BTreeMap<String, Arc<dyn Registered>>,
    error: Option<Error>,
    capacity: usize,
}
impl Default for Builder {
    fn default() -> Self {
        Self {
            registry: BTreeMap::new(),
            error: None,
            capacity: 8,
        }
    }
}
impl Builder {
    pub fn resource<R: Resource>(mut self, d: Definition<R>) -> Self {
        let desc = d.descriptor();
        let mut names = BTreeSet::new();
        if desc.kind != R::KIND
            || desc.kind.is_empty()
            || desc.version != 1
            || desc
                .fields
                .iter()
                .any(|f| f.name.is_empty() || !names.insert(f.name.clone()))
        {
            self.error = Some(Error::invalid(R::KIND, "descriptor"));
        }
        if d.duplicate || self.registry.insert(R::KIND.into(), Arc::new(d)).is_some() {
            self.error = Some(Error::Duplicate(R::KIND.into()));
        }
        self
    }
    pub fn capacity(mut self, n: usize) -> Self {
        self.capacity = n;
        self
    }
    pub fn build(self, storage: Arc<dyn Storage>, pool: Arc<rayon::ThreadPool>) -> Result<Runtime> {
        if let Some(e) = self.error {
            return Err(e);
        }
        let c = storage.capabilities();
        if !c.atomic_bundle || !c.snapshots || !c.effects {
            return Err(Error::Unsupported(
                "atomic state/event/receipt/effect bundle + authoritative snapshots".into(),
            ));
        }
        if self.capacity == 0 {
            return Err(Error::Unsupported("zero capacity".into()));
        }
        let (changes, _) = watch::channel(0u64);
        Ok(Runtime(Arc::new(Inner {
            storage,
            pool,
            registry: self.registry,
            gate: Mutex::new(BTreeSet::new()),
            admission: Arc::new(Semaphore::new(self.capacity)),
            jobs: Mutex::new(tokio::task::JoinSet::new()),
            changes,
        })))
    }
}
struct Inner {
    storage: Arc<dyn Storage>,
    pool: Arc<rayon::ThreadPool>,
    registry: BTreeMap<String, Arc<dyn Registered>>,
    gate: Mutex<BTreeSet<String>>,
    admission: Arc<Semaphore>,
    jobs: Mutex<tokio::task::JoinSet<()>>,
    changes: watch::Sender<u64>,
}
#[derive(Clone)]
pub struct Runtime(Arc<Inner>);
impl Runtime {
    pub fn builder() -> Builder {
        Builder::default()
    }
    pub fn shared_cpu_pool(threads: usize) -> Result<Arc<rayon::ThreadPool>> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map(Arc::new)
            .map_err(|_| Error::Storage)
    }
    pub fn available_capacity(&self) -> usize {
        self.0.admission.available_permits()
    }
    /// Host authority revocation, not a parallel managed-user/configuration model.
    pub fn revoke(&self, actor: &Actor) {
        let mut denied = self.0.gate.lock().unwrap();
        denied.insert(actor.key());
        self.0.changes.send_modify(|g| *g += 1);
    }
    pub fn descriptor<R: Resource>(&self, actor: &Actor) -> Result<Descriptor> {
        let denied = self.0.gate.lock().unwrap();
        if denied.contains(&actor.key()) {
            return Err(Error::Denied);
        }
        // Descriptor discovery is deliberately host-only in this probe; no per-kind discover policy.
        self.0
            .registry
            .get(R::KIND)
            .map(|d| d.descriptor())
            .ok_or(Error::Unregistered)
    }
    pub async fn execute<R: Resource>(
        &self,
        actor: &Actor,
        cmd: Command<R>,
    ) -> Result<Snapshot<R>> {
        if cmd.identity.is_empty() || cmd.id.is_empty() {
            return Err(Error::invalid(R::KIND, "identity"));
        }
        let size = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) | Mutation::Action(_, v) => {
                v.to_string().len()
            }
            Mutation::Delete => 0,
        };
        if size + cmd.id.len() + cmd.identity.len() > 16 * 1024 {
            return Err(Error::TooLarge);
        }
        let (tx, rx) = oneshot::channel();
        {
            let mut jobs = self.0.jobs.lock().unwrap();
            while jobs.try_join_next().is_some() {}
            let permit = self
                .0
                .admission
                .clone()
                .try_acquire_owned()
                .map_err(|e| match e {
                    tokio::sync::TryAcquireError::Closed => Error::Closed,
                    tokio::sync::TryAcquireError::NoPermits => Error::Overloaded,
                })?;
            let engine = self.clone();
            let a = actor.clone();
            let pool = self.0.pool.clone();
            jobs.spawn(async move {
                let (done, finished) = oneshot::channel();
                pool.spawn(move || {
                    let result = catch_unwind(AssertUnwindSafe(|| engine.run::<R>(&a, cmd)))
                        .unwrap_or(Err(Error::Panicked));
                    let _ = tx.send(result);
                    drop(permit);
                    let _ = done.send(());
                });
                let _ = finished.await;
            });
        }
        let row = rx.await.map_err(|_| Error::Panicked)??;
        // The worker response can wait in a channel while authority changes.
        let denied = self.0.gate.lock().unwrap();
        let def = self.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
        let current = self.0.storage.load(&row.key)?;
        self.disclose(actor, &denied, def.as_ref(), current.as_ref(), &row)?;
        typed(row)
    }
    fn run<R: Resource>(&self, actor: &Actor, cmd: Command<R>) -> Result<Row> {
        let def = self.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
        let key = Key {
            kind: R::KIND.into(),
            id: cmd.id.clone(),
        };
        let operation = match &cmd.mutation {
            Mutation::Create(_) => "create",
            Mutation::Replace(_) => "replace",
            Mutation::Delete => "delete",
            Mutation::Action(n, _) => n,
        };
        let identity = json!([
            actor.authority,
            actor.subject,
            R::KIND,
            cmd.id,
            operation,
            cmd.identity
        ])
        .to_string();
        let input = match &cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => def.normalize(v.clone())?,
            Mutation::Delete => Value::Null,
            Mutation::Action(_, v) => v.clone(),
        };
        let fingerprint = json!([cmd.expected, input]).to_string();
        let prior = {
            let denied = self.0.gate.lock().unwrap();
            if denied.contains(&actor.key()) {
                return Err(Error::Denied);
            }
            let prior = self.0.storage.load(&key)?;
            if let Some(receipt) = self.0.storage.receipt(&identity)? {
                self.disclose(actor, &denied, def.as_ref(), prior.as_ref(), &receipt.row)?;
                if receipt.fingerprint != fingerprint {
                    return Err(Error::IdentityMismatch);
                }
                return Ok(receipt.row);
            }
            let authorization = prior
                .as_ref()
                .and_then(|r| r.value.as_ref())
                .or(match &cmd.mutation {
                    Mutation::Create(v) => Some(v),
                    _ => None,
                })
                .ok_or(Error::Missing)?;
            if !def.allows(actor, Access::Write, authorization) {
                return Err(Error::Denied);
            }
            if prior.as_ref().map(|r| r.revision) != cmd.expected {
                return Err(Error::Conflict);
            }
            prior
        };
        // Native business functions compute proposals off Tokio. They must be pure w.r.t. external effects.
        let (new_value, effects) = match cmd.mutation {
            Mutation::Create(v) | Mutation::Replace(v) => (Some(def.normalize(v)?), vec![]),
            Mutation::Delete => (None, vec![]),
            Mutation::Action(name, input) => {
                let value = prior
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .ok_or(Error::Missing)?;
                let (v, e) = def.action(&name)?(value, input)?;
                (Some(def.normalize(v)?), e)
            }
        };
        if new_value
            .as_ref()
            .is_some_and(|v| !def.allows(actor, Access::Write, v))
        {
            return Err(Error::Denied);
        }
        if effects.len() > 8
            || effects
                .iter()
                .map(|e| e.payload.to_string().len() + e.channel.len())
                .sum::<usize>()
                > 16 * 1024
            || new_value
                .as_ref()
                .is_some_and(|v| v.to_string().len() > 16 * 1024)
        {
            return Err(Error::TooLarge);
        }
        let denied = self.0.gate.lock().unwrap();
        if denied.contains(&actor.key()) {
            return Err(Error::Denied);
        }
        let current = self.0.storage.load(&key)?;
        if let Some(receipt) = self.0.storage.receipt(&identity)? {
            self.disclose(actor, &denied, def.as_ref(), current.as_ref(), &receipt.row)?;
            if receipt.fingerprint != fingerprint {
                return Err(Error::IdentityMismatch);
            }
            return Ok(receipt.row);
        }
        // Recheck authoritative current state after CPU work and before conditional commit.
        if let Some(v) = current.as_ref().and_then(|r| r.value.as_ref())
            && !def.allows(actor, Access::Write, v)
        {
            return Err(Error::Denied);
        }
        if current.as_ref().map(|r| r.revision) != cmd.expected {
            return Err(Error::Conflict);
        }
        let changed = current.as_ref().and_then(|r| r.value.as_ref()) != new_value.as_ref();
        let revision = cmd
            .expected
            .unwrap_or(0)
            .checked_add(u64::from(changed))
            .ok_or(Error::TooLarge)?;
        let row = Row {
            key,
            revision,
            value: new_value,
        };
        // No-op effects are rejected: an external effect needs a committed transition in this slice.
        if !changed && !effects.is_empty() {
            return Err(Error::invalid(R::KIND, "no-op effects"));
        }
        let bundle = Bundle {
            expected: cmd.expected,
            receipt: Receipt {
                identity,
                fingerprint,
                row: row.clone(),
            },
            changed,
            effects,
        };
        let receipt = self.0.storage.commit(&bundle);
        // Unknown may mean committed: invalidate even when the adapter loses its acknowledgment.
        if receipt.is_ok() || receipt == Err(Error::Unknown) {
            self.0.changes.send_modify(|g| *g += 1);
        }
        let receipt = receipt?;
        self.disclose(
            actor,
            &denied,
            def.as_ref(),
            Some(&receipt.row),
            &receipt.row,
        )?;
        Ok(receipt.row)
    }
    fn disclose(
        &self,
        actor: &Actor,
        denied: &BTreeSet<String>,
        def: &dyn Registered,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<()> {
        if denied.contains(&actor.key()) {
            return Err(Error::Denied);
        }
        if current.is_some_and(|r| r.value.is_none()) && outcome.value.is_some() {
            return Err(Error::Denied);
        }
        // Tombstone replay returns revision only and uses host revocation. Historical field projection omitted.
        if let Some(v) = current.and_then(|r| r.value.as_ref())
            && !def.allows(actor, Access::Read, v)
        {
            return Err(Error::Denied);
        }
        if let Some(v) = &outcome.value
            && !def.allows(actor, Access::Read, v)
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub fn read<R: Resource>(&self, actor: &Actor, id: &str) -> Result<Snapshot<R>> {
        let denied = self.0.gate.lock().unwrap();
        let def = self.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
        let row = self
            .0
            .storage
            .load(&Key {
                kind: R::KIND.into(),
                id: id.into(),
            })?
            .ok_or(Error::Missing)?;
        if row.value.is_none() {
            return Err(Error::Denied);
        }
        self.disclose(actor, &denied, def.as_ref(), Some(&row), &row)?;
        typed(row)
    }
    pub fn query<R: Resource>(&self, actor: &Actor, query: &Query<R>) -> Result<Vec<Snapshot<R>>> {
        let denied = self.0.gate.lock().unwrap();
        if denied.contains(&actor.key()) {
            return Err(Error::Denied);
        }
        let def = self.0.registry.get(R::KIND).ok_or(Error::Unregistered)?;
        if !def
            .descriptor()
            .fields
            .iter()
            .any(|f| f.name == query.field)
        {
            return Err(Error::invalid(R::KIND, &query.field));
        }
        self.0
            .storage
            .snapshot(R::KIND)?
            .into_iter()
            .filter(|r| {
                r.value.as_ref().is_some_and(|v| {
                    def.allows(actor, Access::Read, v) && v.get(&query.field) == Some(&query.value)
                })
            })
            .map(typed)
            .collect()
    }
    pub fn live<R: Resource>(&self, actor: &Actor, query: Query<R>) -> Result<Live<R>> {
        // Subscribe before validation/snapshot: changes cannot be missed between initial read and registration.
        let changes = self.0.changes.subscribe();
        self.query(actor, &query)?;
        Ok(Live {
            runtime: self.clone(),
            actor: actor.clone(),
            query,
            changes,
            initial: true,
        })
    }
    pub async fn shutdown(&self) -> Result<()> {
        let mut jobs = {
            let mut guard = self.0.jobs.lock().unwrap();
            self.0.admission.close();
            std::mem::take(&mut *guard)
        };
        while let Some(result) = jobs.join_next().await {
            result.map_err(|_| Error::Panicked)?;
        }
        Ok(())
    }
}
pub struct Live<R> {
    runtime: Runtime,
    actor: Actor,
    query: Query<R>,
    changes: watch::Receiver<u64>,
    initial: bool,
}
impl<R: Resource> Live<R> {
    pub async fn changed(&mut self) -> Result<Vec<Snapshot<R>>> {
        if !self.initial {
            self.changes.changed().await.map_err(|_| Error::Closed)?;
        }
        self.initial = false;
        self.changes.borrow_and_update();
        // Retain only generation signals; re-read and authorize at the delivery boundary.
        self.runtime.query(&self.actor, &self.query)
    }
}
