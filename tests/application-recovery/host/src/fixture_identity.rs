//! Shared synthetic identity Resources and seed records for isolated application fixtures.
use crate::configuration::Configuration;
use rom::{Command, PrincipalKind, Resource, Runtime};
use rom_identity::{IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::StudioSettings;
#[derive(Clone, Resource)]
#[resource(name = "fixture-documents")]
pub struct ProtectedDocument {
    pub content: String,
}
pub fn registered(builder: rom::Builder) -> rom::Builder {
    builder
        .resource(
            ProtectedDocument::definition()
                .policy(|a, access, _| {
                    a == &rom_demo::bootstrap_actor()
                        || (matches!(access, rom::Access::Read)
                            && a.authority == "authentik"
                            && a.principal_kind() == PrincipalKind::Human
                            && rom_identity::linked_user_id(a).as_deref() == Some("fixture-user"))
                })
                .allow_all_fields(),
        )
        .resource(
            StudioSettings::definition()
                .policy(|a, _, _| a == &rom_demo::bootstrap_actor())
                .allow_all_fields(),
        )
}
pub async fn seed_identity(runtime: &Runtime, c: &Configuration) -> rom::Result<()> {
    let actor = rom_demo::bootstrap_actor();
    runtime
        .execute(
            &actor,
            Command::create(
                "private",
                ProtectedDocument {
                    content: "Synthetic protected fixture document".into(),
                },
            )
            .idempotency("r8-protected"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                "fixture-user",
                User {
                    enabled: true,
                    display_name: "Recovery fixture".into(),
                },
            )
            .idempotency("r8-user"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                "authentik",
                IdentityProvider {
                    enabled: true,
                    profile: ProviderProfile::OidcRs256Human,
                    issuer: c.issuer.clone(),
                    audience: c.client_id.clone(),
                    endpoint: None,
                    credential_ref: None,
                },
            )
            .idempotency("r8-provider"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                &link_key(
                    "authentik",
                    PrincipalKind::Human,
                    &c.verified_synthetic_subject,
                ),
                IdentityLink {
                    authority: "authentik".into(),
                    subject: c.verified_synthetic_subject.clone(),
                    principal_kind: "human".into(),
                    user_id: "fixture-user".into(),
                    enabled: true,
                },
            )
            .idempotency("r8-link"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                "main",
                StudioSettings {
                    primary_provider: Some("authentik".into()),
                },
            )
            .idempotency("r8-settings"),
        )
        .await?;
    Ok(())
}
