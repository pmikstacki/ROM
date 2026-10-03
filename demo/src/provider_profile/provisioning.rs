use rom::{
    Actor, Command, Error, Invocation, Operation, PrincipalKind, Resource, Result, Row, Runtime,
};
use rom_identity::{IdentityLink, IdentityProvider, User, link_key};

/// Trusted local setup values; this type is not a remote identity proof.
#[derive(Clone)]
pub struct Provisioning {
    pub provider_id: String,
    pub provider: IdentityProvider,
    pub user_id: String,
    pub user: User,
    pub service_subject: String,
}
impl Provisioning {
    pub(super) fn validate(&self) -> Result<()> {
        for value in [&self.provider_id, &self.user_id, &self.service_subject] {
            bounded(value, 2048)?;
        }
        Ok(())
    }
    pub(super) fn link_id(&self) -> String {
        link_key(
            &self.provider_id,
            PrincipalKind::Service,
            &self.service_subject,
        )
    }
}
pub(super) fn bounded(value: &str, limit: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > limit
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        Err(Error::Denied)
    } else {
        Ok(())
    }
}
pub(super) const AUTHORITY: &str = "provider-profile-host";
pub(super) fn local(subject: &str) -> Actor {
    Actor::trusted(AUTHORITY, subject)
}
/// Trusted host configuration reader, granted only provider Resource reads.
pub fn configuration_reader() -> Actor {
    local("configuration-reader")
}
/// Apply three separate, durably idempotent Resource creates. A restart resumes
/// the same accepted inputs; this is not a transaction across all three Resources.
pub async fn provision(runtime: &Runtime, settings: &Provisioning) -> Result<()> {
    settings.validate()?;
    let actor = local("provisioner");
    let identity =
        |step| rom::json!(["provider-provision-v1", settings.provider_id, step]).to_string();
    runtime
        .execute(
            &actor,
            Command::create(&settings.provider_id, settings.provider.clone())
                .idempotency(&identity("provider")),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(&settings.user_id, settings.user.clone())
                .idempotency(&identity("user")),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                &settings.link_id(),
                IdentityLink {
                    authority: settings.provider_id.clone(),
                    subject: settings.service_subject.clone(),
                    principal_kind: "service".into(),
                    user_id: settings.user_id.clone(),
                    enabled: true,
                },
            )
            .idempotency(&identity("link")),
        )
        .await?;
    Ok(())
}
/// Offline host authority only. The caller must own the native database.
pub async fn maintain(
    runtime: &Runtime,
    settings: &Provisioning,
    invocation: Invocation,
) -> Result<Row> {
    settings.validate()?;
    let allowed = match invocation.kind.as_str() {
        IdentityProvider::KIND => invocation.id == settings.provider_id,
        User::KIND => invocation.id == settings.user_id,
        IdentityLink::KIND => invocation.id == settings.link_id(),
        _ => false,
    };
    if !allowed
        || invocation.expected.is_none_or(|v| v == 0)
        || invocation.idempotency.is_empty()
        || invocation.idempotency.chars().any(char::is_control)
        || !matches!(
            invocation.operation,
            Operation::Replace(_) | Operation::Patch(_) | Operation::Delete
        )
    {
        return Err(Error::Denied);
    }
    runtime.invoke(&local("maintainer"), invocation).await
}
