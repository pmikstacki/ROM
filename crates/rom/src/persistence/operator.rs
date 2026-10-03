//! Trusted atomic operator port. These decisions cannot be decoded from client JSON.
use crate::operator::WorkControlRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageWorkControl {
    pub principal: String,
    pub request: WorkControlRequest,
    pub decision: WorkControlDecision,
    pub now: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkControlDecision {
    Retry,
    ActionCommitted,
    DeliveryAccepted { evidence: String },
    DeliveryNotAccepted { evidence: String },
}
