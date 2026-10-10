//! Common current-owner checks; each domain retains its own transition invariants.
use rom::{Actor, Definition, Resource};
pub(crate) trait Owned: Resource {
    fn owner(&self) -> &str;
    fn validate(&self) -> rom::Result<()>;
    fn transition(before: Option<&Self>, after: Option<&Self>) -> rom::Result<()>;
}
pub(crate) fn definition<R: Owned>() -> Definition<R> {
    R::definition()
        .discovery_policy(|actor, _| trusted(actor))
        .policy(|actor, _, value| trusted(actor) && actor.subject == value.owner())
        .field_policy(|actor, _, _, value| trusted(actor) && actor.subject == value.owner())
        .validate_transition(|actor, before, after| {
            if !trusted(actor)
                || before.is_some_and(|value| value.owner() != actor.subject)
                || after.is_some_and(|value| value.owner() != actor.subject)
            {
                return Err(rom::Error::Denied);
            }
            if let Some(after) = after {
                after.validate()?;
            }
            R::transition(before, after)
        })
}
fn trusted(actor: &Actor) -> bool {
    actor.authority == "external-ai-consumer" && actor.subject == "author"
}
