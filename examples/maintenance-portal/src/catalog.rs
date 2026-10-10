//! Explicit public fields use the same Resource and projection contracts.
use rom::{Access, Actor, Definition, DiscoveryTarget, PrincipalKind, Resource};
use rom_fields::DateTime;

#[derive(Clone, Resource)]
#[resource(name = "maintenance-guides")]
pub struct MaintenanceGuide {
    pub title: String,
    pub nominal_voltage: u64,
    pub updated_at: DateTime,
    pub internal_notes: String,
}

/// Host-established anonymous read context. This does not verify credentials.
pub fn public_guest() -> Actor {
    Actor::trusted("maintenance-public", "guest")
}

fn publisher(actor: &Actor) -> bool {
    actor.authority == "maintenance-portal"
        && actor.principal_kind() == PrincipalKind::Human
        && actor.subject == "alice"
}

fn reader(actor: &Actor) -> bool {
    (actor.authority == "maintenance-portal" && actor.principal_kind() == PrincipalKind::Human)
        || (actor.authority == "maintenance-public"
            && actor.principal_kind() == PrincipalKind::Embedded
            && actor.subject == "guest")
}

fn public_field(field: &str) -> bool {
    matches!(field, "title" | "nominal_voltage" | "updated_at")
}

pub(crate) fn definition() -> Definition<MaintenanceGuide> {
    MaintenanceGuide::definition()
        .policy(|actor, access, _| match access {
            Access::Read => reader(actor),
            Access::Write => publisher(actor),
        })
        .field_policy(|actor, access, field, _| match access {
            Access::Read => publisher(actor) || (reader(actor) && public_field(field)),
            Access::Write => publisher(actor),
        })
        .query_policy(|actor, field| publisher(actor) || (reader(actor) && public_field(field)))
        .sort_policy(|actor, field| publisher(actor) || (reader(actor) && public_field(field)))
        .discovery_policy(|actor, target| {
            reader(actor)
                && match target {
                    DiscoveryTarget::Resource => true,
                    DiscoveryTarget::Field(field) => publisher(actor) || public_field(field),
                    DiscoveryTarget::Action(_) => false,
                }
        })
        .validate_transition(|actor, _, after| {
            if !publisher(actor) {
                return Err(rom::Error::Denied);
            }
            if let Some(value) = after {
                if value.title.trim().is_empty() || value.title.len() > 128 {
                    return Err(rom::Error::invalid("maintenance-guides", "title"));
                }
                if !(1..=100_000).contains(&value.nominal_voltage) {
                    return Err(rom::Error::invalid("maintenance-guides", "nominal_voltage"));
                }
                if value.internal_notes.len() > 4096 {
                    return Err(rom::Error::TooLarge);
                }
            }
            Ok(())
        })
}
