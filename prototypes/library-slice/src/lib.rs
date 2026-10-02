//! Disposable transport-free library: one shared resource/action/live-query path.
//! This tests API and correctness boundaries, not production readiness.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::{watch, Semaphore};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Forbidden,
    Invalid,
    NotFound,
    Conflict,
    Closed,
    Busy,
    Storage,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        error(ErrorCode::Storage, e.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        error(ErrorCode::Storage, e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn error(code: ErrorCode, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}

/// Trusted actor supplied by the embedding host. This is not an authentication credential.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Actor(pub String);
impl Actor {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Access<'a> {
    Read {
        kind: &'a str,
        id: &'a str,
    },
    Query {
        kind: &'a str,
    },
    Action {
        kind: &'a str,
        id: &'a str,
        name: &'a str,
    },
}
/// Synchronous, nonblocking host policy. Do not reenter this Runtime from this callback.
/// Replace policies through Runtime::set_authorizer so live queries are invalidated.
pub trait Authorizer: Send + Sync + 'static {
    fn allows(&self, actor: &Actor, access: Access<'_>, resource: Option<&Resource>) -> bool;
}
pub struct DenyAll;
impl Authorizer for DenyAll {
    fn allows(&self, _: &Actor, _: Access<'_>, _: Option<&Resource>) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Boolean,
    Integer,
    Text,
}
pub trait FieldType {
    const KIND: FieldKind;
    fn accepts(value: &Value) -> bool;
}
impl FieldType for bool {
    const KIND: FieldKind = FieldKind::Boolean;
    fn accepts(v: &Value) -> bool {
        v.is_boolean()
    }
}
impl FieldType for i64 {
    const KIND: FieldKind = FieldKind::Integer;
    fn accepts(v: &Value) -> bool {
        v.is_i64()
    }
}
impl FieldType for String {
    const KIND: FieldKind = FieldKind::Text;
    fn accepts(v: &Value) -> bool {
        v.is_string()
    }
}
#[derive(Clone)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
    accepts: fn(&Value) -> bool,
}
impl Field {
    pub fn of<T: FieldType>(name: &'static str) -> Self {
        Self {
            name,
            kind: T::KIND,
            accepts: T::accepts,
        }
    }
}
pub type ActionFn = fn(&mut Value, &Value) -> std::result::Result<(), String>;
#[derive(Clone)]
pub struct Action {
    pub name: &'static str,
    pub apply: ActionFn,
}
#[derive(Clone)]
pub struct Definition {
    pub kind: &'static str,
    pub fields: Vec<Field>,
    pub actions: Vec<Action>,
}
impl Definition {
    fn validate(&self, value: &Value, partial: bool) -> Result<()> {
        let fields = value.as_object().ok_or_else(|| {
            error(
                ErrorCode::Invalid,
                format!("resource {}: expected object", self.kind),
            )
        })?;
        for (name, value) in fields {
            let field = self.fields.iter().find(|f| f.name == name).ok_or_else(|| {
                error(
                    ErrorCode::Invalid,
                    format!("resource {}, field {name}: unknown field", self.kind),
                )
            })?;
            if !(field.accepts)(value) {
                return Err(error(
                    ErrorCode::Invalid,
                    format!(
                        "resource {}, field {name}: expected {:?}",
                        self.kind, field.kind
                    ),
                ));
            }
        }
        if !partial {
            for field in &self.fields {
                if !fields.contains_key(field.name) {
                    return Err(error(
                        ErrorCode::Invalid,
                        format!("resource {}, field {}: required", self.kind, field.name),
                    ));
                }
            }
        }
        Ok(())
    }
}
/// The declaration creates a descriptor; Rust custom-action functions remain ordinary code.
#[macro_export]
macro_rules! resource {
    ($vis:vis $name:ident ($kind:literal) { $($field:ident:$ty:ty),* $(,)? } actions { $($action:literal=>$handler:path),* $(,)? }) => {
        $vis struct $name;
        impl $name {
            pub const KIND:&'static str=$kind;
            pub fn definition()->$crate::Definition{
                $crate::Definition{kind:$kind,fields:vec![$($crate::Field::of::<$ty>(stringify!($field))),*],actions:vec![$($crate::Action{name:$action,apply:$handler}),*]}
            }
        }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub kind: String,
    pub id: String,
    pub revision: i64,
    pub data: Value,
    pub deleted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub seq: i64,
    pub action: String,
    pub resource: Resource,
}
#[derive(Clone, Debug)]
pub struct Command {
    kind: String,
    id: String,
    name: String,
    revision: i64,
    input: Value,
}
impl Command {
    pub fn create(kind: impl Into<String>, id: impl Into<String>, input: Value) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            name: "create".into(),
            revision: 0,
            input,
        }
    }
    pub fn update(
        kind: impl Into<String>,
        id: impl Into<String>,
        revision: i64,
        input: Value,
    ) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            name: "update".into(),
            revision,
            input,
        }
    }
    pub fn delete(kind: impl Into<String>, id: impl Into<String>, revision: i64) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            name: "delete".into(),
            revision,
            input: json!({}),
        }
    }
    pub fn action(
        kind: impl Into<String>,
        id: impl Into<String>,
        name: impl Into<String>,
        revision: i64,
        input: Value,
    ) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            name: name.into(),
            revision,
            input,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Outcome {
    pub resource: Resource,
    pub event_id: Option<i64>,
    pub generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Query {
    kind: String,
    field: String,
    equals: bool,
}
impl Query {
    pub fn equals(kind: impl Into<String>, field: impl Into<String>, equals: bool) -> Self {
        Self {
            kind: kind.into(),
            field: field.into(),
            equals,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub rows: Vec<Resource>,
    pub journal_cursor: i64,
    pub generation: u64,
    pub policy_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    actor: Actor,
    query: Query,
}
type Updates = std::result::Result<Snapshot, Error>;
struct State {
    db: Connection,
    definitions: HashMap<String, Definition>,
    authorizer: Arc<dyn Authorizer>,
    queries: HashMap<Key, watch::Sender<Updates>>,
    generation: u64,
    policy_epoch: u64,
    closed: bool,
    max_queries: usize,
}
struct Inner {
    state: Mutex<State>,
    admission: Arc<Semaphore>,
    changes: watch::Sender<u64>,
    settled: watch::Sender<u64>,
    worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<Inner>,
}
pub struct Builder {
    definitions: Vec<Definition>,
    authorizer: Arc<dyn Authorizer>,
    max_queries: usize,
}
impl Default for Builder {
    fn default() -> Self {
        Self {
            definitions: vec![],
            authorizer: Arc::new(DenyAll),
            max_queries: 16,
        }
    }
}
impl Builder {
    pub fn resource(mut self, definition: Definition) -> Self {
        self.definitions.push(definition);
        self
    }
    pub fn authorizer(mut self, authorizer: impl Authorizer) -> Self {
        self.authorizer = Arc::new(authorizer);
        self
    }
    pub fn max_queries(mut self, max: usize) -> Self {
        self.max_queries = max;
        self
    }
    pub async fn open_sqlite(self, path: impl AsRef<Path>) -> Result<Runtime> {
        let path = path.as_ref().to_owned();
        let definitions = self.definitions;
        let authorizer = self.authorizer;
        let max_queries = self.max_queries;
        let state=tokio::task::spawn_blocking(move||->Result<State>{
            if max_queries==0{return Err(error(ErrorCode::Invalid,"max_queries must be positive"));}
            let mut registry=HashMap::new();
            for definition in definitions {
                if registry.contains_key(definition.kind){return Err(error(ErrorCode::Invalid,"duplicate resource kind"));}
                let mut fields=std::collections::HashSet::new();
                for field in &definition.fields{if !fields.insert(field.name){return Err(error(ErrorCode::Invalid,"duplicate field"));}}
                let mut names=std::collections::HashSet::from(["create","update","delete"]);
                for action in &definition.actions{if !names.insert(action.name){return Err(error(ErrorCode::Invalid,"duplicate or reserved action"));}}
                registry.insert(definition.kind.to_string(),definition);
            }
            let db=Connection::open(path)?;
            db.busy_timeout(Duration::from_secs(2))?;
            db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
              CREATE TABLE IF NOT EXISTS resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,deleted INTEGER NOT NULL,PRIMARY KEY(kind,id));
              CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY AUTOINCREMENT,action TEXT NOT NULL,resource TEXT NOT NULL);")?;
            Ok(State{db,definitions:registry,authorizer,queries:HashMap::new(),generation:0,policy_epoch:0,closed:false,max_queries})
        }).await.map_err(join_error)??;
        let (changes, rx) = watch::channel(0);
        let (settled, _) = watch::channel(0);
        let inner = Arc::new(Inner {
            state: Mutex::new(state),
            admission: Arc::new(Semaphore::new(32)),
            changes,
            settled,
            worker: Mutex::new(None),
        });
        let handle = tokio::spawn(refresh_worker(Arc::downgrade(&inner), rx));
        *inner.worker.lock().expect("worker mutex") = Some(handle);
        Ok(Runtime { inner })
    }
}
fn join_error(error: tokio::task::JoinError) -> Error {
    crate::error(
        ErrorCode::Storage,
        format!("library worker failed: {error}"),
    )
}
async fn access<T: Send + 'static>(
    inner: Arc<Inner>,
    f: impl FnOnce(&mut State, &Inner) -> Result<T> + Send + 'static,
) -> Result<T> {
    let permit = inner
        .admission
        .clone()
        .try_acquire_owned()
        .map_err(|_| error(ErrorCode::Busy, "operation admission full"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut state = inner
            .state
            .lock()
            .map_err(|_| error(ErrorCode::Storage, "state mutex poisoned"))?;
        if state.closed {
            return Err(error(ErrorCode::Closed, "runtime shut down"));
        }
        f(&mut state, &inner)
    })
    .await
    .map_err(join_error)?
}
fn invalidate(state: &mut State, inner: &Inner) {
    state.generation += 1;
    inner.changes.send_replace(state.generation);
}
fn authorize(
    state: &State,
    actor: &Actor,
    access: Access<'_>,
    resource: Option<&Resource>,
) -> Result<()> {
    if state.authorizer.allows(actor, access, resource) {
        Ok(())
    } else {
        Err(error(
            ErrorCode::Forbidden,
            format!("actor {} denied {access:?}", actor.0),
        ))
    }
}
impl Runtime {
    pub fn builder() -> Builder {
        Builder::default()
    }
    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Outcome> {
        let actor = actor.clone();
        access(self.inner.clone(), move |state, inner| {
            let c = command;
            let definition = state
                .definitions
                .get(&c.kind)
                .cloned()
                .ok_or_else(|| error(ErrorCode::NotFound, "unknown resource kind"))?;
            let authorizer = state.authorizer.clone();
            let tx = state
                .db
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            // Policy, revision checks, and mutation use the same authoritative row.
            let previous = load(&tx, &c.kind, &c.id)?;
            let action_access = Access::Action {
                kind: &c.kind,
                id: &c.id,
                name: &c.name,
            };
            if !authorizer.allows(&actor, action_access, previous.as_ref()) {
                return Err(error(
                    ErrorCode::Forbidden,
                    format!("actor {} denied {action_access:?}", actor.0),
                ));
            }
            let revision = previous.as_ref().map_or(0, |r| r.revision);
            if revision != c.revision {
                return Err(error(
                    ErrorCode::Conflict,
                    format!(
                        "resource {}/{}, action {}: expected revision {}, found {revision}",
                        c.kind, c.id, c.name, c.revision
                    ),
                ));
            }
            let mut current = if c.name == "create" {
                if previous.is_some() {
                    return Err(error(
                        ErrorCode::Conflict,
                        "resource identity already exists",
                    ));
                }
                definition.validate(&c.input, false)?;
                Resource {
                    kind: c.kind.clone(),
                    id: c.id.clone(),
                    revision: 1,
                    data: c.input.clone(),
                    deleted: false,
                }
            } else {
                let mut resource = previous
                    .clone()
                    .filter(|r| !r.deleted)
                    .ok_or_else(|| error(ErrorCode::NotFound, "resource not found"))?;
                match c.name.as_str() {
                    "update" => {
                        definition.validate(&c.input, true)?;
                        for (name, value) in c.input.as_object().expect("validated object") {
                            resource.data[name] = value.clone();
                        }
                    }
                    "delete" => resource.deleted = true,
                    name => {
                        let action = definition
                            .actions
                            .iter()
                            .find(|a| a.name == name)
                            .ok_or_else(|| error(ErrorCode::Invalid, "action not declared"))?;
                        (action.apply)(&mut resource.data, &c.input).map_err(|message| {
                            error(
                                ErrorCode::Invalid,
                                format!("resource {}/{}, action {name}: {message}", c.kind, c.id),
                            )
                        })?;
                    }
                }
                if !resource.deleted {
                    definition.validate(&resource.data, false)?;
                }
                resource.revision += 1;
                resource
            };
            let changed = previous
                .as_ref()
                .is_none_or(|old| old.data != current.data || old.deleted != current.deleted);
            if !changed {
                current.revision = revision;
            }
            let event_id =
                if changed {
                    tx.execute(
                    "INSERT INTO resources(kind,id,revision,data,deleted) VALUES(?1,?2,?3,?4,?5)
                     ON CONFLICT(kind,id) DO UPDATE SET revision=excluded.revision,
                     data=excluded.data,deleted=excluded.deleted",
                    params![current.kind, current.id, current.revision,
                        serde_json::to_string(&current.data)?, current.deleted],
                )?;
                    tx.execute(
                        "INSERT INTO events(action,resource) VALUES(?1,?2)",
                        params![c.name, serde_json::to_string(&current)?],
                    )?;
                    Some(tx.last_insert_rowid())
                } else {
                    None
                };
            tx.commit()?;
            if changed {
                invalidate(state, inner);
            }
            Ok(Outcome {
                resource: current,
                event_id,
                generation: state.generation,
            })
        })
        .await
    }
    pub async fn get(&self, actor: &Actor, kind: &str, id: &str) -> Result<Resource> {
        let (actor, kind, id) = (actor.clone(), kind.to_string(), id.to_string());
        access(self.inner.clone(), move |state, _| {
            let resource = load(&state.db, &kind, &id)?;
            authorize(
                state,
                &actor,
                Access::Read {
                    kind: &kind,
                    id: &id,
                },
                resource.as_ref(),
            )?;
            resource
                .filter(|r| !r.deleted)
                .ok_or_else(|| error(ErrorCode::NotFound, "resource not found"))
        })
        .await
    }
    pub async fn live(&self, actor: &Actor, query: Query) -> Result<Subscription> {
        let key = Key {
            actor: actor.clone(),
            query,
        };
        let copy = key.clone();
        let receiver = access(self.inner.clone(), move |state, _| {
            validate_query(state, &copy.query)?;
            // Snapshot + registration share the mutation/policy mutex. There is no gap.
            let snapshot = snapshot(state, &copy)?;
            state
                .queries
                .retain(|_, sender| sender.receiver_count() > 0);
            if let Some(sender) = state.queries.get(&copy) {
                publish(sender, Ok(snapshot));
                return Ok(sender.subscribe());
            }
            if state.queries.len() >= state.max_queries {
                return Err(error(ErrorCode::Busy, "live query capacity reached"));
            }
            let (sender, receiver) = watch::channel(Ok(snapshot));
            state.queries.insert(copy, sender);
            Ok(receiver)
        })
        .await?;
        Ok(Subscription {
            runtime: self.clone(),
            key,
            receiver,
            terminated: false,
        })
    }
    pub async fn set_authorizer(&self, authorizer: impl Authorizer) -> Result<()> {
        access(self.inner.clone(), move |state, inner| {
            state.authorizer = Arc::new(authorizer);
            state.policy_epoch += 1;
            invalidate(state, inner);
            Ok(())
        })
        .await
    }
    /// Host diagnostic barrier: wait until live queries account for changes already submitted.
    pub async fn settle_queries(&self) -> Result<()> {
        let target = *self.inner.changes.borrow();
        let mut settled = self.inner.settled.subscribe();
        loop {
            if *settled.borrow() >= target {
                return Ok(());
            }
            settled
                .changed()
                .await
                .map_err(|_| error(ErrorCode::Closed, "query worker stopped"))?;
        }
    }
    /// Authorized journal inspection for the prototype host, not a public transport.
    pub async fn journal(&self, actor: &Actor) -> Result<Vec<Event>> {
        let actor = actor.clone();
        access(self.inner.clone(), move |state, _| {
            let mut stmt = state
                .db
                .prepare("SELECT seq,action,resource FROM events ORDER BY seq")?;
            let records = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let mut events = Vec::new();
            for (seq, action, raw) in records {
                let resource: Resource = serde_json::from_str(&raw)?;
                let current = load(&state.db, &resource.kind, &resource.id)?;
                let access = Access::Read {
                    kind: &resource.kind,
                    id: &resource.id,
                };
                // History never grants access lost on the authoritative current row.
                // Also require the historical row to be visible under current policy.
                if current
                    .as_ref()
                    .is_some_and(|current| state.authorizer.allows(&actor, access, Some(current)))
                    && state.authorizer.allows(
                        &actor,
                        Access::Read {
                            kind: &resource.kind,
                            id: &resource.id,
                        },
                        Some(&resource),
                    )
                {
                    events.push(Event {
                        seq,
                        action,
                        resource,
                    });
                }
            }
            Ok(events)
        })
        .await
    }
    pub async fn active_queries(&self) -> Result<usize> {
        access(self.inner.clone(), |state, _| {
            state
                .queries
                .retain(|_, sender| sender.receiver_count() > 0);
            Ok(state.queries.len())
        })
        .await
    }
    pub async fn shutdown(&self) -> Result<()> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut state = inner
                .state
                .lock()
                .map_err(|_| error(ErrorCode::Storage, "state mutex poisoned"))?;
            state.closed = true;
            invalidate(&mut state, &inner);
            Ok(())
        })
        .await
        .map_err(join_error)??;
        let worker = self.inner.worker.lock().expect("worker mutex").take();
        if let Some(worker) = worker {
            worker.await.map_err(join_error)?;
        }
        Ok(())
    }
}
pub struct Subscription {
    runtime: Runtime,
    key: Key,
    receiver: watch::Receiver<Updates>,
    terminated: bool,
}
impl Subscription {
    /// Recheck current policy at delivery, including a snapshot buffered before policy replacement.
    pub async fn current(&self) -> Result<Snapshot> {
        let mut current = self.receiver.borrow().clone()?;
        let key = self.key.clone();
        access(self.runtime.inner.clone(), move |state, _| {
            // Buffered data can predate a change to an ownership/visibility field.
            // Refresh from authoritative state before evaluating data-dependent policy.
            if current.generation != state.generation || current.policy_epoch != state.policy_epoch
            {
                return snapshot(state, &key);
            }
            authorize(
                state,
                &key.actor,
                Access::Query {
                    kind: &key.query.kind,
                },
                None,
            )?;
            current.rows.retain(|r| {
                state.authorizer.allows(
                    &key.actor,
                    Access::Read {
                        kind: &r.kind,
                        id: &r.id,
                    },
                    Some(r),
                )
            });
            current.policy_epoch = state.policy_epoch;
            Ok(current)
        })
        .await
    }
    pub async fn changed(&mut self) -> Result<Snapshot> {
        if self.terminated {
            return Err(error(ErrorCode::Closed, "subscription terminated"));
        }
        self.receiver
            .changed()
            .await
            .map_err(|_| error(ErrorCode::Closed, "subscription terminated"))?;
        let result = self.current().await;
        if let Err(error) = &result {
            if error.code == ErrorCode::Busy {
                // Admission pressure must not consume the only pending delivery.
                self.receiver.mark_changed();
            } else {
                self.terminated = true;
            }
        }
        result
    }
    pub fn has_changed(&self) -> bool {
        self.receiver.has_changed().unwrap_or(false)
    }
}
fn validate_query(state: &State, query: &Query) -> Result<()> {
    let definition = state
        .definitions
        .get(&query.kind)
        .ok_or_else(|| error(ErrorCode::NotFound, "unknown query kind"))?;
    if !definition
        .fields
        .iter()
        .any(|f| f.name == query.field && f.kind == FieldKind::Boolean)
    {
        return Err(error(
            ErrorCode::Invalid,
            format!(
                "query {}.{}: equality probe supports declared boolean fields",
                query.kind, query.field
            ),
        ));
    }
    Ok(())
}
fn snapshot(state: &State, key: &Key) -> Result<Snapshot> {
    authorize(
        state,
        &key.actor,
        Access::Query {
            kind: &key.query.kind,
        },
        None,
    )?;
    let mut query=state.db.prepare("SELECT kind,id,revision,data,deleted FROM resources WHERE kind=?1 AND deleted=0 ORDER BY id")?;
    let raw = query
        .query_map([&key.query.kind], read_raw)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    for raw in raw {
        let resource = decode(raw)?;
        if resource.data.get(&key.query.field) == Some(&Value::Bool(key.query.equals))
            && state.authorizer.allows(
                &key.actor,
                Access::Read {
                    kind: &resource.kind,
                    id: &resource.id,
                },
                Some(&resource),
            )
        {
            rows.push(resource);
        }
    }
    let journal_cursor =
        state
            .db
            .query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
    Ok(Snapshot {
        rows,
        journal_cursor,
        generation: state.generation,
        policy_epoch: state.policy_epoch,
    })
}
fn publish(sender: &watch::Sender<Updates>, next: Updates) {
    sender.send_if_modified(|old| {
        if *old == next {
            false
        } else {
            *old = next;
            true
        }
    });
}
async fn refresh_worker(inner: Weak<Inner>, mut changes: watch::Receiver<u64>) {
    while changes.changed().await.is_ok() {
        let Some(inner) = inner.upgrade() else {
            break;
        };
        let finished = tokio::task::spawn_blocking(move || {
            let mut state = inner.state.lock().expect("query state mutex");
            if state.closed {
                state.queries.clear();
                inner.settled.send_replace(state.generation);
                return true;
            }
            let keys: Vec<_> = state.queries.keys().cloned().collect();
            for key in keys {
                let Some(sender) = state.queries.get(&key) else {
                    continue;
                };
                if sender.receiver_count() == 0 {
                    state.queries.remove(&key);
                    continue;
                }
                let next = snapshot(&state, &key);
                let terminated = next.is_err();
                publish(sender, next);
                if terminated {
                    state.queries.remove(&key);
                }
            }
            inner.settled.send_replace(state.generation);
            false
        })
        .await;
        if !matches!(finished, Ok(false)) {
            break;
        }
    }
}
type Raw = (String, String, i64, String, bool);
fn read_raw(row: &rusqlite::Row<'_>) -> rusqlite::Result<Raw> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}
fn decode((kind, id, revision, data, deleted): Raw) -> Result<Resource> {
    Ok(Resource {
        kind,
        id,
        revision,
        data: serde_json::from_str(&data)?,
        deleted,
    })
}
fn load(db: &Connection, kind: &str, id: &str) -> Result<Option<Resource>> {
    db.query_row(
        "SELECT kind,id,revision,data,deleted FROM resources WHERE kind=?1 AND id=?2",
        params![kind, id],
        read_raw,
    )
    .optional()?
    .map(decode)
    .transpose()
}

#[cfg(test)]
mod delivery_regression {
    use super::*;

    struct Visible;
    impl Authorizer for Visible {
        fn allows(&self, _: &Actor, access: Access<'_>, row: Option<&Resource>) -> bool {
            match access {
                Access::Read { .. } => row.is_some_and(|r| r.data["visible"] == true),
                _ => true,
            }
        }
    }
    resource! { Document("documents") { visible:bool } actions {} }

    #[tokio::test]
    async fn admission_pressure_keeps_pending_delivery_retryable() {
        let runtime = Runtime::builder()
            .resource(Document::definition())
            .authorizer(Visible)
            .open_sqlite(":memory:")
            .await
            .unwrap();
        let actor = Actor::new("reader");
        let mut subscription = runtime
            .live(&actor, Query::equals(Document::KIND, "visible", true))
            .await
            .unwrap();
        subscription.receiver.mark_changed();
        let permits = runtime
            .inner
            .admission
            .clone()
            .acquire_many_owned(32)
            .await
            .unwrap();
        assert_eq!(
            subscription.changed().await.unwrap_err().code,
            ErrorCode::Busy
        );
        drop(permits);
        assert!(
            tokio::time::timeout(Duration::from_secs(5), subscription.changed())
                .await
                .unwrap()
                .unwrap()
                .rows
                .is_empty()
        );
        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn forced_stale_buffer_cannot_reveal_resource_after_data_revokes_access() {
        let runtime = Runtime::builder()
            .resource(Document::definition())
            .authorizer(Visible)
            .open_sqlite(":memory:")
            .await
            .unwrap();
        let actor = Actor::new("reader");
        runtime
            .execute(
                &actor,
                Command::create(Document::KIND, "one", json!({"visible":true})),
            )
            .await
            .unwrap();
        let mut subscription = runtime
            .live(&actor, Query::equals(Document::KIND, "visible", true))
            .await
            .unwrap();
        let old = subscription.current().await.unwrap();
        assert_eq!(old.rows.len(), 1);
        runtime
            .execute(
                &actor,
                Command::update(Document::KIND, "one", 1, json!({"visible":false})),
            )
            .await
            .unwrap();
        // Force exactly the old watch buffer, regardless of query-worker scheduling.
        let (_sender, receiver) = watch::channel(Ok(old));
        subscription.receiver = receiver;
        let delivered = subscription.current().await.unwrap();
        assert!(delivered.rows.is_empty());
        assert_eq!(delivered.journal_cursor, 2);
        assert!(runtime.journal(&actor).await.unwrap().is_empty());
        runtime.shutdown().await.unwrap();
    }
}
