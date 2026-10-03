//! Explicit business compensation after a confirmed, simulated payment rejection.
//! Unknown outcomes require reconciliation; terminal worker errors are not inferred here.
use crate::{domain, seed, service};
use rom::{Action, Builder, Error, Reaction, Resource, Result, Runtime, Snapshot, Target};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Resource)]
#[resource(name = "reservation-stock")]
pub struct Stock {
    pub total: u64,
    pub reservations: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Resource)]
#[resource(name = "checkouts")]
pub struct Checkout {
    pub stock_id: String,
    pub reservation_id: String,
    pub payment_outcome: String,
}
/// One token and quantity. Tokens are unique per checkout and are never recycled.
pub const RESERVE: Action<Stock, BTreeMap<String, u64>> = Action::new("reserve", |stock, input| {
    if input.len() != 1 {
        return Err(Error::invalid("reservation", "one token required"));
    }
    let (token, quantity) = input.into_iter().next().expect("one entry");
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
    Action::new("record-payment", |checkout, outcome| {
        if !matches!(
            outcome.as_str(),
            "unknown" | "transient" | "confirmed_rejected" | "succeeded"
        ) {
            return Err(Error::invalid(
                "payment_outcome",
                "explicit simulated outcome required",
            ));
        }
        if matches!(
            checkout.payment_outcome.as_str(),
            "confirmed_rejected" | "succeeded"
        ) && checkout.payment_outcome != outcome
        {
            return Err(Error::invalid(
                "payment_outcome",
                "terminal outcome cannot change",
            ));
        }
        checkout.payment_outcome = outcome;
        Ok(vec![])
    });
fn rejected(checkout: &Snapshot<Checkout>) -> Result<Vec<Target<String>>> {
    Ok(checkout
        .value
        .as_ref()
        .filter(|c| c.payment_outcome == "confirmed_rejected")
        .map(|c| vec![Target::new(&c.stock_id, c.reservation_id.clone())])
        .unwrap_or_default())
}
pub fn declarations(builder: Builder) -> Builder {
    builder
        .resource(
            Stock::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(RESERVE)
                .action(RELEASE),
        )
        .resource(
            Checkout::definition()
                .policy(|a, _, _| domain(a))
                .allow_all_fields()
                .discovery_policy(|a, _| domain(a))
                .action(RECORD_PAYMENT),
        )
        .reaction(
            Reaction::new("rejected-checkout-release", 1, service(), RELEASE, rejected)
                .depends_on(Checkout::payment_outcome_field()),
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
            stock_id: "workshop-stock".into(),
            reservation_id: "checkout-a".into(),
            payment_outcome: "pending".into(),
        },
    )
    .await
}
