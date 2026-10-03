//! Explicit, idempotent host initialization.
use crate::{Dashboard, Settings, bootstrap_actor, compensation, identity::service};
use rom::{Command, PrincipalKind, Resource, Result, Runtime};
use rom_config::{Format, ReloadTicket, SourceActivation};
use rom_identity::{IdentityLink, IdentityProvider, ProviderProfile, User, link_key};

pub(crate) async fn seed<R: Resource>(runtime: &Runtime, id: &str, value: R) -> Result<()> {
    runtime
        .execute(
            &bootstrap_actor(),
            Command::create(id, value).idempotency(&format!("seed-{}-{id}", R::KIND)),
        )
        .await?;
    Ok(())
}
/// Explicit host startup, never an HTTP endpoint. Stable seed receipts preserve user edits.
pub async fn bootstrap(runtime: &Runtime) -> Result<()> {
    compensation::bootstrap(runtime).await?;
    seed(
        runtime,
        "workshop",
        Dashboard {
            latest: "Waiting for a completed task".into(),
        },
    )
    .await?;
    seed(
        runtime,
        "alice",
        User {
            enabled: true,
            display_name: "Synthetic Alice".into(),
        },
    )
    .await?;
    seed(
        runtime,
        "synthetic",
        IdentityProvider {
            enabled: false,
            profile: ProviderProfile::JwtRs256Human,
            issuer: "https://issuer.example.invalid".into(),
            audience: "rom-demo".into(),
            endpoint: None,
            credential_ref: None,
        },
    )
    .await?;
    seed(
        runtime,
        &link_key("synthetic", PrincipalKind::Human, "alice-subject"),
        IdentityLink {
            authority: "synthetic".into(),
            subject: "alice-subject".into(),
            principal_kind: "human".into(),
            user_id: "alice".into(),
            enabled: true,
        },
    )
    .await?;
    seed(
        runtime,
        "deployment",
        SourceActivation {
            worker_authority: "demo-host".into(),
            worker_subject: "worker".into(),
            enabled: true,
            target_kind: Settings::KIND.into(),
            target_id: "workshop".into(),
            requested_generation: 0,
            source_version: "unloaded".into(),
            target_revision: None,
            target_present: false,
            valid_until: 4_102_444_800,
        },
    )
    .await?;
    // Restart reuses the accepted source state; explicit reloads need a new ticket/version.
    if runtime
        .source_state(&service(), Settings::KIND, "workshop")
        .await?
        .is_none()
    {
        ReloadTicket::request(runtime, &service(), "deployment", "demo-v1")
            .await
            .map_err(|_| rom::Error::NotCommitted)?
            .load(
                runtime,
                Format::Toml,
                include_str!("../settings.toml"),
                "bundled-demo",
            )
            .await
            .map_err(|_| rom::Error::NotCommitted)?;
    }
    Ok(())
}
