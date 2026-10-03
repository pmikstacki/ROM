//! Explicit business compensation after a confirmed, simulated payment rejection.
//! Unknown outcomes require reconciliation; terminal worker errors are not inferred here.
use crate::{domain, seed, service};
use rom::{
    Action, Actor, Builder, Definition, Error, FieldRef, Reaction, Resource, ResourceRef, Result,
    Runtime,
};
pub(crate) mod checkout_rules;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Resource)]
#[resource(name = "reservation-stock")]
pub struct Stock {
    pub total: u64,
    pub reservations: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "checkouts", version = 2)]
pub struct Checkout {
    pub stock_id: ResourceRef<Stock>,
    pub reservation_id: String,
    pub payment_outcome: String,
}
// Invariants belong to the Resource, so generic patch/replace cannot bypass them.
fn valid_stock(_: &Actor, before: Option<&Stock>, after: Option<&Stock>) -> Result<()> {
    let Some(stock) = after else {
        return if before.is_some_and(|stock| !stock.reservations.is_empty()) {
            Err(Error::invalid(Stock::KIND, "outstanding reservations"))
        } else {
            Ok(())
        };
    };
    let used = stock
        .reservations
        .iter()
        .try_fold(0u64, |sum, (token, quantity)| {
            if token.is_empty() || *quantity == 0 {
                return Err(Error::invalid(Stock::KIND, "reservation"));
            }
            sum.checked_add(*quantity)
                .ok_or_else(|| Error::invalid(Stock::KIND, "quantity overflow"))
        })?;
    if used > stock.total {
        return Err(Error::invalid(Stock::KIND, "insufficient stock"));
    }
    Ok(())
}
/// Typed command arguments, not another domain entity or independent schema.
#[derive(Clone, Debug, rom::Input)]
pub struct ReserveInput {
    pub token: String,
    pub quantity: u64,
}
/// Tokens are unique per checkout and are never recycled.
pub const RESERVE: Action<Stock, ReserveInput> = Action::new("reserve", |stock, input| {
    let ReserveInput { token, quantity } = input;
    if token.is_empty() || quantity == 0 {
        return Err(Error::invalid(
            "reservation",
            "nonempty token and positive quantity required",
        ));
    }
    if let Some(existing) = stock.reservations.get(&token) {
        return if *existing == quantity {
            Ok(vec![])
        } else {
            Err(Error::Conflict)
        };
    }
    let used = stock
        .reservations
        .values()
        .try_fold(0u64, |sum, n| sum.checked_add(*n))
        .ok_or_else(|| Error::invalid("reservations", "quantity overflow"))?;
    if quantity > stock.total.saturating_sub(used) {
        return Err(Error::invalid("quantity", "insufficient stock"));
    }
    stock.reservations.insert(token, quantity);
    Ok(vec![])
});
/// Business idempotence complements the durable command identity. Other tokens stay intact.
pub const RELEASE: Action<Stock, String> = Action::new("release", |stock, token| {
    stock.reservations.remove(&token);
    Ok(vec![])
});
pub const RECORD_PAYMENT: Action<Checkout, String> =
    Action::new("record-payment", checkout_rules::record_payment::<Checkout>);
pub fn declarations(builder: Builder) -> Builder {
    with_checkout(
        builder,
        checkout_rules::definition::<Checkout>()
            .replay_from::<crate::upgrade::legacy::CheckoutV1>(),
        Checkout::payment_outcome_field(),
    )
}
pub(crate) fn with_checkout<R: checkout_rules::CheckoutState>(
    builder: Builder,
    definition: Definition<R>,
    payment_field: FieldRef<R, String>,
) -> Builder {
    builder
        .resource(
            Stock::definition()
                .validate_transition(valid_stock)
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(RESERVE)
                .action(RELEASE),
        )
        .resource(definition)
        .reaction(
            Reaction::new(
                "rejected-checkout-release",
                1,
                service(),
                RELEASE,
                checkout_rules::rejected::<R>,
            )
            .depends_on(payment_field),
        )
}
pub async fn bootstrap(runtime: &Runtime) -> Result<()> {
    seed(
        runtime,
        "workshop-stock",
        Stock {
            total: 10,
            reservations: BTreeMap::new(),
        },
    )
    .await?;
    // Record compensation context before any forward reservation/payment attempt.
    seed(
        runtime,
        "checkout-a",
        Checkout {
            stock_id: ResourceRef::new("workshop-stock")?,
            reservation_id: "checkout-a".into(),
            payment_outcome: "pending".into(),
        },
    )
    .await
}
