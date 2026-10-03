use super::provisioning::{AUTHORITY, Provisioning};
use rom::{
    Actor, ActorGate, AuthorizationRead, Clock, Error, PrincipalKind, Resource, Result, Runtime,
    Storage,
};
use rom_identity::{IdentityGate, IdentityLink, IdentityProvider, User, linked_user_id};
use std::sync::Arc;
/// Provisioning permits the explicit local bootstrap identity; serving does not.
#[derive(Clone, Copy)]
pub enum LocalMode {
    Provisioning,
    Serving,
}
pub fn declarations(settings: &Provisioning, mode: LocalMode) -> Result<rom::Builder> {
    settings.validate()?;
    let mut gate = IdentityGate::default()
        .allow_host(AUTHORITY, PrincipalKind::Embedded, "configuration-reader")?
        .allow_host(AUTHORITY, PrincipalKind::Embedded, "maintainer")?;
    if matches!(mode, LocalMode::Provisioning) {
        gate = gate.allow_host(AUTHORITY, PrincipalKind::Embedded, "provisioner")?;
    }
    Ok(Runtime::builder()
        .actor_gate(Arc::new(ProfileGate {
            inner: gate,
            provider: settings.provider_id.clone(),
            user: settings.user_id.clone(),
            subject: settings.service_subject.clone(),
        }))
        .resource(
            IdentityProvider::definition()
                .policy(|a, access, _| {
                    local_writer(a)
                        || (local_role(a, "configuration-reader")
                            && matches!(access, rom::Access::Read))
                })
                .allow_all_fields(),
        )
        .resource(
            User::definition()
                .policy(|a, _, _| local_writer(a))
                .allow_all_fields(),
        )
        .resource(
            IdentityLink::definition()
                .policy(|a, _, _| local_writer(a))
                .allow_all_fields(),
        )
        .resource(
            crate::Task::definition()
                .discovery_policy(|a, target| {
                    service(a)
                        && match target {
                            rom::DiscoveryTarget::Resource => true,
                            rom::DiscoveryTarget::Field(name) => matches!(name, "title" | "done"),
                            rom::DiscoveryTarget::Action(name) => name == "complete",
                        }
                })
                .policy(|a, _, _| service(a))
                .allow_all_fields()
                .action(crate::COMPLETE),
        ))
}
fn service(actor: &Actor) -> bool {
    actor.principal_kind() == PrincipalKind::Service && actor.host_stamp().is_some()
}
fn local_role(actor: &Actor, subject: &str) -> bool {
    actor.authority == AUTHORITY
        && actor.subject == subject
        && actor.principal_kind() == PrincipalKind::Embedded
        && actor.host_stamp().is_none()
}
fn local_writer(actor: &Actor) -> bool {
    local_role(actor, "provisioner") || local_role(actor, "maintainer")
}
struct ProfileGate {
    inner: IdentityGate,
    provider: String,
    user: String,
    subject: String,
}
impl ActorGate for ProfileGate {
    fn check(&self, actor: &Actor, storage: &mut dyn AuthorizationRead) -> Result<()> {
        self.inner.check(actor, storage)?;
        if actor.host_stamp().is_some()
            && (actor.principal_kind() != PrincipalKind::Service
                || actor.authority != self.provider
                || actor.subject != self.subject
                || linked_user_id(actor).as_deref() != Some(self.user.as_str()))
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
}
pub fn build(
    storage: Arc<dyn Storage>,
    clock: Arc<dyn Clock>,
    settings: &Provisioning,
    mode: LocalMode,
) -> Result<Runtime> {
    declarations(settings, mode)?
        .clock(clock)
        .build(storage, Runtime::shared_cpu_pool(2)?)
}
