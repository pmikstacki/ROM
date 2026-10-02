//! A complete local application built from Resource declarations and business functions.
use rom::{
    Access, Action, Actor, Channel, Command, DeliveryOutcome, Field, Patch, PrincipalKind,
    Reaction, Resource, Result, Runtime, Shape, Snapshot, Storage, Target, Value,
};
use rom_config::{Format, REQUEST_RELOAD, ReloadTicket, SourceActivation};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use std::sync::{Arc, Mutex};

/// Canonical stock code: uppercase ASCII letters, digits, and hyphens, at most 24 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct StockCode(String);
impl Field for StockCode {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        self.0.clone().into()
    }
    fn decode(value: Value) -> Result<Self> {
        let value = value
            .as_str()
            .ok_or_else(|| rom::Error::invalid("code", "string required"))?;
        let value = value.trim().to_ascii_uppercase();
        if value.is_empty()
            || value.len() > 24
            || !value
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err(rom::Error::invalid("code", "invalid stock code"));
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "tasks")]
pub struct Task {
    pub title: String,
    pub done: bool,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "inventory")]
pub struct InventoryItem {
    pub code: StockCode,
    pub quantity: u64,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "dashboards")]
pub struct Dashboard {
    pub latest: String,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "settings")]
pub struct Settings {
    pub workshop: String,
    pub enabled: bool,
}

pub const NOTICE: Channel<String> = Channel::new("local-completions", 1);
pub const COMPLETE: Action<Task, ()> = Action::new("complete", |task, ()| {
    task.done = true;
    Ok(vec![])
});
pub const DISPLAY: Action<Dashboard, String> = Action::new("display", |dashboard, title| {
    dashboard.latest = title.clone();
    Ok(vec![NOTICE.intent(title)])
});
fn completed(task: &Snapshot<Task>) -> Result<Vec<Target<String>>> {
    Ok(task
        .value
        .as_ref()
        .filter(|task| task.done)
        .map(|task| vec![Target::new("workshop", task.title.clone())])
        .unwrap_or_default())
}
pub fn bootstrap_actor() -> Actor {
    Actor::trusted("demo-host", "bootstrap")
}
pub fn session_actor() -> Actor {
    Actor::trusted("demo-host", "local-session")
}
fn service() -> Actor {
    Actor::trusted("demo-host", "worker").with_kind(PrincipalKind::Service)
}
fn same_principal(actor: &Actor, expected: &Actor) -> bool {
    actor.authority == expected.authority
        && actor.subject == expected.subject
        && actor.principal_kind() == expected.principal_kind()
}
fn admin(actor: &Actor) -> bool {
    same_principal(actor, &bootstrap_actor())
}
fn internal(actor: &Actor) -> bool {
    admin(actor) || same_principal(actor, &service())
}
fn domain(actor: &Actor) -> bool {
    internal(actor) || same_principal(actor, &session_actor())
}
fn source_fields(actor: &Actor, access: Access, name: &str, _: &SourceActivation) -> bool {
    admin(actor)
        || (same_principal(actor, &service())
            && (matches!(access, Access::Read)
                || matches!(
                    name,
                    "requested_generation"
                        | "source_version"
                        | "target_revision"
                        | "target_present"
                )))
}
/// Notification receiver fixture. The stable delivery ID deduplicates the in-process sink.
/// This sink deliberately makes no durable/external exactly-once claim.
pub type Notices = Arc<Mutex<Vec<(String, String)>>>;
pub fn declarations(notices: Notices) -> Result<rom::Builder> {
    let gate = IdentityGate::default()
        .allow_host("demo-host", PrincipalKind::Embedded, "bootstrap")?
        .allow_host("demo-host", PrincipalKind::Embedded, "local-session")?
        .allow_host("demo-host", PrincipalKind::Service, "worker")?
        .allow_host("rom-blob-host", PrincipalKind::Service, "attachments")?;
    Ok(Runtime::builder()
        .actor_gate(Arc::new(gate))
        .resource(rom_blob::definition())
        .resource(
            Task::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .action(COMPLETE),
        )
        .resource(
            InventoryItem::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields(),
        )
        .resource(
            Dashboard::definition()
                .policy(|a, access, _| {
                    internal(a)
                        || (same_principal(a, &session_actor()) && matches!(access, Access::Read))
                })
                .allow_all_fields()
                .action(DISPLAY),
        )
        .resource(
            Settings::definition()
                .policy(|a, access, _| {
                    internal(a)
                        || (same_principal(a, &session_actor()) && matches!(access, Access::Read))
                })
                .allow_all_fields()
                .source_owner("deployment")
                .source_metadata_policy(internal),
        )
        .resource(
            SourceActivation::definition()
                .policy(|a, _, _| internal(a))
                .field_policy(source_fields)
                .action(REQUEST_RELOAD),
        )
        .resource(
            User::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields(),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields(),
        )
        .reaction(
            Reaction::new("completed-task-dashboard", 1, service(), DISPLAY, completed)
                .depends_on(Task::done_field()),
        )
        .channel(NOTICE, service(), move |delivery| {
            let notices = notices.clone();
            async move {
                let Ok(mut notices) = notices.lock() else {
                    return DeliveryOutcome::Permanent;
                };
                if !notices.iter().any(|(id, _)| id == &delivery.id) {
                    notices.push((delivery.id, delivery.payload));
                }
                DeliveryOutcome::Accepted
            }
        }))
}
pub fn build(storage: Arc<dyn Storage>, notices: Notices) -> Result<Runtime> {
    declarations(notices)?.build(storage, Runtime::shared_cpu_pool(2)?)
}
async fn seed<R: Resource>(runtime: &Runtime, id: &str, value: R) -> Result<()> {
    runtime
        .execute(
            &bootstrap_actor(),
            Command::create(id, value).idempotency(&format!("seed-{}-{id}", R::KIND)),
        )
        .await?;
    Ok(())
}
/// Explicit host startup, never an HTTP endpoint. Stable seed receipts preserve user edits.
pub async fn bootstrap(runtime: &Runtime) -> Result<()> {
    seed(
        runtime,
        "workshop",
        Dashboard {
            latest: "Waiting for a completed task".into(),
        },
    )
    .await?;
    seed(
        runtime,
        "alice",
        User {
            enabled: true,
            display_name: "Synthetic Alice".into(),
        },
    )
    .await?;
    seed(
        runtime,
        "synthetic",
        IdentityProvider {
            enabled: false,
            profile: ProviderProfile::JwtRs256Human,
            issuer: "https://issuer.example.invalid".into(),
            audience: "rom-demo".into(),
            endpoint: None,
            credential_ref: None,
        },
    )
    .await?;
    seed(
        runtime,
        &link_key("synthetic", PrincipalKind::Human, "alice-subject"),
        IdentityLink {
            authority: "synthetic".into(),
            subject: "alice-subject".into(),
            principal_kind: "human".into(),
            user_id: "alice".into(),
            enabled: true,
        },
    )
    .await?;
    seed(
        runtime,
        "deployment",
        SourceActivation {
            worker_authority: "demo-host".into(),
            worker_subject: "worker".into(),
            enabled: true,
            target_kind: Settings::KIND.into(),
            target_id: "workshop".into(),
            requested_generation: 0,
            source_version: "unloaded".into(),
            target_revision: None,
            target_present: false,
            valid_until: 4_102_444_800,
        },
    )
    .await?;
    // Restart reuses the accepted source state; explicit reloads need a new ticket/version.
    if runtime
        .source_state(&service(), Settings::KIND, "workshop")
        .await?
        .is_none()
    {
        ReloadTicket::request(runtime, &service(), "deployment", "demo-v1")
            .await
            .map_err(|_| rom::Error::NotCommitted)?
            .load(
                runtime,
                Format::Toml,
                include_str!("../settings.toml"),
                "bundled-demo",
            )
            .await
            .map_err(|_| rom::Error::NotCommitted)?;
    }
    Ok(())
}
/// Typed patch uses the generated selector; no string field names or per-kind mutation route.
pub fn rename_task(id: &str, revision: u64, title: String) -> Command<Task> {
    Command::patch(id, Patch::new().set(Task::title_field(), title))
        .at_revision(revision)
        .idempotency("demo-rename")
}
/// Deliberately public synthetic session marker, restricted to numeric-loopback demo hosting.
/// It never resolves bootstrap authority, and is not production authentication.
pub fn resolver() -> rom_http::AuthResolver {
    Arc::new(
        |headers| match headers.get("authorization").and_then(|h| h.to_str().ok()) {
            Some("Demo local") => Ok(session_actor()),
            _ => Err(rom::Error::Denied),
        },
    )
}
/// Finite, actual TCP smoke scenario shared by the command and integration test.
pub mod smoke;

/// Host-owned folder adapter and attachment lifecycle.
pub mod attachments;
