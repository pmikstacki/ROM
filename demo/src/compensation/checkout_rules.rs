//! Shared checkout invariants and compensation mapping across stored schema versions.
use super::Checkout;
use crate::domain;
use rom::{Action, Actor, Definition, Error, Intent, Resource, Result, Snapshot, Target};

pub(crate) struct CheckoutView<'a> {
    pub stock_id: &'a str,
    pub reservation_id: &'a str,
    pub payment_outcome: &'a str,
}
pub(crate) trait CheckoutState: Resource {
    fn view(&self) -> CheckoutView<'_>;
    fn payment_outcome_mut(&mut self) -> &mut String;
}
impl CheckoutState for Checkout {
    fn view(&self) -> CheckoutView<'_> {
        CheckoutView {
            stock_id: self.stock_id.id(),
            reservation_id: &self.reservation_id,
            payment_outcome: &self.payment_outcome,
        }
    }
    fn payment_outcome_mut(&mut self) -> &mut String {
        &mut self.payment_outcome
    }
}
fn terminal(outcome: &str) -> bool {
    matches!(outcome, "confirmed_rejected" | "succeeded")
}
fn validate<R: CheckoutState>(_: &Actor, before: Option<&R>, after: Option<&R>) -> Result<()> {
    let before = before.map(CheckoutState::view);
    let after = after.map(CheckoutState::view);
    if let Some(checkout) = &after
        && (checkout.stock_id.is_empty()
            || checkout.reservation_id.is_empty()
            || !matches!(
                checkout.payment_outcome,
                "pending" | "unknown" | "transient" | "confirmed_rejected" | "succeeded"
            ))
    {
        return Err(Error::invalid(Checkout::KIND, "checkout state"));
    }
    if let Some(old) = before {
        if let Some(new) = &after
            && (old.stock_id != new.stock_id || old.reservation_id != new.reservation_id)
        {
            return Err(Error::invalid(Checkout::KIND, "compensation context"));
        }
        if terminal(old.payment_outcome)
            && after.is_none_or(|new| new.payment_outcome != old.payment_outcome)
        {
            return Err(Error::invalid(
                Checkout::KIND,
                "terminal outcome cannot change",
            ));
        }
    }
    Ok(())
}
pub(crate) fn record_payment<R: CheckoutState>(
    checkout: &mut R,
    outcome: String,
) -> Result<Vec<Intent>> {
    if !matches!(
        outcome.as_str(),
        "unknown" | "transient" | "confirmed_rejected" | "succeeded"
    ) {
        return Err(Error::invalid(
            "payment_outcome",
            "explicit simulated outcome required",
        ));
    }
    let previous = checkout.view().payment_outcome;
    if terminal(previous) && previous != outcome {
        return Err(Error::invalid(
            "payment_outcome",
            "terminal outcome cannot change",
        ));
    }
    *checkout.payment_outcome_mut() = outcome;
    Ok(vec![])
}
pub(crate) fn definition<R: CheckoutState>() -> Definition<R> {
    R::definition()
        .validate_transition(validate::<R>)
        .policy(|a, _, _| domain(a))
        .allow_all_fields()
        .discovery_policy(|a, _| domain(a))
        .action(Action::new("record-payment", record_payment::<R>))
}
pub(crate) fn rejected<R: CheckoutState>(checkout: &Snapshot<R>) -> Result<Vec<Target<String>>> {
    Ok(checkout
        .value
        .as_ref()
        .map(CheckoutState::view)
        .filter(|checkout| checkout.payment_outcome == "confirmed_rejected")
        .map(|checkout| {
            vec![Target::new(
                checkout.stock_id,
                checkout.reservation_id.to_owned(),
            )]
        })
        .unwrap_or_default())
}
