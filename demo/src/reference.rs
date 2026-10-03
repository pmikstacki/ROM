//! Reproducible application journey using only public ROM APIs.
//! The executable closes/reopens cleanly; subprocess tests additionally skip shutdown.
use crate::{
    COMPLETE, Dashboard, InventoryItem, Notices, StockCode, Task, bootstrap, build,
    compensation::{Checkout, RECORD_PAYMENT, RESERVE, Stock},
    seed, session_actor,
};
use rom::{Actor, Command, Direction, Error, Field, Resource, Result, Runtime, Storage};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};

fn require(condition: bool, reason: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::invalid("reference", reason))
    }
}
fn rejection() -> Command<Checkout> {
    Command::action("checkout-a", RECORD_PAYMENT, "confirmed_rejected".into())
        .at_revision(2)
        .idempotency("reference-rejection")
}
async fn drain(runtime: &Runtime) -> Result<()> {
    for _ in 0..16 {
        if runtime.process_work(32).await? == 0 {
            return Ok(());
        }
    }
    Err(Error::invalid("reference", "work drain exhausted"))
}
/// Commit a confirmed business rejection but leave its reaction pending.
pub async fn prepare(runtime: &Runtime) -> Result<()> {
    bootstrap(runtime).await?;
    let actor = session_actor();
    for (token, quantity, revision) in [("checkout-a", 3, 1), ("checkout-b", 2, 2)] {
        runtime
            .execute(
                &actor,
                Command::action(
                    "workshop-stock",
                    RESERVE,
                    crate::compensation::ReserveInput {
                        token: token.into(),
                        quantity,
                    },
                )
                .at_revision(revision)
                .idempotency(&format!("reference-reserve-{token}")),
            )
            .await?;
    }
    runtime
        .execute(
            &actor,
            Command::action("checkout-a", RECORD_PAYMENT, "unknown".into())
                .at_revision(1)
                .idempotency("reference-unknown"),
        )
        .await?;
    drain(runtime).await?;
    let held = runtime.read::<Stock>(&actor, "workshop-stock").await?;
    require(
        held.value.ok_or(Error::Missing)?.reservations
            == BTreeMap::from([("checkout-a".into(), 3), ("checkout-b".into(), 2)]),
        "unknown outcome must preserve reservations",
    )?;
    for (id, quantity) in [("low", 2), ("high", 9)] {
        seed(
            runtime,
            id,
            InventoryItem {
                code: StockCode::decode(rom::json!(id))?,
                quantity,
            },
        )
        .await?;
    }
    seed(
        runtime,
        "recovery-task",
        Task {
            title: "Reconcile checkout".into(),
            done: false,
        },
    )
    .await?;
    drain(runtime).await?;
    runtime.execute(&actor, rejection()).await?;
    // No work processing after this point: the next process must recover it.
    Ok(())
}
/// Recover prepared work and exercise queries, live membership and duplicate replay.
pub async fn recover(runtime: &Runtime) -> Result<()> {
    let actor = session_actor();
    let before = runtime.read::<Stock>(&actor, "workshop-stock").await?;
    require(before.revision == 3, "prepared stock revision")?;
    require(
        before
            .value
            .as_ref()
            .ok_or(Error::Missing)?
            .reservations
            .len()
            == 2,
        "reaction must still be pending",
    )?;
    let checkout = runtime.read::<Checkout>(&actor, "checkout-a").await?;
    require(
        checkout.revision == 3
            && checkout.value.ok_or(Error::Missing)?.payment_outcome == "confirmed_rejected",
        "confirmed outcome survives restart",
    )?;
    // The application's typed query composes exact filtering, ordering and continuation.
    let query = InventoryItem::quantity_field()
        .at_least(2)
        .order_by(InventoryItem::quantity_field(), Direction::Desc)
        .limit(1);
    let first = runtime.query(&actor, &query).await?;
    require(
        first.len() == 1 && first[0].id == "high",
        "ordered first page",
    )?;
    let next = runtime
        .query(&actor, &query.after_snapshot(&first[0])?)
        .await?;
    require(next.len() == 1 && next[0].id == "low", "moving next page")?;
    require(
        matches!(
            runtime
                .query(
                    &Actor::trusted("demo-host", "unprovisioned"),
                    &InventoryItem::quantity_field().at_least(0),
                )
                .await,
            Err(Error::Denied)
        ),
        "restart must not bypass current authority",
    )?;

    let mut stock_live = runtime
        .live(
            &actor,
            Stock::reservations_field().equals(BTreeMap::from([("checkout-b".into(), 2)])),
        )
        .await?;
    require(
        stock_live.changed().await?.is_empty(),
        "initial filtered stock",
    )?;
    drain(runtime).await?;
    let visible = tokio::time::timeout(Duration::from_secs(5), stock_live.changed())
        .await
        .map_err(|_| Error::invalid("reference", "stock live timeout"))??;
    require(
        visible.len() == 1 && visible[0].id == "workshop-stock" && visible[0].revision == 4,
        "recovered compensation must update live membership once",
    )?;
    let stock_events = runtime
        .journal(&actor, Stock::KIND, None)
        .await?
        .events
        .len();
    let checkout_events = runtime
        .journal(&actor, Checkout::KIND, None)
        .await?
        .events
        .len();
    require(
        stock_events == 4 && checkout_events == 3,
        "recovery event counts",
    )?;
    runtime.execute(&actor, rejection()).await?;
    require(
        runtime.process_work(32).await? == 0,
        "replay must not enqueue work",
    )?;
    require(
        runtime
            .journal(&actor, Stock::KIND, None)
            .await?
            .events
            .len()
            == stock_events
            && runtime
                .journal(&actor, Checkout::KIND, None)
                .await?
                .events
                .len()
                == checkout_events,
        "replay must not add events",
    )?;

    let mut open_tasks = runtime
        .live(&actor, Task::done_field().equals(false))
        .await?;
    require(open_tasks.changed().await?.len() == 1, "open task")?;
    runtime
        .execute(
            &actor,
            Command::action("recovery-task", COMPLETE, ())
                .at_revision(1)
                .idempotency("reference-complete"),
        )
        .await?;
    let remaining = tokio::time::timeout(Duration::from_secs(5), open_tasks.changed())
        .await
        .map_err(|_| Error::invalid("reference", "task live timeout"))??;
    require(remaining.is_empty(), "completed task leaves live query")?;
    drain(runtime).await?;
    require(
        runtime
            .read::<Dashboard>(&actor, "workshop")
            .await?
            .value
            .ok_or(Error::Missing)?
            .latest
            == "Reconcile checkout",
        "task reaction reaches another Resource",
    )?;
    Ok(())
}
fn open(redb: bool, path: &Path) -> Result<Runtime> {
    let storage: Arc<dyn Storage> = if redb {
        Arc::new(rom_redb::Redb::open(path)?)
    } else {
        Arc::new(rom_sqlite::Sqlite::open(path)?)
    };
    build(storage, Notices::default())
}
struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
/// Run on a fresh private scratch database, close and reopen with the same declarations.
pub async fn run(redb: bool) -> crate::smoke::SmokeResult<()> {
    let path = std::env::temp_dir().join(format!(
        "rom-reference-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&path)?;
    let scratch = Scratch(path);
    let database = scratch.0.join("db");
    let objects = scratch.0.join("objects");
    let runtime = open(redb, &database)?;
    let blobs = crate::attachments::open(runtime.clone(), &objects)?;
    crate::attachments::attach(&blobs).await?;
    prepare(&runtime).await?;
    blobs.shutdown().await?;
    drop(blobs);
    runtime.shutdown().await?;
    drop(runtime);
    let runtime = open(redb, &database)?;
    let blobs = crate::attachments::open(runtime.clone(), &objects)?;
    require(
        blobs.read(&session_actor(), crate::attachments::ID).await? == crate::attachments::CONTENT,
        "attachment survives application restart",
    )?;
    recover(&runtime).await?;
    blobs.shutdown().await?;
    runtime.shutdown().await?;
    Ok(())
}
