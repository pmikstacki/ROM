//! Demo composition uses the same identity gate, Resource definitions and durable adapters.
use crate::{
    COMPLETE, DISPLAY, Dashboard, InventoryItem, NOTICE, Task,
    model::completed,
    studio_model::{CLOSE, MaintenanceTicket},
};
use rom::operator::{OperatorAccess, OperatorAuthorizer, WorkScope};
use rom::{Actor, DeliveryOutcome, PrincipalKind, Reaction, Resource, Result, Runtime, Storage};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, User};
use rom_studio_host::StudioSettings;
use std::sync::Arc;

pub fn host_actor() -> Actor {
    Actor::trusted("studio-demo-host", "configuration")
}
fn admin(actor: &Actor) -> bool {
    actor.authority == "studio-demo-host"
        || (actor.authority == "local" && actor.subject == "alice")
}
fn domain(actor: &Actor) -> bool {
    actor.authority == "studio-demo-host" || actor.authority == "local"
}
struct Operator;
impl OperatorAuthorizer for Operator {
    fn authorize(
        &self,
        actor: &Actor,
        _: OperatorAccess,
        _: Option<&WorkScope>,
        _: &mut dyn rom::AuthorizationRead,
    ) -> Result<()> {
        if admin(actor) {
            Ok(())
        } else {
            Err(rom::Error::Denied)
        }
    }
}
pub fn build(storage: Arc<dyn Storage>, clock: Arc<dyn rom::Clock>) -> Result<Runtime> {
    let host = host_actor();
    let worker = Actor::trusted("studio-demo-host", "worker").with_kind(PrincipalKind::Service);
    let gate = IdentityGate::default()
        .allow_host(&host.authority, PrincipalKind::Embedded, &host.subject)?
        .allow_host(&worker.authority, PrincipalKind::Service, &worker.subject)?;
    Runtime::builder()
        .clock(clock)
        .actor_gate(Arc::new(gate))
        .operator_authorizer(Arc::new(Operator))
        .resource(
            User::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields()
                .discovery_policy(|a, _| admin(a)),
        )
        .resource(
            IdentityProvider::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields()
                .discovery_policy(|a, _| admin(a)),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields()
                .discovery_policy(|a, _| admin(a)),
        )
        .resource(
            StudioSettings::definition()
                .policy(|a, _, _| admin(a))
                .allow_all_fields()
                .discovery_policy(|a, _| admin(a)),
        )
        .resource(
            Task::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(COMPLETE),
        )
        .resource(
            InventoryItem::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a)),
        )
        .resource(
            Dashboard::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(DISPLAY),
        )
        .resource(
            MaintenanceTicket::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(CLOSE),
        )
        .reaction(
            Reaction::new(
                "studio-completed-dashboard",
                1,
                worker.clone(),
                DISPLAY,
                completed,
            )
            .depends_on(Task::done_field()),
        )
        .channel(NOTICE, worker, |_| async { DeliveryOutcome::Accepted })
        .build(storage, Runtime::shared_cpu_pool(2)?)
}
