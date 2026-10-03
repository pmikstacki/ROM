//! Bounded durable operator evidence and exact identity arbitration.
use super::protocol::*;
use crate::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkScope {
    pub category: WorkCategory,
    pub definition: WorkDefinition,
    pub source: Option<Key>,
    pub target: Option<Key>,
}
impl WorkScope {
    pub(crate) fn from_record(record: &WorkRecord) -> Result<Self> {
        let (category, source, target) = match &record.pending.payload {
            WorkPayload::Source(row) => (WorkCategory::Reaction, Some(row.key.clone()), None),
            WorkPayload::Notification { source, .. } => {
                (WorkCategory::Notification, Some(source.key.clone()), None)
            }
            WorkPayload::Action(value) => {
                let invocation: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                (
                    WorkCategory::Reaction,
                    None,
                    Some(Key {
                        kind: invocation.kind,
                        id: invocation.id,
                    }),
                )
            }
        };
        Ok(Self {
            category,
            definition: WorkDefinition {
                name: record.pending.definition.clone(),
                version: record.pending.version,
            },
            source,
            target,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkControlPrior {
    pub state: WorkStatus,
    #[serde(deserialize_with = "required_delivery")]
    pub delivery: Option<DeliveryOutcome>,
}
fn required_delivery<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> std::result::Result<Option<DeliveryOutcome>, D::Error> {
    Option::<DeliveryOutcome>::deserialize(de)
}
impl WorkControlPrior {
    pub(crate) fn from_record(record: &WorkRecord) -> Self {
        Self {
            state: super::work_status(&record.state),
            delivery: record.delivery.clone(),
        }
    }
    fn validate(
        &self,
        scope: &WorkScope,
        operation: &WorkControlOperation,
        outcome: &WorkControlOutcome,
    ) -> Result<()> {
        if matches!(self.state, WorkStatus::Leased | WorkStatus::Done)
            || (scope.category == WorkCategory::Reaction
                && (self.delivery.is_some() || self.state == WorkStatus::AwaitingReconciliation))
            || (self.state == WorkStatus::AwaitingReconciliation
                && (self.delivery.is_none() || self.delivery == Some(DeliveryOutcome::Accepted)))
            || (matches!(operation, WorkControlOperation::Retry)
                && self.state == WorkStatus::AwaitingReconciliation)
            || (matches!(outcome, WorkControlOutcome::Scheduled)
                && matches!(
                    self.state,
                    WorkStatus::Stopped(
                        StopReason::Depth
                            | StopReason::Fanout
                            | StopReason::Attempts
                            | StopReason::WorkBudget
                            | StopReason::Age
                    )
                ))
        {
            return Err(Error::Storage);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkControlReceipt {
    pub principal: String,
    pub request: WorkControlRequest,
    pub result: WorkControlResult,
    pub now: u64,
    pub scope: WorkScope,
    pub prior: WorkControlPrior,
    pub evidence: Option<String>,
}
impl WorkControlReceipt {
    /// Trusted receipt coherence; reply replay flags are allowed here.
    /// Durable ledger validation separately rejects a stored replay flag.
    pub(crate) fn validate(&self) -> Result<()> {
        check_identifier(&self.principal)?;
        self.request.validate()?;
        self.prior
            .validate(&self.scope, &self.request.operation, &self.result.outcome)?;
        if let Some(evidence) = &self.evidence {
            check_identifier(evidence)?;
        }
        if self.result.protocol_version != OPERATOR_PROTOCOL_VERSION
            || matches!(self.result.outcome, WorkControlOutcome::Unresolved)
        {
            return Err(Error::Storage);
        }
        self.result.validate_correspondence(&self.request)?;
        let valid_outcome = match (
            &self.request.operation,
            &self.scope.category,
            &self.result.outcome,
        ) {
            (WorkControlOperation::Retry, _, WorkControlOutcome::Scheduled) => {
                self.evidence.is_none()
            }
            (
                WorkControlOperation::Reconcile { .. },
                WorkCategory::Reaction,
                WorkControlOutcome::Completed,
            ) => {
                self.evidence.is_none()
                    && self.scope.target.is_some()
                    && self.scope.source.is_none()
            }
            (
                WorkControlOperation::Reconcile { .. },
                WorkCategory::Notification,
                WorkControlOutcome::Completed
                | WorkControlOutcome::Scheduled
                | WorkControlOutcome::Stopped(_),
            ) => {
                self.evidence.is_some()
                    && self.scope.source.is_some()
                    && self.scope.target.is_none()
            }
            _ => false,
        };
        if !valid_outcome {
            return Err(Error::Storage);
        }
        self.result.version.validate()?;
        check_identifier(&self.scope.definition.name)?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorReceiptLimits {
    pub max_records: usize,
    pub max_bytes: usize,
}
impl Default for OperatorReceiptLimits {
    fn default() -> Self {
        Self {
            max_records: 1024,
            max_bytes: 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorLedger {
    pub limits: OperatorReceiptLimits,
    pub receipts: BTreeMap<String, WorkControlReceipt>,
}
pub(crate) fn receipt_identity(principal: &str, request: &WorkControlRequest) -> String {
    json!([principal, request.retry_epoch, request.key]).to_string()
}
pub(crate) fn check_identifier(value: &str) -> Result<()> {
    bounded(value, MAX_OPERATOR_IDENTIFIER_BYTES)
}
impl OperatorLedger {
    /// Lookup stays trusted; Runtime checks current authority before disclosure.
    pub fn receipt(
        &self,
        principal: &str,
        request: &WorkControlRequest,
    ) -> Option<&WorkControlReceipt> {
        self.receipts.get(&receipt_identity(principal, request))
    }
    pub(crate) fn check_bounds(&self) -> Result<()> {
        if self.limits.max_records == 0 || self.limits.max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        if self.receipts.len() > self.limits.max_records
            || serde_json::to_vec(&self.receipts)
                .map_err(|_| Error::Storage)?
                .len()
                > self.limits.max_bytes
        {
            return Err(Error::Overloaded);
        }
        Ok(())
    }
    pub(crate) fn validate_archive(&self) -> Result<()> {
        self.check_bounds()?;
        for (identity, receipt) in &self.receipts {
            receipt.validate()?;
            if identity != &receipt_identity(&receipt.principal, &receipt.request)
                || receipt.result.replayed
            {
                return Err(Error::Storage);
            }
        }
        Ok(())
    }
}
