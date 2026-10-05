//! Provision absent demo rows without replaying seed receipts over edits or tombstones.
use crate::{
    Dashboard, InventoryItem, StockCode, Task,
    studio_application::host_actor,
    studio_model::{MaintenanceTicket, OpaqueHandle, TicketCode},
};
use rom::{
    Command, Field, Key, PrincipalKind, Resource, ResourceRef, Result, Row, Runtime, Storage,
};
use rom_identity::{IdentityLink, IdentityProvider, ProviderProfile, User, link_key};
use rom_studio_host::StudioSettings;
const TASK_SEEDS: [(&str, &str, bool); 3] = [
    ("task-a", "Inspect pump", false),
    ("task-b", "Review report", false),
    ("task-c", "Archived check", true),
];
fn load<R: Resource>(storage: &dyn Storage, id: &str) -> Result<Option<Row>> {
    storage.load(&Key {
        kind: R::KIND.into(),
        id: id.into(),
    })
}
async fn seed<R: Resource>(
    runtime: &Runtime,
    storage: &dyn Storage,
    id: &str,
    value: R,
) -> Result<()> {
    // Host provisioning inspects existence only; ordinary commands still enforce mutation authority.
    if load::<R>(storage, id)?.is_some() {
        return Ok(());
    }
    runtime
        .execute(
            &host_actor(),
            Command::create(id, value).idempotency(&format!("studio-seed-{}-{id}", R::KIND)),
        )
        .await?;
    Ok(())
}
pub async fn seed_all(runtime: &Runtime, storage: &dyn Storage, issuer: &str) -> Result<()> {
    // Existing rows must not turn a closed or unauthorized runtime into startup success.
    runtime.establish_actor(|_| Ok(host_actor())).await?;
    for account in ["alice", "bob"] {
        let user = format!("{account}-user");
        seed(
            runtime,
            storage,
            &user,
            User {
                enabled: true,
                display_name: format!("Demo {account}"),
            },
        )
        .await?;
        seed(
            runtime,
            storage,
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
        storage,
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
        storage,
        "default",
        StudioSettings {
            primary_provider: None,
        },
    )
    .await?;
    for (id, title, done) in TASK_SEEDS {
        seed(
            runtime,
            storage,
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
        storage,
        "inventory-a",
        InventoryItem {
            code: StockCode::decode("PUMP-01".into())?,
            quantity: 0,
        },
    )
    .await?;
    seed(
        runtime,
        storage,
        "workshop",
        Dashboard {
            latest: "Waiting for a completed task".into(),
        },
    )
    .await?;
    seed(
        runtime,
        storage,
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
    seed_showcase(runtime, storage).await?;
    Ok(())
}

pub(super) async fn seed_showcase(runtime: &Runtime, storage: &dyn Storage) -> Result<()> {
    use crate::studio_semantic::FieldShowcase;
    let id = "workshop-sample";
    if load::<FieldShowcase>(storage, id)?.is_some() {
        return Ok(());
    }
    for (task, _, _) in TASK_SEEDS {
        if load::<Task>(storage, task)?.is_some_and(|row| row.value.is_some()) {
            let mut example = crate::studio_semantic::example()?;
            example.task = ResourceRef::new(task)?;
            return seed(runtime, storage, id, example).await;
        }
    }
    // This optional synthetic fixture cannot refer to a deleted prerequisite.
    Ok(())
}
