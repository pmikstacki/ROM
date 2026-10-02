//! DISPOSABLE transport-free core seam. No network, serde, crypto or policy-engine dependency.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrincipalKind {
    Human,
    Service,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Principal {
    pub authority: String,
    pub subject: String,
    pub kind: PrincipalKind,
}
/// Private fields; ordinary request data cannot deserialize or construct a trusted Actor.
/// ```compile_fail
/// use rom_auth_probe_core::{Actor, Principal, PrincipalKind};
/// let forged = Actor { principal: Principal { authority: "issuer".into(), subject: "admin".into(), kind: PrincipalKind::Human }, valid_until: u64::MAX, tenant:"tenant-a".into() };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    principal: Principal,
    valid_until: u64,
    tenant: String,
}
impl Actor {
    pub fn principal(&self) -> &Principal {
        &self.principal
    }
    pub fn valid_until(&self) -> u64 {
        self.valid_until
    }
}
/// Explicit trusted native-host capability. Possession is not a sandbox against host code.
/// Transport drivers never receive this value; configured verifier adapters retain it.
pub struct ActorIssuer {
    authority: String,
    tenant: String,
}
impl ActorIssuer {
    pub fn configured_by_host(authority: &str, tenant: &str) -> Self {
        Self {
            authority: authority.into(),
            tenant: tenant.into(),
        }
    }
    pub fn verified(&self, subject: String, kind: PrincipalKind, valid_until: u64) -> Actor {
        Actor {
            principal: Principal {
                authority: self.authority.clone(),
                subject,
                kind,
            },
            valid_until,
            tenant: self.tenant.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Bool(bool),
    Bindings(Vec<Principal>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub id: String,
    pub kind: String,
    pub tenant: String,
    pub fields: BTreeMap<String, Value>,
    pub revision: u64,
}
impl Resource {
    pub fn new(id: &str, kind: &str, fields: BTreeMap<String, Value>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            tenant: "tenant-a".into(),
            fields,
            revision: 1,
        }
    }
}
fn validate(r: &Resource) -> Result<(), &'static str> {
    let defs: &[(&str, &str)] = match r.kind.as_str() {
        "User" => &[
            ("name", "text"),
            ("email", "text"),
            ("enabled", "bool"),
            ("identities", "bindings"),
        ],
        "IdentityProvider" => &[("issuer", "text"), ("enabled", "bool")],
        "AppSettings" => &[("title", "text")],
        _ => return Err("unknown-kind"),
    };
    for (key, value) in &r.fields {
        let ty = defs
            .iter()
            .find(|(name, _)| key == name)
            .ok_or("unknown-field")?
            .1;
        if !matches!(
            (ty, value),
            ("text", Value::Text(_)) | ("bool", Value::Bool(_)) | ("bindings", Value::Bindings(_))
        ) {
            return Err("invalid-field");
        }
    }
    if defs.iter().any(|(name, _)| !r.fields.contains_key(*name)) {
        return Err("missing-field");
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
    Deliver,
}
pub struct Context<'a> {
    pub actor: &'a Actor,
    pub linked_user: Option<&'a str>,
    pub operation: Operation,
    pub resource: &'a Resource,
    pub fields: &'a [String],
}
pub trait Authorizer {
    fn decide(&self, context: Context<'_>) -> Result<bool, &'static str>;
}
#[derive(Default)]
pub struct LocalPolicy {
    pub admins: Vec<Principal>,
    pub revoked: Vec<Principal>,
    pub fail: bool,
}
impl Authorizer for LocalPolicy {
    fn decide(&self, c: Context<'_>) -> Result<bool, &'static str> {
        if self.fail {
            return Err("policy-error");
        }
        if self.revoked.contains(c.actor.principal()) {
            return Ok(false);
        }
        if self.admins.contains(c.actor.principal()) {
            return Ok(true);
        }
        Ok(c.resource.kind == "User"
            && c.linked_user == Some(c.resource.id.as_str())
            && c.fields
                .iter()
                .all(|f| f == "name" || (c.operation != Operation::Write && f == "email")))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub revision: u64,
}
#[derive(Clone, Debug)]
struct StoredReceipt {
    expected_revision: u64,
    input: BTreeMap<String, Value>,
    result: Receipt,
}
#[derive(Debug)]
pub struct Core {
    resources: BTreeMap<String, Resource>,
    receipts: BTreeMap<(Principal, String, String), StoredReceipt>,
    pub events: Vec<(String, u64, Principal)>,
}
impl Core {
    /// Trusted bootstrap input goes through the same compiled Resource validation.
    pub fn bootstrap(resources: Vec<Resource>) -> Result<Self, &'static str> {
        let mut map = BTreeMap::new();
        for r in resources {
            validate(&r)?;
            if map.insert(r.id.clone(), r).is_some() {
                return Err("duplicate-id");
            }
        }
        let out = Self {
            resources: map,
            receipts: BTreeMap::new(),
            events: Vec::new(),
        };
        out.check_links()?;
        Ok(out)
    }
    fn check_links(&self) -> Result<(), &'static str> {
        let mut seen = std::collections::BTreeSet::new();
        for r in self.resources.values().filter(|r| r.kind == "User") {
            if let Some(Value::Bindings(bindings)) = r.fields.get("identities") {
                for b in bindings {
                    if !seen.insert(b) {
                        return Err("duplicate-binding");
                    }
                }
            }
        }
        Ok(())
    }
    fn user(&self, actor: &Actor) -> Result<Option<&str>, &'static str> {
        if actor.principal.kind == PrincipalKind::Service {
            return Ok(None);
        }
        let user=self.resources.values().find(|r|r.kind=="User" && matches!(r.fields.get("identities"),Some(Value::Bindings(b)) if b.contains(actor.principal()))).ok_or("unlinked-identity")?;
        if user.fields.get("enabled") != Some(&Value::Bool(true)) {
            return Err("disabled-user");
        }
        Ok(Some(&user.id))
    }
    fn authorize(
        &self,
        actor: &Actor,
        policy: &dyn Authorizer,
        id: &str,
        op: Operation,
        fields: &[String],
        now: u64,
    ) -> Result<&Resource, &'static str> {
        if now >= actor.valid_until {
            return Err("expired-actor");
        }
        // Tenant is trusted host context; this probe is deliberately single tenant.
        if actor.tenant != "tenant-a" {
            return Err("tenant-denied");
        }
        let linked_user = self.user(actor)?;
        let r = self.resources.get(id).ok_or("not-found")?;
        if r.tenant != actor.tenant {
            return Err("tenant-denied");
        }
        match policy.decide(Context {
            actor,
            linked_user,
            operation: op,
            resource: r,
            fields,
        }) {
            Ok(true) => Ok(r),
            _ => Err("denied"),
        }
    }
    pub fn read(
        &self,
        actor: &Actor,
        policy: &dyn Authorizer,
        id: &str,
        fields: &[String],
        now: u64,
    ) -> Result<BTreeMap<String, Value>, &'static str> {
        let r = self.authorize(actor, policy, id, Operation::Read, fields, now)?;
        fields
            .iter()
            .map(|f| Ok((f.clone(), r.fields.get(f).ok_or("unknown-field")?.clone())))
            .collect()
    }
    pub fn deliver(
        &self,
        actor: &Actor,
        policy: &dyn Authorizer,
        id: &str,
        fields: &[String],
        now: u64,
    ) -> Result<(), &'static str> {
        self.authorize(actor, policy, id, Operation::Deliver, fields, now)
            .map(|_| ())
    }
    pub fn patch(
        &mut self,
        actor: &Actor,
        policy: &dyn Authorizer,
        request: Patch<'_>,
        now: u64,
    ) -> Result<Receipt, &'static str> {
        let fields: Vec<_> = request.fields.keys().cloned().collect();
        let r = self.authorize(actor, policy, request.id, Operation::Write, &fields, now)?;
        let key = (
            actor.principal.clone(),
            request.id.to_string(),
            request.idempotency.to_string(),
        );
        if let Some(receipt) = self.receipts.get(&key) {
            if receipt.input != request.fields
                || receipt.expected_revision != request.expected_revision
            {
                return Err("idempotency-conflict");
            }
            return Ok(receipt.result.clone());
        }
        if r.revision != request.expected_revision {
            return Err("revision-conflict");
        }
        let mut next = r.clone();
        next.fields.extend(request.fields.clone());
        validate(&next)?;
        // Explicit unique identity binding has the same Resource admission path.
        if let Some(Value::Bindings(bindings)) = next.fields.get("identities") {
            let mut seen = std::collections::BTreeSet::new();
            if bindings.iter().any(|b| !seen.insert(b)) {
                return Err("duplicate-binding");
            }
            if self.resources.values().filter(|r|r.id!=next.id && r.kind=="User").any(|r|matches!(r.fields.get("identities"),Some(Value::Bindings(b)) if b.iter().any(|p|bindings.contains(p)))){return Err("duplicate-binding");}
        }
        if next.fields != r.fields {
            next.revision += 1;
            self.events
                .push((next.id.clone(), next.revision, actor.principal.clone()));
        }
        let result = Receipt {
            revision: next.revision,
        };
        self.resources.insert(next.id.clone(), next);
        self.receipts.insert(
            key,
            StoredReceipt {
                expected_revision: request.expected_revision,
                input: request.fields,
                result: result.clone(),
            },
        );
        Ok(result)
    }
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}
pub struct Patch<'a> {
    pub id: &'a str,
    pub expected_revision: u64,
    pub idempotency: &'a str,
    pub fields: BTreeMap<String, Value>,
}
