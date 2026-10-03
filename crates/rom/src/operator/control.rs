//! Trusted control preparation and conservative response admission before commit.
use super::*;
use crate::*;
pub(super) fn access(operation: &WorkControlOperation) -> OperatorAccess {
    match operation {
        WorkControlOperation::Retry => OperatorAccess::Retry,
        WorkControlOperation::Reconcile { .. } => OperatorAccess::Reconcile,
    }
}
pub(super) fn reserve_response(
    request: &WorkControlRequest,
    bounds: &WorkResponseLimits,
) -> Result<()> {
    // Reserve all result shapes without predicting or duplicating the ledger transition.
    let response = WorkControlResult {
        protocol_version: OPERATOR_PROTOCOL_VERSION,
        handle: request.handle.clone(),
        version: WorkVersion {
            generation: request.expected.generation.clone(),
            revision: u64::MAX,
        },
        key: request.key.clone(),
        operation: request.operation.clone(),
        outcome: WorkControlOutcome::Stopped(StopReason::DefinitionChanged),
        replayed: false,
    };
    bounds.check_response(&response, 1)
}
pub(super) fn validate_receipt(
    principal: &str,
    request: &WorkControlRequest,
    receipt: &WorkControlReceipt,
    bounds: &WorkResponseLimits,
) -> Result<()> {
    receipt.validate()?;
    if receipt.principal != principal || receipt.request != *request {
        return Err(Error::Storage);
    }
    receipt.result.validate_for(request, bounds)?;
    Ok(())
}
pub(super) enum PreparedDecision {
    Atomic(WorkControlDecision),
    Verify(Arc<channels::RegisteredChannel>),
}
impl Runtime {
    pub(super) fn prepare_operator_decision(
        &self,
        record: &WorkRecord,
        operation: &WorkControlOperation,
    ) -> Result<PreparedDecision> {
        let pending = &record.pending;
        let service = match &pending.payload {
            WorkPayload::Notification { .. } => {
                let definition = self.notification_definition(pending)?;
                self.authorize_notification(&definition, pending)?;
                return match operation {
                    WorkControlOperation::Retry => {
                        Ok(PreparedDecision::Atomic(WorkControlDecision::Retry))
                    }
                    WorkControlOperation::Reconcile { .. } if definition.verifier.is_some() => {
                        Ok(PreparedDecision::Verify(definition))
                    }
                    WorkControlOperation::Reconcile { .. } => Err(Error::Unsupported(
                        "delivery reconciliation requires a registered verifier".into(),
                    )),
                };
            }
            WorkPayload::Source(_) | WorkPayload::Action(_) => self
                .0
                .reactions
                .get(&pending.definition)
                .filter(|definition| {
                    definition.version == pending.version
                        && definition.actor.key() == pending.service_key
                })
                .map(|definition| &definition.actor),
        }
        .ok_or_else(|| Error::Unsupported("frozen work definition unavailable".into()))?;
        match operation {
            WorkControlOperation::Retry => Ok(PreparedDecision::Atomic(WorkControlDecision::Retry)),
            WorkControlOperation::Reconcile { .. } => {
                let WorkPayload::Action(value) = &pending.payload else {
                    return Err(Error::Unsupported(
                        "work has no frozen action receipt".into(),
                    ));
                };
                let invocation: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                if invocation.retry_epoch != pending.cause.retry_epoch {
                    return Err(Error::Storage);
                }
                if !self.resolve_frozen_action(service, &invocation)? {
                    return Err(Error::Unsupported(
                        "frozen action receipt unavailable".into(),
                    ));
                }
                Ok(PreparedDecision::Atomic(
                    WorkControlDecision::ActionCommitted,
                ))
            }
        }
    }
}
