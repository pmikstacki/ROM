use super::{configuration::Configuration, credentials};
use rom::{Actor, Command, PrincipalKind, Runtime, Storage};
use rom_maintenance_portal::{Equipment, Inspection, MaintenanceGuide, PortalSettings, WorkOrder};
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

pub(super) async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Configuration::read()?;
    let store: Arc<dyn Storage> = match config.database.as_str() {
        "sqlite" => Arc::new(rom_sqlite::Sqlite::open(&config.path)?),
        "redb" => Arc::new(rom_redb::Redb::open(&config.path)?),
        _ => return Err("unsupported adapter".into()),
    };
    let runtime =
        rom_maintenance_portal::declarations().build(store, Runtime::shared_cpu_pool(1)?)?;
    seed(&runtime).await?;
    let http = rom_http::Http::new(
        runtime,
        credentials::resolver(Arc::new(AtomicBool::new(true))),
        rom_http::Limits::default(),
    )?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let ready = rom::json!({"lane":"fixture-only","address":address.to_string(),"base_url":format!("http://{address}"),"database":config.database});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&config.ready)?;
    file.write_all(ready.to_string().as_bytes())?;
    file.sync_all()?;
    println!("{ready}");
    http.serve(listener, async move {
        let deadline = Instant::now() + Duration::from_secs(300);
        while Instant::now() < deadline && !config.stop.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await?;
    Ok(())
}

async fn seed(runtime: &Runtime) -> rom::Result<()> {
    let actor = Actor::trusted("maintenance-portal", "alice").with_kind(PrincipalKind::Human);
    runtime
        .execute(
            &actor,
            Command::create(
                "electrical",
                MaintenanceGuide {
                    title: "Electrical inspection".into(),
                    nominal_voltage: 230,
                    updated_at: rom_fields::DateTime::new("2026-10-08T12:00:00Z")?,
                    internal_notes: "Private publisher note".into(),
                },
            )
            .idempotency("seed-public-guide"),
        )
        .await?;
    for (id, title) in [("pump-1", "Feed pump"), ("valve-1", "Isolation valve")] {
        runtime
            .execute(
                &actor,
                Command::create(
                    id,
                    Equipment {
                        owner: "alice".into(),
                        title: title.into(),
                        active: true,
                    },
                )
                .idempotency(&format!("seed-{id}")),
            )
            .await?;
    }
    runtime
        .execute(
            &actor,
            Command::create(
                "check-1",
                Inspection {
                    owner: "alice".into(),
                    equipment: rom::ResourceRef::new("pump-1")?,
                    notes: "Valve inspected".into(),
                    inspected_at: rom_fields::DateTime::new("2026-10-08T12:00:00Z")?,
                },
            )
            .idempotency("seed-inspection"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                "repair-1",
                WorkOrder {
                    owner: "alice".into(),
                    equipment: rom::ResourceRef::new("pump-1")?,
                    title: "Replace seal".into(),
                    completed: false,
                },
            )
            .idempotency("seed-work"),
        )
        .await?;
    runtime
        .execute(
            &actor,
            Command::create(
                "workspace",
                PortalSettings {
                    owner: "alice".into(),
                    show_history: true,
                    columns: 4,
                    layout: rom_maintenance_portal::WorkspaceLayout::initial()?,
                },
            )
            .idempotency("seed-settings"),
        )
        .await?;
    Ok(())
}
