pub use crate::mailbox::Mailbox;
use rom::{Actor, Command, Error, PrincipalKind, Resource, Runtime};
use rom_identity::{IdentityLink, IdentityProvider, User, link_key};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    DisableUser,
    EnableUser,
    DisableLink,
    EnableLink,
    DisableProvider,
    EnableProvider,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    sequence: u8,
    action: Action,
}
impl Request {
    pub fn sequence(&self) -> u8 {
        self.sequence
    }
    pub fn parse(bytes: &[u8]) -> rom::Result<Self> {
        if bytes.len() > 4096 {
            return Err(Error::Denied);
        }
        let request: Self = serde_json::from_slice(bytes).map_err(|_| Error::Denied)?;
        if !(1..=16).contains(&request.sequence) {
            return Err(Error::Denied);
        }
        Ok(request)
    }
}
pub fn configured(actor: &Actor) -> bool {
    actor.principal_kind() == PrincipalKind::Embedded
        && actor.authority == "fixture-host"
        && actor.subject == "configuration"
}
async fn replace<R: Resource>(
    runtime: &Runtime,
    actor: &Actor,
    id: &str,
    request: &Request,
    set: impl FnOnce(&mut R),
) -> rom::Result<u64> {
    let snapshot = runtime.read::<R>(actor, id).await?;
    let mut value = snapshot.value.ok_or(Error::Denied)?;
    set(&mut value);
    runtime
        .execute(
            actor,
            Command::replace(id, value)
                .at_revision(snapshot.revision)
                .idempotency(&format!("control-{}", request.sequence)),
        )
        .await?;
    Ok(runtime.read::<R>(actor, id).await?.revision)
}
pub async fn apply(
    runtime: &Runtime,
    actor: &Actor,
    subject: &str,
    request: &Request,
) -> rom::Result<u64> {
    if !configured(actor) {
        return Err(Error::Denied);
    }
    match request.action {
        Action::DisableUser | Action::EnableUser => {
            replace::<User>(runtime, actor, "fixture-user", request, |value| {
                value.enabled = matches!(request.action, Action::EnableUser)
            })
            .await
        }
        Action::DisableLink | Action::EnableLink => {
            replace::<IdentityLink>(
                runtime,
                actor,
                &link_key("authentik", PrincipalKind::Human, subject),
                request,
                |value| value.enabled = matches!(request.action, Action::EnableLink),
            )
            .await
        }
        Action::DisableProvider | Action::EnableProvider => {
            replace::<IdentityProvider>(runtime, actor, "authentik", request, |value| {
                value.enabled = matches!(request.action, Action::EnableProvider)
            })
            .await
        }
    }
}
