pub use crate::fixture_identity::seed_identity;
use rom::{Actor, Command, PrincipalKind, Resource, Runtime, Storage};
use rom_identity::IdentityGate;
use std::sync::Arc;
#[derive(Clone, Resource)]
#[resource(name = "recovery-records")]
pub struct Record {
    pub value: String,
}
pub fn runtime(storage: Arc<dyn Storage>) -> rom::Result<Runtime> {
    let gate = IdentityGate::default()
        .allow_host("demo-host", PrincipalKind::Embedded, "bootstrap")?
        .allow_host("demo-host", PrincipalKind::Embedded, "local-session")?
        .allow_host("demo-host", PrincipalKind::Service, "worker")?
        .allow_host("rom-blob-host", PrincipalKind::Service, "attachments")?;
    crate::fixture_identity::registered(
        rom_demo::declarations(rom_demo::Notices::default())?
            .actor_gate(Arc::new(gate))
            .resource(
                Record::definition()
                    .policy(|a, _, _| a.authority == "demo-host")
                    .allow_all_fields(),
            ),
    )
    .build(storage, Runtime::shared_cpu_pool(2)?)
}
pub fn actor() -> Actor {
    rom_demo::bootstrap_actor()
}
pub fn record(i: usize) -> Command<Record> {
    Command::create(
        &format!("record-{i:04}"),
        Record {
            value: format!("snapshot-{i:04}"),
        },
    )
    .idempotency(&format!("r8-record-{i:04}"))
}
