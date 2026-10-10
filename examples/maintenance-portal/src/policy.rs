use rom::{Access, Actor, Definition, PrincipalKind, Resource};

pub(crate) trait Owned {
    fn owner(&self) -> &str;
    fn validate(&self) -> rom::Result<()> {
        Ok(())
    }
}

pub(crate) fn definition<R: Resource + Owned>() -> Definition<R> {
    R::definition()
        .allow_all_fields()
        .discovery_policy(|actor, _| trusted(actor))
        .policy(|actor, _: Access, resource| trusted(actor) && actor.subject == resource.owner())
        .validate_transition(|actor, before, after| {
            if !trusted(actor) {
                return Err(rom::Error::Denied);
            }
            if before.is_some_and(|resource| resource.owner() != actor.subject)
                || after.is_some_and(|resource| resource.owner() != actor.subject)
            {
                return Err(rom::Error::Denied);
            }
            if let Some(after) = after {
                after.validate()?;
            }
            Ok(())
        })
}

fn trusted(actor: &Actor) -> bool {
    actor.authority == "maintenance-portal" && actor.principal_kind() == PrincipalKind::Human
}
