//! DISPOSABLE probe, not ROM production code or an adopted public API.
//! Compiled Resource definitions are authoritative; loaders supply only values.
use figment::{
    Figment,
    providers::{Format, Json},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub enum Loader {
    Config,
    Figment,
}

impl Loader {
    pub fn merge(self, layers: &[Value]) -> Result<Value, String> {
        match self {
            Self::Config => {
                let mut builder = config::Config::builder();
                for layer in layers {
                    builder = builder.add_source(config::File::from_str(
                        &layer.to_string(),
                        config::FileFormat::Json,
                    ));
                }
                builder
                    .build()
                    .and_then(|v| v.try_deserialize())
                    .map_err(|_| "load".into())
            }
            Self::Figment => {
                let mut figment = Figment::new();
                for layer in layers {
                    figment = figment.merge(Json::string(&layer.to_string()));
                }
                figment.extract().map_err(|_| "load".into())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    pub kind: String,
    pub key: String,
}
impl Id {
    pub fn new(kind: &str, key: &str) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
        }
    }
}
#[derive(Clone, Copy)]
enum FieldType {
    Bool,
    Text,
    TextList,
}
#[derive(Clone, Copy)]
struct Field {
    name: &'static str,
    ty: FieldType,
    nullable: bool,
}
// The same registry and validator serve source imports and interactive writes.
// There is no schema declaration in a source document and no per-kind mutation path.
fn definition(kind: &str) -> Result<&'static [Field], &'static str> {
    use FieldType::*;
    match kind {
        "User" => Ok(&[
            Field {
                name: "name",
                ty: Text,
                nullable: false,
            },
            Field {
                name: "enabled",
                ty: Bool,
                nullable: false,
            },
        ]),
        "AppSettings" => Ok(&[
            Field {
                name: "title",
                ty: Text,
                nullable: false,
            },
            Field {
                name: "note",
                ty: Text,
                nullable: true,
            },
        ]),
        "IdentityProvider" => Ok(&[
            Field {
                name: "issuer",
                ty: Text,
                nullable: false,
            },
            Field {
                name: "audiences",
                ty: TextList,
                nullable: false,
            },
            Field {
                name: "secret_ref",
                ty: Text,
                nullable: true,
            },
        ]),
        _ => Err("unknown-kind"),
    }
}
fn validate(id: &Id, fields: &Value, complete: bool) -> Result<(), &'static str> {
    if id.key.is_empty() {
        return Err("empty-id");
    }
    let definition = definition(&id.kind)?;
    let object = fields.as_object().ok_or("object-required")?;
    for (key, value) in object {
        let field = definition
            .iter()
            .find(|f| f.name == key)
            .ok_or("unknown-field")?;
        let valid = if value.is_null() {
            field.nullable
        } else {
            match field.ty {
                FieldType::Bool => value.is_boolean(),
                FieldType::Text => value.is_string(),
                FieldType::TextList => value
                    .as_array()
                    .is_some_and(|a| a.iter().all(Value::is_string)),
            }
        };
        if !valid {
            return Err("invalid-field");
        }
        if key == "issuer" && !value.as_str().is_some_and(|s| s.starts_with("https://")) {
            return Err("invalid-issuer");
        }
        if key == "secret_ref"
            && !value.is_null()
            && !value.as_str().is_some_and(|s| s.starts_with("secret://"))
        {
            return Err("secret-reference-required");
        }
        if key == "audiences" && value.as_array().is_some_and(Vec::is_empty) {
            return Err("empty-audiences");
        }
    }
    if complete
        && definition
            .iter()
            .any(|f| !f.nullable && !object.contains_key(f.name))
    {
        return Err("missing-required-field");
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Grant {
    pub kinds: Vec<String>,
    pub priority: u32,
    pub trust_admin: bool,
    pub overlay: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Values(Value),
    Delete,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot(pub BTreeMap<Id, Input>);
impl Snapshot {
    pub fn one(id: Id, fields: Value) -> Self {
        Self(BTreeMap::from([(id, Input::Values(fields))]))
    }
    // A supporting import envelope; "kind" selects an accepted Resource, never defines one.
    pub fn decode(bytes: &str) -> Result<Self, &'static str> {
        let rows: Value = serde_json::from_str(bytes).map_err(|_| "parse")?;
        let mut out = BTreeMap::new();
        for row in rows.as_array().ok_or("envelope-array")? {
            let obj = row.as_object().ok_or("envelope-object")?;
            if obj
                .keys()
                .any(|k| !["kind", "id", "values", "delete"].contains(&k.as_str()))
            {
                return Err("unknown-envelope-field");
            }
            let id = Id::new(
                row["kind"].as_str().ok_or("kind")?,
                row["id"].as_str().ok_or("id")?,
            );
            let input = match (obj.get("values"), obj.get("delete")) {
                (Some(v), None) => Input::Values(v.clone()),
                (None, Some(Value::Bool(true))) => Input::Delete,
                _ => return Err("ambiguous-operation"),
            };
            if out.insert(id, input).is_some() {
                return Err("duplicate-id");
            }
        }
        Ok(Self(out))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Resource {
    pub fields: Value,
    pub revision: u64,
    pub winners: BTreeMap<String, String>,
}
#[derive(Clone, Debug)]
struct Layer {
    sequence: u64,
    snapshot: Snapshot,
}
#[derive(Clone, Debug)]
pub struct Ticket {
    source: String,
    sequence: u64,
    base: u64,
}
#[derive(Clone, Debug)]
pub struct Candidate {
    ticket: Ticket,
    layer: Layer,
    resources: BTreeMap<Id, Resource>,
}
#[derive(Debug)]
pub struct Runtime {
    loader: Loader,
    grants: BTreeMap<String, Grant>,
    latest: BTreeMap<String, u64>,
    layers: BTreeMap<String, Layer>,
    pub resources: BTreeMap<Id, Resource>,
    pub generation: u64,
    pub status: &'static str,
    sealed: bool,
    pub valid_until: u64,
}
impl Runtime {
    pub fn new(loader: Loader, grants: BTreeMap<String, Grant>) -> Result<Self, &'static str> {
        let mut priorities = std::collections::BTreeSet::new();
        if grants.values().any(|g| !priorities.insert(g.priority)) {
            return Err("duplicate-priority");
        }
        Ok(Self {
            loader,
            grants,
            latest: BTreeMap::new(),
            layers: BTreeMap::new(),
            resources: BTreeMap::new(),
            generation: 0,
            status: "uninitialized",
            sealed: false,
            valid_until: 100,
        })
    }
    // Source identity is supplied by the host, never trusted from the document.
    pub fn begin(&mut self, source: &str) -> Result<Ticket, &'static str> {
        if !self.grants.contains_key(source) {
            return Err("unauthorized-source");
        }
        if source == "bootstrap" && self.sealed {
            return Err("bootstrap-ended");
        }
        let sequence = self.latest.entry(source.into()).or_default();
        *sequence += 1;
        Ok(Ticket {
            source: source.into(),
            sequence: *sequence,
            base: self.generation,
        })
    }
    pub fn prepare(&self, ticket: Ticket, snapshot: Snapshot) -> Result<Candidate, &'static str> {
        let grant = &self.grants[&ticket.source];
        for (id, input) in &snapshot.0 {
            definition(&id.kind)?;
            if !grant.kinds.contains(&id.kind)
                || (id.kind == "IdentityProvider" && !grant.trust_admin)
            {
                return Err("forbidden-kind");
            }
            if let Some(old) = self.resources.get(id) {
                let owned_elsewhere = old.winners.values().any(|s| s != &ticket.source);
                if owned_elsewhere && (!grant.overlay || matches!(input, Input::Delete)) {
                    return Err("ownership-conflict");
                }
            }
            if let Input::Values(fields) = input {
                validate(id, fields, false)?;
            }
        }
        let layer = Layer {
            sequence: ticket.sequence,
            snapshot,
        };
        let mut layers = self.layers.clone();
        layers.insert(ticket.source.clone(), layer.clone());
        let mut ordered: Vec<_> = layers.iter().collect();
        ordered.sort_by_key(|(source, _)| self.grants[*source].priority);
        let mut values: BTreeMap<Id, Vec<Value>> = BTreeMap::new();
        let mut winners: BTreeMap<Id, BTreeMap<String, String>> = BTreeMap::new();
        for (source, layer) in ordered {
            for (id, input) in &layer.snapshot.0 {
                match input {
                    Input::Delete => {
                        values.remove(id);
                        winners.remove(id);
                    }
                    Input::Values(v) => {
                        values.entry(id.clone()).or_default().push(v.clone());
                        for key in v.as_object().ok_or("object-required")?.keys() {
                            winners
                                .entry(id.clone())
                                .or_default()
                                .insert(key.clone(), source.clone());
                        }
                    }
                }
            }
        }
        let mut resources = BTreeMap::new();
        for (id, layers) in values {
            let fields = self.loader.merge(&layers).map_err(|_| "load")?;
            validate(&id, &fields, true)?;
            let revision = self
                .resources
                .get(&id)
                .map_or(1, |r| r.revision + u64::from(r.fields != fields));
            resources.insert(
                id.clone(),
                Resource {
                    fields,
                    revision,
                    winners: winners.remove(&id).unwrap_or_default(),
                },
            );
        }
        // Minimal cross-resource startup invariant, deliberately not production trust validation.
        if !resources.keys().any(|id| id.kind == "IdentityProvider") {
            return Err("missing-trust-resource");
        }
        Ok(Candidate {
            ticket,
            layer,
            resources,
        })
    }
    pub fn publish(&mut self, candidate: Candidate) -> Result<(), &'static str> {
        let t = &candidate.ticket;
        if self.latest.get(&t.source) != Some(&t.sequence) {
            return Err("late-completion");
        }
        if self.generation != t.base {
            return Err("revision-conflict");
        }
        if t.source == "bootstrap" && self.sealed {
            return Err("bootstrap-ended");
        }
        let unchanged = self
            .layers
            .get(&t.source)
            .is_some_and(|l| l.snapshot == candidate.layer.snapshot);
        self.layers.insert(t.source.clone(), candidate.layer);
        self.resources = candidate.resources;
        if !unchanged {
            self.generation += 1;
        }
        self.status = "active";
        self.sealed = true;
        Ok(())
    }
    pub fn reload(
        &mut self,
        source: &str,
        input: Result<Snapshot, &'static str>,
    ) -> Result<(), &'static str> {
        let result = self
            .begin(source)
            .and_then(|t| input.and_then(|s| self.prepare(t, s)))
            .and_then(|c| self.publish(c));
        if result.is_err() {
            self.status = "rejected";
        }
        result
    }
    pub fn read(&self, id: &Id, now: u64) -> Result<&Resource, &'static str> {
        if self.generation == 0 {
            return Err("not-ready");
        }
        if now >= self.valid_until {
            return Err("expired-active-state");
        }
        self.resources.get(id).ok_or("not-found")
    }
    pub fn provenance(&self, id: &Id, field: &str) -> Option<(&str, u64)> {
        let source = self.resources.get(id)?.winners.get(field)?;
        Some((source, self.layers.get(source)?.sequence))
    }
}

pub fn fixture(loader: Loader) -> Runtime {
    let all = vec![
        "User".into(),
        "AppSettings".into(),
        "IdentityProvider".into(),
    ];
    let grants = BTreeMap::from([
        (
            "bootstrap".into(),
            Grant {
                kinds: all.clone(),
                priority: 0,
                trust_admin: true,
                overlay: false,
            },
        ),
        (
            "deployment".into(),
            Grant {
                kinds: all.clone(),
                priority: 1,
                trust_admin: true,
                overlay: true,
            },
        ),
        (
            "studio".into(),
            Grant {
                kinds: all.clone(),
                priority: 2,
                trust_admin: true,
                overlay: true,
            },
        ),
        (
            "project".into(),
            Grant {
                kinds: all,
                priority: 3,
                trust_admin: false,
                overlay: false,
            },
        ),
    ]);
    Runtime::new(loader, grants).unwrap()
}
pub fn seed() -> Snapshot {
    Snapshot(BTreeMap::from([
        (
            Id::new("User", "alice.example[0]"),
            Input::Values(json!({"name":"Alice", "enabled":true})),
        ),
        (
            Id::new("AppSettings", "app"),
            Input::Values(json!({"title":"base", "note":"inherited"})),
        ),
        (
            Id::new("IdentityProvider", "auth"),
            Input::Values(
                json!({"issuer":"https://issuer.example", "audiences":["rom"], "secret_ref":"secret://auth/client"}),
            ),
        ),
    ]))
}

/// ROM-owned allowlist and descriptor-aware decoding over a captured environment.
/// Never reads process-wide environment; no crate's heuristic inference is enabled.
pub fn environment_snapshot(entries: &[(&str, &str)]) -> Result<Snapshot, &'static str> {
    let mut fields = serde_json::Map::new();
    for (name, value) in entries {
        if !name.to_ascii_uppercase().starts_with("ROM_APP_") {
            continue;
        }
        let field = match name.to_ascii_uppercase().as_str() {
            "ROM_APP_TITLE" => "title",
            "ROM_APP_NOTE" => "note",
            _ => return Err("environment-not-allowlisted"),
        };
        if fields
            .insert(field.into(), Value::String((*value).into()))
            .is_some()
        {
            return Err("environment-alias-collision");
        }
    }
    Ok(Snapshot::one(
        Id::new("AppSettings", "app"),
        Value::Object(fields),
    ))
}
