//! Current authorized control preparation, gate-free verification and atomic recheck.
use super::{
    control::{self, PreparedDecision},
    *,
};
use crate::*;
enum CurrentControl {
    Replay(WorkControlResult),
    Work {
        record: Box<WorkRecord>,
        scope: WorkScope,
    },
}
impl Runtime {
    pub(super) fn execute_operator_control(
        &self,
        actor: &Actor,
        request: &WorkControlRequest,
    ) -> Result<WorkControlResult> {
        let (record, definition) = {
            let _gate = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            let (record, scope) = match self.current_operator_control(actor, request)? {
                CurrentControl::Replay(result) => return Ok(result),
                CurrentControl::Work { record, scope } => (record, scope),
            };
            control::reserve_response(request, &self.0.operator_limits.responses)?;
            match self.prepare_operator_decision(&record, &request.operation)? {
                PreparedDecision::Atomic(decision) => {
                    return self.commit_operator_control(actor, request, &scope, decision);
                }
                PreparedDecision::Verify(definition) => (record, definition),
            }
        };
        // No core gate or storage guard crosses the trusted provider callback.
        let verifier = definition.verifier.clone().ok_or(Error::Storage)?;
        let WorkControlOperation::Reconcile { evidence_ref } = &request.operation else {
            return Err(Error::Storage);
        };
        let input = DeliveryReconciliation {
            id: record.pending.id.clone(),
            evidence_ref: evidence_ref.clone(),
        };
        let timeout = definition
            .verification_timeout
            .unwrap_or(self.0.delivery_timeout);
        let decision = match channels::supervision::run(timeout, move || verifier(input)) {
            channels::supervision::Callback::Returned(DeliveryVerification::Accepted {
                evidence,
            }) if receipts::check_identifier(&evidence).is_ok() => {
                Some(WorkControlDecision::DeliveryAccepted { evidence })
            }
            channels::supervision::Callback::Returned(DeliveryVerification::NotAccepted {
                evidence,
            }) if receipts::check_identifier(&evidence).is_ok() => {
                Some(WorkControlDecision::DeliveryNotAccepted { evidence })
            }
            _ => None,
        };
        let _gate = self.0.gate.lock().map_err(|_| Error::Panicked)?;
        // Exact concurrent receipts are arbitrated before CAS, under current saved-scope authority.
        let (current, scope) = match self.current_operator_control(actor, request)? {
            CurrentControl::Replay(result) => return Ok(result),
            CurrentControl::Work { record, scope } => (record, scope),
        };
        self.prepare_operator_decision(&current, &request.operation)?;
        match decision {
            Some(decision) => self.commit_operator_control(actor, request, &scope, decision),
            None => {
                let result = WorkControlResult {
                    protocol_version: OPERATOR_PROTOCOL_VERSION,
                    handle: request.handle.clone(),
                    version: request.expected.clone(),
                    key: request.key.clone(),
                    operation: request.operation.clone(),
                    outcome: WorkControlOutcome::Unresolved,
                    replayed: false,
                };
                result.validate_for(request, &self.0.operator_limits.responses)?;
                Ok(result)
            }
        }
    }
    /// Caller holds the core gate. Exact receipt arbitration precedes expected-version CAS.
    fn current_operator_control(
        &self,
        actor: &Actor,
        request: &WorkControlRequest,
    ) -> Result<CurrentControl> {
        let access = control::access(&request.operation);
        self.authorize_operator(actor, access, None)?;
        self.require_operator_storage()?;
        let snapshot = self.operator_snapshot()?;
        let principal = actor.key();
        snapshot
            .retry_epochs
            .check(request.retry_epoch, true, false)?;
        if let Some(receipt) = snapshot.operator.receipt(&principal, request) {
            self.authorize_operator(actor, access, Some(&receipt.scope))?;
            if receipt.request != *request {
                return Err(Error::IdentityMismatch);
            }
            control::validate_receipt(
                &principal,
                request,
                receipt,
                &self.0.operator_limits.responses,
            )
            .map_err(|_| Error::Unknown)?;
            let mut result = receipt.result.clone();
            result.replayed = true;
            self.check_authority(actor)?;
            return Ok(CurrentControl::Replay(result));
        }
        snapshot
            .retry_epochs
            .check(request.retry_epoch, false, false)?;
        let record = snapshot
            .records
            .iter()
            .find(|record| WorkHandle::from_work_id(&record.pending.id) == request.handle)
            .ok_or(Error::Denied)?;
        let scope = WorkScope::from_record(record)?;
        self.authorize_operator(actor, access, Some(&scope))?;
        if snapshot.version(record) != request.expected
            || matches!(record.state, WorkState::Leased { .. } | WorkState::Done)
        {
            return Err(Error::Conflict);
        }
        Ok(CurrentControl::Work {
            record: Box::new(record.clone()),
            scope,
        })
    }
    /// Caller holds the gate and has checked current version, definition and authority.
    fn commit_operator_control(
        &self,
        actor: &Actor,
        request: &WorkControlRequest,
        scope: &WorkScope,
        decision: WorkControlDecision,
    ) -> Result<WorkControlResult> {
        let access = control::access(&request.operation);
        self.authorize_operator(actor, access, Some(scope))?;
        let principal = actor.key();
        let prepared = StorageWorkControl {
            principal: principal.clone(),
            request: request.clone(),
            decision,
            now: self.0.clock.now(),
        };
        let receipt = self.0.storage.control_work(&prepared)?;
        control::validate_receipt(
            &principal,
            request,
            &receipt,
            &self.0.operator_limits.responses,
        )
        .map_err(|_| Error::Unknown)?;
        self.authorize_operator(actor, access, Some(scope))
            .map_err(|_| Error::Unknown)?;
        self.0
            .changes
            .send_modify(|change| *change = change.wrapping_add(1));
        Ok(receipt.result)
    }
}
