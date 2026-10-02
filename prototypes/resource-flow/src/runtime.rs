//! THROWAWAY generic resource runtime. SQLite is a probe choice, not a ROM decision.
use crate::declarations::{registry, Descriptor};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{broadcast, oneshot, Semaphore};

#[derive(Debug)]
pub struct Error(pub StatusCode, pub String);
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn bad(message: impl Into<String>) -> Error {
    Error(StatusCode::UNPROCESSABLE_ENTITY, message.into())
}
fn conflict(message: &str) -> Error {
    Error(StatusCode::CONFLICT, message.into())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub kind: String,
    pub id: String,
    pub owner: String,
    pub revision: i64,
    pub data: Value,
    pub deleted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub seq: i64,
    pub kind: String,
    pub id: String,
    pub owner: String,
    pub action: String,
    pub resource: Resource,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub action_id: String,
    pub expected_revision: i64,
    pub input: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub resource: Resource,
    pub changed: bool,
    pub event_id: Option<i64>,
}

#[derive(Clone)]
pub struct Runtime {
    db: Arc<Mutex<Connection>>,
    pub descriptors: Arc<Vec<Descriptor>>,
    pub hints: broadcast::Sender<()>,
    cpu: Arc<rayon::ThreadPool>,
    cpu_admission: Arc<Semaphore>,
    db_admission: Arc<Semaphore>,
    pub faults: bool,
}
impl Runtime {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(2))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
          CREATE TABLE IF NOT EXISTS resources(kind TEXT NOT NULL,id TEXT NOT NULL,owner TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,deleted INTEGER NOT NULL,PRIMARY KEY(kind,id));
          CREATE TABLE IF NOT EXISTS events(seq INTEGER PRIMARY KEY AUTOINCREMENT,kind TEXT NOT NULL,id TEXT NOT NULL,owner TEXT NOT NULL,action TEXT NOT NULL,resource TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS receipts(principal TEXT NOT NULL,kind TEXT NOT NULL,id TEXT NOT NULL,action TEXT NOT NULL,key TEXT NOT NULL,request TEXT NOT NULL,outcome TEXT NOT NULL,PRIMARY KEY(principal,kind,id,action,key));
          CREATE TABLE IF NOT EXISTS reaction_progress(name TEXT PRIMARY KEY,cursor INTEGER NOT NULL);")?;
        let descriptors = registry();
        let mut reaction_namespaces = std::collections::BTreeSet::new();
        // Fail early on declaration collisions and invalid literal action/reaction data.
        for (index, descriptor) in descriptors.iter().enumerate() {
            if descriptors[..index]
                .iter()
                .any(|d| d.kind == descriptor.kind)
            {
                return Err(bad("duplicate kind"));
            }
            for (_, patch) in &descriptor.actions {
                descriptor.validate(patch, true).map_err(bad)?;
            }
            for reaction in &descriptor.reactions {
                if reaction.id_prefix.is_empty()
                    || reaction.id_prefix.len() > 64
                    || reaction.name.is_empty()
                    || reaction.name.len() > 64
                    || !reaction_namespaces.insert(reaction.id_prefix)
                {
                    return Err(bad(
                        "reaction names must be bounded and ID namespaces unique",
                    ));
                }
                let target = descriptors
                    .iter()
                    .find(|d| d.kind == reaction.target_kind)
                    .ok_or_else(|| bad("unknown reaction target"))?;
                target.validate(&reaction.input, false).map_err(bad)?;
            }
        }
        let (hints, _) = broadcast::channel(16);
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            descriptors: Arc::new(descriptors),
            hints,
            cpu: Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(2)
                    .build()
                    .map_err(|e| bad(e.to_string()))?,
            ),
            cpu_admission: Arc::new(Semaphore::new(8)),
            db_admission: Arc::new(Semaphore::new(8)),
            faults: std::env::var("PROTOTYPE_FAULTS").as_deref() == Ok("1"),
        })
    }
    pub fn descriptor(&self, kind: &str) -> Result<Descriptor> {
        self.descriptors
            .iter()
            .find(|d| d.kind == kind)
            .cloned()
            .ok_or_else(|| Error(StatusCode::NOT_FOUND, "unknown kind".into()))
    }
    async fn with_db<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let permit = self.db_admission.clone().try_acquire_owned().map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "database admission full".into(),
            )
        })?;
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit; // Retained until actual work ends even if caller disappears.
            let mut connection = db.lock().map_err(|_| {
                Error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database mutex poisoned".into(),
                )
            })?;
            f(&mut connection)
        })
        .await
        .map_err(|e| Error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    }
    async fn validate_cpu(
        &self,
        descriptor: Descriptor,
        value: Value,
        partial: bool,
    ) -> Result<()> {
        // Deliberately demonstrates the extension boundary; tiny field checks do not need Rayon.
        let permit = self
            .cpu_admission
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error(StatusCode::SERVICE_UNAVAILABLE, "CPU admission full".into()))?;
        let (tx, rx) = oneshot::channel();
        self.cpu.spawn(move || {
            let _permit = permit;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                descriptor.validate(&value, partial)
            }))
            .unwrap_or_else(|_| Err("field extension panicked".into()));
            let _ = tx.send(result);
        });
        rx.await
            .map_err(|_| bad("validation worker stopped"))?
            .map_err(bad)
    }
    pub async fn execute(
        &self,
        principal: String,
        kind: String,
        id: String,
        verb: String,
        request: Action,
        inject_failure: bool,
    ) -> Result<Outcome> {
        let descriptor = self.descriptor(&kind)?;
        if request.action_id.is_empty()
            || request.action_id.len() > 128
            || id.is_empty()
            || id.len() > 128
            || request.expected_revision < 0
        {
            return Err(bad("invalid action identity or revision"));
        }
        let patch = match verb.as_str() {
            "create" | "set" => request.input.clone(),
            "delete" => {
                if request.input != json!({}) {
                    return Err(bad("delete input must be empty"));
                }
                json!({})
            }
            action => {
                if request.input != json!({}) {
                    return Err(bad("literal action accepts empty input only"));
                }
                descriptor
                    .actions
                    .iter()
                    .find(|(name, _)| *name == action)
                    .map(|(_, p)| p.clone())
                    .ok_or_else(|| bad("unknown action"))?
            }
        };
        self.validate_cpu(descriptor.clone(), patch.clone(), verb != "create")
            .await?;
        let hints = self.hints.clone();
        let drop_hint = self.faults && std::env::var("PROTOTYPE_DROP_HINTS").as_deref() == Ok("1");
        self.with_db(move |db| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let previous = load(&tx, &kind, &id)?;
            if previous.as_ref().is_some_and(|r| r.owner != principal) { return Err(Error(StatusCode::FORBIDDEN, "resource owned by another principal".into())); }
            let fingerprint = serde_json::to_string(&request)?;
            let cached: Option<(String, String)> = tx.query_row("SELECT request,outcome FROM receipts WHERE principal=?1 AND kind=?2 AND id=?3 AND action=?4 AND key=?5", params![principal,kind,id,verb,request.action_id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
            if let Some((old_request, outcome)) = cached {
                if old_request != fingerprint { return Err(conflict("action ID reused with different input")); }
                return Ok(serde_json::from_str(&outcome)?);
            }
            let current_revision = previous.as_ref().map_or(0, |r| r.revision);
            if current_revision != request.expected_revision { return Err(conflict("stale expected revision")); }
            let resource = if verb == "create" {
                if previous.is_some() { return Err(conflict("identity already exists (including tombstones)")); }
                Resource {kind:kind.clone(), id:id.clone(), owner:principal.clone(), revision:1, data:patch, deleted:false}
            } else {
                let mut current = previous.clone().filter(|r| !r.deleted).ok_or_else(|| Error(StatusCode::NOT_FOUND,"resource not found".into()))?;
                if verb == "delete" { current.deleted = true; } else {
                    let object = current.data.as_object_mut().ok_or_else(|| bad("stored data is not an object"))?;
                    for (name, value) in patch.as_object().ok_or_else(|| bad("patch must be object"))? { object.insert(name.clone(),value.clone()); }
                    // Cross-field/merged validation would belong here, inside authoritative transaction.
                    descriptor.validate(&current.data,false).map_err(bad)?;
                }
                current.revision += 1;
                current
            };
            let changed = previous.as_ref().map_or(true, |old| old.data != resource.data || old.deleted != resource.deleted);
            let mut resource = resource;
            if !changed { resource.revision = current_revision; }
            let event_id = if changed {
                let serialized = serde_json::to_string(&resource.data)?;
                if previous.is_none() {
                    tx.execute("INSERT INTO resources(kind,id,owner,revision,data,deleted) VALUES(?1,?2,?3,?4,?5,?6)",params![kind,id,principal,resource.revision,serialized,resource.deleted])?;
                } else {
                    let rows=tx.execute("UPDATE resources SET revision=?1,data=?2,deleted=?3 WHERE kind=?4 AND id=?5 AND revision=?6",params![resource.revision,serialized,resource.deleted,kind,id,current_revision])?;
                    if rows != 1 { return Err(conflict("revision changed")); }
                }
                tx.execute("INSERT INTO events(kind,id,owner,action,resource) VALUES(?1,?2,?3,?4,?5)",params![kind,id,principal,verb,serde_json::to_string(&resource)?])?;
                Some(tx.last_insert_rowid())
            } else { None };
            let outcome = Outcome {resource,changed,event_id};
            tx.execute("INSERT INTO receipts(principal,kind,id,action,key,request,outcome) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![principal,kind,id,verb,request.action_id,fingerprint,serde_json::to_string(&outcome)?])?;
            if inject_failure { return Err(Error(StatusCode::INTERNAL_SERVER_ERROR,"injected failure after state/event/receipt SQL before commit".into())); }
            tx.commit()?;
            if changed && !drop_hint { let _ = hints.send(()); } // Hint only after commit, never durable authority.
            Ok(outcome)
        }).await
    }
    pub async fn get(&self, principal: String, kind: String, id: String) -> Result<Resource> {
        self.descriptor(&kind)?;
        self.with_db(move |db| {
            let resource = load(db, &kind, &id)?
                .filter(|r| !r.deleted)
                .ok_or_else(|| Error(StatusCode::NOT_FOUND, "resource not found".into()))?;
            if resource.owner != principal {
                return Err(Error(
                    StatusCode::FORBIDDEN,
                    "resource owned by another principal".into(),
                ));
            }
            Ok(resource)
        })
        .await
    }
    pub async fn list(&self, principal: String, kind: String) -> Result<Vec<Resource>> {
        self.descriptor(&kind)?;
        self.with_db(move |db| {
            let mut query=db.prepare("SELECT kind,id,owner,revision,data,deleted FROM resources WHERE kind=?1 AND owner=?2 AND deleted=0 ORDER BY id")?;
            let records=query.query_map(params![kind,principal],row_resource)?.collect::<std::result::Result<Vec<_>,_>>()?;
            records.into_iter().map(decode_resource).collect()
        }).await
    }
    pub async fn events(&self, principal: Option<String>, after: i64) -> Result<Vec<Event>> {
        self.with_db(move |db| {
            let mut query=db.prepare("SELECT seq,kind,id,owner,action,resource FROM events WHERE seq>?1 AND (?2 IS NULL OR owner=?2) ORDER BY seq LIMIT 128")?;
            let rows=query.query_map(params![after,principal],|r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
            rows.into_iter().map(|(seq,kind,id,owner,action,payload)|Ok(Event{seq,kind,id,owner,action,resource:serde_json::from_str(&payload)?})).collect()
        }).await
    }
    pub async fn react_once(&self) -> Result<usize> {
        let cursor = self
            .with_db(|db| {
                Ok(db
                    .query_row(
                        "SELECT cursor FROM reaction_progress WHERE name='declarations-v1'",
                        [],
                        |r| r.get::<_, i64>(0),
                    )
                    .optional()?
                    .unwrap_or(0))
            })
            .await?;
        let events = self.events(None, cursor).await?;
        for event in &events {
            let descriptor = self.descriptor(&event.kind)?;
            if !event.resource.deleted && event.action != "create" {
                for reaction in descriptor.reactions {
                    if event.resource.data.get(reaction.when_field) == Some(&reaction.equals) {
                        // Journal sequence is globally unique; source IDs need not fit inside target IDs.
                        let id = format!("{}-{}", reaction.id_prefix, event.seq);
                        self.execute(
                            event.owner.clone(),
                            reaction.target_kind.into(),
                            id,
                            "create".into(),
                            Action {
                                action_id: format!("reaction:{}:{}", reaction.name, event.seq),
                                expected_revision: 0,
                                input: reaction.input,
                            },
                            false,
                        )
                        .await?;
                        if self.faults
                            && std::env::var("PROTOTYPE_REACTION_CRASH_AFTER_COMMIT").as_deref()
                                == Ok("1")
                        {
                            eprintln!("PROTOTYPE: exiting after reaction action commit, before cursor checkpoint");
                            std::process::exit(86);
                        }
                    }
                }
            }
            let seq = event.seq;
            self.with_db(move |db|{ db.execute("INSERT INTO reaction_progress(name,cursor) VALUES('declarations-v1',?1) ON CONFLICT(name) DO UPDATE SET cursor=excluded.cursor",[seq])?; Ok(()) }).await?;
        }
        Ok(events.len())
    }
}
type RawResource = (String, String, String, i64, String, bool);
fn row_resource(r: &rusqlite::Row<'_>) -> rusqlite::Result<RawResource> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
    ))
}
fn decode_resource(raw: RawResource) -> Result<Resource> {
    let (kind, id, owner, revision, data, deleted) = raw;
    Ok(Resource {
        kind,
        id,
        owner,
        revision,
        data: serde_json::from_str(&data)?,
        deleted,
    })
}
fn load(db: &Connection, kind: &str, id: &str) -> Result<Option<Resource>> {
    db.query_row(
        "SELECT kind,id,owner,revision,data,deleted FROM resources WHERE kind=?1 AND id=?2",
        params![kind, id],
        row_resource,
    )
    .optional()?
    .map(decode_resource)
    .transpose()
}
