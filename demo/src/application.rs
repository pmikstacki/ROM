//! Runtime composition and notification delivery.
use crate::identity::{
    admin, domain, internal, same_principal, service, session_actor, source_fields,
};
use crate::{
    compensation,
    model::{COMPLETE, DISPLAY, Dashboard, InventoryItem, NOTICE, Settings, Task, completed},
};
use rom::{Access, DeliveryOutcome, PrincipalKind, Reaction, Resource, Result, Runtime, Storage};
use rom_config::{REQUEST_RELOAD, SourceActivation};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, User};
use std::sync::{Arc, Mutex};

/// Notification receiver fixture. The stable delivery ID deduplicates the in-process sink.
/// This sink deliberately makes no durable/external exactly-once claim.
pub type Notices = Arc<Mutex<Vec<(String, String)>>>;
pub fn declarations(notices: Notices) -> Result<rom::Builder> {
    let gate = IdentityGate::default()
        .allow_host("demo-host", PrincipalKind::Embedded, "bootstrap")?
        .allow_host("demo-host", PrincipalKind::Embedded, "local-session")?
        .allow_host("demo-host", PrincipalKind::Service, "worker")?
        .allow_host("rom-blob-host", PrincipalKind::Service, "attachments")?;
    let builder = Runtime::builder()
        .actor_gate(Arc::new(gate))
        .resource(rom_blob::definition())
        .resource(
            Task::definition()
                .discovery_policy(|a, target| {
                    domain(a)
                        && match target {
                            rom::DiscoveryTarget::Resource => true,
                            rom::DiscoveryTarget::Field(name) => {
                                matches!(name, "title" | "done")
                            }
                            rom::DiscoveryTarget::Action(name) => name == "complete",
                        }
                })
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .action(COMPLETE),
        )
        .resource(
            InventoryItem::definition()
                .discovery_policy(|a, target| {
                    domain(a)
                        && match target {
                            rom::DiscoveryTarget::Resource => true,
                            rom::DiscoveryTarget::Field(name) => {
                                matches!(name, "code" | "quantity")
                            }
                            rom::DiscoveryTarget::Action(_) => false,
                        }
                })
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
        });
    Ok(compensation::declarations(builder))
}
pub fn build(storage: Arc<dyn Storage>, notices: Notices) -> Result<Runtime> {
    declarations(notices)?.build(storage, Runtime::shared_cpu_pool(2)?)
}
