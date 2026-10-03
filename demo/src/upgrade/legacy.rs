//! Frozen Checkout v1 codec and the explicit offline transition to typed references.
use crate::{
    Notices, application,
    compensation::{
        self, Checkout, Stock,
        checkout_rules::{self, CheckoutState, CheckoutView},
    },
    model::{Dashboard, Task},
    service,
};
use rom::{
    Builder, Error, Input, Invocation, Operation, PendingWork, Resource, ResourceRef, Result, Row,
    WorkPayload,
};
use rom_backup::{MigrationPlan, ResourceMigration};

/// Preserve the original field types and default schema version for old receipts.
#[derive(Clone, Debug, Resource)]
#[resource(name = "checkouts")]
pub(crate) struct CheckoutV1 {
    pub stock_id: String,
    pub reservation_id: String,
    pub payment_outcome: String,
}
impl CheckoutState for CheckoutV1 {
    fn view(&self) -> CheckoutView<'_> {
        CheckoutView {
            stock_id: &self.stock_id,
            reservation_id: &self.reservation_id,
            payment_outcome: &self.payment_outcome,
        }
    }
    fn payment_outcome_mut(&mut self) -> &mut String {
        &mut self.payment_outcome
    }
}
pub(crate) fn declarations(notices: Notices) -> Result<Builder> {
    Ok(compensation::with_checkout(
        application::base_declarations(notices)?,
        checkout_rules::definition::<CheckoutV1>(),
        CheckoutV1::payment_outcome_field(),
    ))
}
pub(crate) fn migration_plan() -> Result<MigrationPlan> {
    Ok(
        MigrationPlan::new(vec![ResourceMigration::new::<CheckoutV1, Checkout>(
            |old| {
                Ok(Checkout {
                    stock_id: ResourceRef::new(old.stock_id)?,
                    reservation_id: old.reservation_id,
                    payment_outcome: old.payment_outcome,
                })
            },
        )?])?
        .validate_work(validate_work),
    )
}
fn validate_row<R: Resource>(row: &Row) -> Result<()> {
    if row.key.kind != R::KIND || row.key.id.is_empty() || row.revision == 0 {
        return Err(Error::Unsupported("upgrade work source".into()));
    }
    row.map_resource_values(&mut |_, value| {
        if R::decode(value.clone())?.encode() != *value {
            return Err(Error::Unsupported("upgrade work source codec".into()));
        }
        Ok(value.clone())
    })?;
    Ok(())
}
fn validate_action(
    work: &PendingWork,
    value: &rom::Value,
    target: &str,
    action: &str,
) -> Result<()> {
    let invocation: Invocation =
        serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
    if invocation.kind != target
        || invocation.id.is_empty()
        || invocation.idempotency != work.id
        || invocation.expected.is_none_or(|revision| revision == 0)
        || invocation.retry_epoch != work.cause.retry_epoch
    {
        return Err(Error::Unsupported("upgrade work invocation".into()));
    }
    match invocation.operation {
        Operation::Action { name, input } if name == action => {
            String::decode(input)?;
            Ok(())
        }
        _ => Err(Error::Unsupported("upgrade work action".into())),
    }
}
fn validate_work(work: &PendingWork) -> Result<()> {
    let actor = service();
    let service_key =
        rom::json!([actor.authority, actor.principal_kind(), actor.subject]).to_string();
    if work.version != 1 || work.service_key != service_key {
        return Err(Error::Unsupported("upgrade work version or service".into()));
    }
    match (work.definition.as_str(), &work.payload) {
        ("rejected-checkout-release", WorkPayload::Source(row)) => validate_row::<Checkout>(row),
        ("rejected-checkout-release", WorkPayload::Action(value)) => {
            validate_action(work, value, Stock::KIND, "release")
        }
        ("completed-task-dashboard", WorkPayload::Source(row)) => validate_row::<Task>(row),
        ("completed-task-dashboard", WorkPayload::Action(value)) => {
            validate_action(work, value, Dashboard::KIND, "display")
        }
        ("local-completions", WorkPayload::Notification { source, payload }) => {
            validate_row::<Dashboard>(source)?;
            String::decode(payload.clone())?;
            Ok(())
        }
        _ => Err(Error::Unsupported(
            "upgrade work definition or payload".into(),
        )),
    }
}
#[cfg(test)]
#[path = "legacy_tests.rs"]
mod tests;
