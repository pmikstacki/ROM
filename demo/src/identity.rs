//! Trusted demo principals and resource access rules.
use rom::{Access, Actor, PrincipalKind};
use rom_config::SourceActivation;
use std::sync::Arc;

pub fn bootstrap_actor() -> Actor {
    Actor::trusted("demo-host", "bootstrap")
}
pub fn session_actor() -> Actor {
    Actor::trusted("demo-host", "local-session")
}
pub(crate) fn service() -> Actor {
    Actor::trusted("demo-host", "worker").with_kind(PrincipalKind::Service)
}
pub(crate) fn same_principal(actor: &Actor, expected: &Actor) -> bool {
    actor.authority == expected.authority
        && actor.subject == expected.subject
        && actor.principal_kind() == expected.principal_kind()
}
pub(crate) fn admin(actor: &Actor) -> bool {
    same_principal(actor, &bootstrap_actor())
}
pub(crate) fn internal(actor: &Actor) -> bool {
    admin(actor) || same_principal(actor, &service())
}
pub(crate) fn domain(actor: &Actor) -> bool {
    internal(actor) || same_principal(actor, &session_actor())
}
pub(crate) fn source_fields(
    actor: &Actor,
    access: Access,
    name: &str,
    _: &SourceActivation,
) -> bool {
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
