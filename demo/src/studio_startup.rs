//! Stable seed receipts preserve edits when the same demo store is reopened.
use crate::{
    Dashboard, InventoryItem, StockCode, Task,
    studio_application::host_actor,
    studio_model::{MaintenanceTicket, OpaqueHandle, TicketCode},
};
use rom::{Command, Field, PrincipalKind, Resource, Result, Runtime};
use rom_identity::{IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::StudioSettings;
async fn seed<R: Resource>(runtime: &Runtime, id: &str, value: R) -> Result<()> {
    runtime
        .execute(
            &host_actor(),
            Command::create(id, value).idempotency(&format!("studio-seed-{}-{id}", R::KIND)),
        )
        .await?;
    Ok(())
}
pub async fn seed_all(runtime: &Runtime, issuer: &str) -> Result<()> {
    for account in ["alice", "bob"] {
        let user = format!("{account}-user");
        seed(
            runtime,
            &user,
            User {
                enabled: true,
                display_name: format!("Demo {account}"),
            },
        )
        .await?;
        seed(
            runtime,
            &link_key("local", PrincipalKind::Human, account),
            IdentityLink {
                authority: "local".into(),
                subject: account.into(),
                principal_kind: "human".into(),
                user_id: user,
                enabled: true,
            },
        )
        .await?;
    }
    seed(
        runtime,
        "local",
        IdentityProvider {
            enabled: true,
            profile: ProviderProfile::OidcRs256Human,
            issuer: issuer.into(),
            audience: "studio".into(),
            endpoint: None,
            credential_ref: None,
        },
    )
    .await?;
    seed(
        runtime,
        "default",
        StudioSettings {
            primary_provider: None,
        },
    )
    .await?;
    for (id, title, done) in [
        ("task-a", "Inspect pump", false),
        ("task-b", "Review report", false),
        ("task-c", "Archived check", true),
    ] {
        seed(
            runtime,
            id,
            Task {
                title: title.into(),
                done,
            },
        )
        .await?;
    }
    seed(
        runtime,
        "inventory-a",
        InventoryItem {
            code: StockCode::decode("PUMP-01".into())?,
            quantity: 0,
        },
    )
    .await?;
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
        "ticket-a",
        MaintenanceTicket {
            code: TicketCode::decode("TICKET-A1".into())?,
            optional_code: Some(TicketCode::decode("TICKET-O1".into())?),
            code_list: vec![TicketCode::decode("TICKET-L1".into())?],
            summary: "Held-out custom codec example".into(),
            open: true,
            attempts: 18_446_744_073_709_551_615,
            opaque: Some(OpaqueHandle::decode("retained-handle".into())?),
            required_handle: OpaqueHandle::decode("required-handle".into())?,
        },
    )
    .await?;
    Ok(())
}
