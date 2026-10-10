//! Disposable loopback test authority. These routes are never a product API.
use crate::storage::FixtureStorage;
use axum::{Json, Router, extract::State, routing::post};
use rom::{Actor, Key, PrincipalKind, json};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Clone)]
pub struct Fixture {
    pub storage: Arc<FixtureStorage>,
    pub generation: Arc<AtomicU64>,
    pub revoked: Arc<AtomicBool>,
}

struct FixtureGate {
    generation: Arc<AtomicU64>,
    revoked: Arc<AtomicBool>,
}
impl rom::ActorGate for FixtureGate {
    fn check(&self, actor: &Actor, _: &mut dyn rom::AuthorizationRead) -> rom::Result<()> {
        let stamp = format!("fixture-session-{}", self.generation.load(Ordering::SeqCst));
        if actor.authority != "recovery-fixture"
            || actor.principal_kind() != PrincipalKind::Human
            || actor.host_stamp() != Some(stamp.as_str())
            || self.revoked.load(Ordering::SeqCst)
        {
            return Err(rom::Error::Denied);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    id: String,
    idempotency: String,
    operation: String,
}

async fn inspect(State(fixture): State<Fixture>, Json(probe): Json<Probe>) -> Json<Value> {
    assert!(probe.id.len() <= 128 && probe.idempotency.len() <= 128);
    let key = Key {
        kind: "recovery-notes".into(),
        id: probe.id.clone(),
    };
    let tag = match probe.operation.as_str() {
        "save" => json!(["custom", "save"]),
        "patch" => json!(["standard", "patch"]),
        "delete" => json!(["standard", "delete"]),
        "create" => json!(["standard", "create"]),
        _ => return Json(json!({"error":"unsupported fixture probe"})),
    };
    // Explicit expected fixture identity, checked against the adapter's real receipt table.
    let identity = json!([
        "recovery-fixture",
        "human",
        "alice",
        "recovery-notes",
        probe.id,
        tag,
        probe.idempotency
    ])
    .to_string();
    let store = &fixture.storage.store;
    let row = store.load(&key).expect("fixture row");
    let receipt = store.receipt(&identity).expect("fixture receipt");
    let events = store
        .journal("recovery-notes", None, 1000, 1048576)
        .expect("fixture journal")
        .events
        .into_iter()
        .filter(|event| event.row.key == key)
        .count();
    Json(
        json!({"counts": (fixture.storage.counts)().expect("fixture counts"), "row": row,
        "receipt":receipt,"events_for_id":events,"expected_identity":identity,
        "generation":fixture.generation.load(Ordering::SeqCst),"revoked":fixture.revoked.load(Ordering::SeqCst)}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    generation: Option<u64>,
    revoked: Option<bool>,
}

async fn control(State(fixture): State<Fixture>, Json(control): Json<Control>) -> Json<Value> {
    if let Some(value) = control.generation {
        fixture.generation.store(value, Ordering::SeqCst);
    }
    if let Some(value) = control.revoked {
        fixture.revoked.store(value, Ordering::SeqCst);
    }
    Json(
        json!({"generation":fixture.generation.load(Ordering::SeqCst),"revoked":fixture.revoked.load(Ordering::SeqCst)}),
    )
}

impl Fixture {
    pub fn gate(&self) -> Arc<dyn rom::ActorGate> {
        Arc::new(FixtureGate {
            generation: self.generation.clone(),
            revoked: self.revoked.clone(),
        })
    }
    pub fn resolver(&self) -> rom_http::AuthResolver {
        let generation = self.generation.clone();
        Arc::new(move |headers| {
            let current = generation.load(Ordering::SeqCst);
            let owner = format!("Bearer fixture-owner-{current}");
            let other = format!("Bearer fixture-other-{current}");
            let csrf = format!("fixture-csrf-{current}");
            if headers.get("x-rom-csrf").and_then(|v| v.to_str().ok()) != Some(csrf.as_str()) {
                return Err(rom::Error::Denied);
            }
            let subject = match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some(value) if value == owner => "alice",
                Some(value) if value == other => "bob",
                _ => return Err(rom::Error::Denied),
            };
            Ok(Actor::trusted("recovery-fixture", subject)
                .with_kind(PrincipalKind::Human)
                .with_host_stamp(&format!("fixture-session-{current}")))
        })
    }
    pub fn router(self) -> Router {
        Router::new()
            .route("/__fixture/inspect", post(inspect))
            .route("/__fixture/control", post(control))
            .with_state(self)
    }
}
