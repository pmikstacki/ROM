//! Assertions about the actual application, expressed through public ROM operations.
use super::native::Database;
use crate::{
    bootstrap_actor,
    compensation::{Checkout, Stock},
    session_actor,
};
use rom::{Actor, Command, Error, Invocation, Operation, Resource, Result, Runtime};
use std::collections::BTreeMap;

const EMPTY_STOCK: &str = "upgrade-empty-stock";
const PENDING_CHECKOUT: &str = "upgrade-pending-checkout";
pub(super) async fn seed_restrict(runtime: &Runtime) -> Result<()> {
    crate::seed(
        runtime,
        EMPTY_STOCK,
        Stock {
            total: 1,
            reservations: BTreeMap::new(),
        },
    )
    .await?;
    crate::seed(
        runtime,
        PENDING_CHECKOUT,
        Checkout {
            stock_id: rom::ResourceRef::new(EMPTY_STOCK)?,
            reservation_id: "upgrade-unused-reservation".into(),
            payment_outcome: "pending".into(),
        },
    )
    .await
}
pub(super) async fn rebuilt_restrict(runtime: &Runtime) -> Result<()> {
    let actor = session_actor();
    let stock = runtime.read::<Stock>(&actor, EMPTY_STOCK).await?;
    require(
        stock.value.ok_or(Error::Missing)?.reservations.is_empty(),
        "restrict target must have no reservations",
    )?;
    let deletion = || {
        Command::<Stock>::delete(EMPTY_STOCK)
            .at_revision(stock.revision)
            .idempotency("upgrade-delete-empty-stock")
    };
    require(
        matches!(
            runtime.execute(&actor, deletion()).await,
            Err(Error::Conflict)
        ),
        "rebuilt reference must block target deletion",
    )?;
    runtime
        .execute(
            &actor,
            Command::<Checkout>::delete(PENDING_CHECKOUT)
                .at_revision(1)
                .idempotency("upgrade-delete-pending-checkout"),
        )
        .await?;
    runtime.execute(&actor, deletion()).await?;
    crate::reference::drain(runtime).await
}
fn old_seed() -> Invocation {
    Invocation {
        retry_epoch: 0,
        kind: Checkout::KIND.into(),
        id: "checkout-a".into(),
        expected: None,
        idempotency: "seed-checkouts-checkout-a".into(),
        operation: Operation::Create(
            rom::json!({"stock_id":"workshop-stock","reservation_id":"checkout-a","payment_outcome":"pending"}),
        ),
    }
}
pub(super) async fn old_receipt_replay(runtime: &Runtime, database: &Database) -> Result<()> {
    let storage = database.storage();
    let checkout_rows = storage.snapshot(Checkout::KIND, 32, 1_000_000)?;
    let stock_rows = storage.snapshot(Stock::KIND, 32, 1_000_000)?;
    let heads = [
        storage.journal_head(Checkout::KIND)?,
        storage.journal_head(Stock::KIND)?,
    ];
    let work = storage.reaction_records()?;
    let effects = database.intentions()?;
    let replay = runtime.invoke(&bootstrap_actor(), old_seed()).await?;
    require(
        replay.revision == 1
            && replay.value
                == Some(
                    rom::json!({"stock_id":"workshop-stock","reservation_id":"checkout-a","payment_outcome":"pending"}),
                ),
        "legacy create receipt uses retained codec",
    )?;
    require(
        matches!(
            runtime
                .invoke(&Actor::trusted("demo-host", "unprovisioned"), old_seed())
                .await,
            Err(Error::Denied)
        ),
        "receipt access requires current authority",
    )?;
    require(
        storage.snapshot(Checkout::KIND, 32, 1_000_000)? == checkout_rows
            && storage.snapshot(Stock::KIND, 32, 1_000_000)? == stock_rows,
        "late replay must not mutate resources",
    )?;
    require(
        [
            storage.journal_head(Checkout::KIND)?,
            storage.journal_head(Stock::KIND)?,
        ] == heads
            && storage.reaction_records()? == work
            && database.intentions()? == effects,
        "late replay must not add events, work or effects",
    )
}
pub(super) fn require(condition: bool, reason: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::invalid("upgrade", reason))
    }
}
