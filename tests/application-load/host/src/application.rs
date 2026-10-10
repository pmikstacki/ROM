//! Isolated load Resources and explicit complete-candidate and retained-work limits.
use rom::{
    Action, Actor, Channel, Command, DeliveryOutcome, Limits, OperatorAccess, PrincipalKind,
    Reaction, ReactionLimits, Resource, Runtime, Snapshot, Storage, Target,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Clone, Resource)]
#[resource(name = "load-records")]
pub struct LoadRecord {
    pub title: String,
    pub counter: i64,
    pub open: bool,
}
pub const NOTICE: Channel<String> = Channel::new("load-notices", 1);
pub const TOUCH: Action<LoadRecord, bool> = Action::new("touch", |row, notify| {
    row.counter = row.counter.checked_add(1).ok_or(rom::Error::TooLarge)?;
    Ok(if notify {
        vec![NOTICE.intent("synthetic-load-change".into())]
    } else {
        vec![]
    })
});
fn observed(_: &Snapshot<LoadRecord>) -> rom::Result<Vec<Target<bool>>> {
    Ok(vec![])
}
fn admitted(actor: &Actor) -> bool {
    actor == &rom_demo::bootstrap_actor()
        || actor == &Actor::trusted("demo-host", "worker").with_kind(PrincipalKind::Service)
        || (actor.authority == "authentik"
            && actor.principal_kind() == PrincipalKind::Human
            && rom_identity::linked_user_id(actor).as_deref() == Some("fixture-user"))
}
pub fn runtime(storage: Arc<dyn Storage>, deliveries: Arc<AtomicUsize>) -> rom::Result<Runtime> {
    let worker = Actor::trusted("demo-host", "worker").with_kind(PrincipalKind::Service);
    crate::fixture_identity::registered(rom_demo::declarations(rom_demo::Notices::default())?)
        .resource(
            LoadRecord::definition()
                .policy(|actor, _, _| admitted(actor))
                .read_policy(admitted)
                .allow_all_fields()
                .action(TOUCH),
        )
        .reaction(Reaction::new(
            "load-observation",
            1,
            worker.clone(),
            TOUCH,
            observed,
        ))
        .channel(NOTICE, worker, move |_| {
            let deliveries = deliveries.clone();
            async move {
                deliveries.fetch_add(1, Ordering::Relaxed);
                DeliveryOutcome::Accepted
            }
        })
        .limits(Limits {
            actions: 32,
            io_jobs: 32,
            subscriptions: 64,
            snapshot_rows: 12000,
            snapshot_bytes: 32 * 1024 * 1024,
            ..Limits::default()
        })
        .reaction_limits(ReactionLimits {
            max_records: 12000,
            max_bytes: 16 * 1024 * 1024,
            ..ReactionLimits::default()
        })
        .operator_limits(rom::OperatorLimits {
            max_snapshot_records: 12000,
            max_snapshot_bytes: 16 * 1024 * 1024,
            ..rom::OperatorLimits::default()
        })
        .operator_authorizer(Arc::new(
            |actor: &Actor,
             _: OperatorAccess,
             _: Option<&rom::WorkScope>,
             _: &mut dyn rom::AuthorizationRead| {
                if admitted(actor) {
                    Ok(())
                } else {
                    Err(rom::Error::Denied)
                }
            },
        ))
        .build(storage, Runtime::shared_cpu_pool(2)?)
}
pub fn seeded(index: usize) -> Command<LoadRecord> {
    Command::create(
        &format!("load-{index:05}"),
        LoadRecord {
            title: format!("Synthetic load row {index:05}"),
            counter: 0,
            open: index.is_multiple_of(2),
        },
    )
    .idempotency(&format!("load-seed-{index:05}"))
}
