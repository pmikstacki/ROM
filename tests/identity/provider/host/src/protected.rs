use rom::{Access, Actor, Definition, PrincipalKind, Resource};

#[derive(Clone, Resource)]
#[resource(name = "fixture-documents")]
pub struct ProtectedDocument {
    pub content: String,
}

pub fn definition() -> Definition<ProtectedDocument> {
    ProtectedDocument::definition()
        .policy(|actor: &Actor, access, _| {
            crate::control::configured(actor)
                || (matches!(access, Access::Read)
                    && actor.principal_kind() == PrincipalKind::Human
                    && actor.authority == "authentik"
                    && rom_identity::linked_user_id(actor).as_deref() == Some("fixture-user"))
        })
        .allow_all_fields()
}
