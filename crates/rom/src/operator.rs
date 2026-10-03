//! Bounded, transport-neutral operator work recovery protocol.
mod protocol;
pub use protocol::*;

pub(crate) mod receipts;
pub use receipts::{
    OperatorLedger, OperatorReceiptLimits, WorkControlPrior, WorkControlReceipt, WorkScope,
};
mod status;
pub(crate) use status::work_status;

pub(crate) mod authorization;
pub use authorization::{OperatorAccess, OperatorAuthorizer, OperatorLimits};
mod control;
mod control_execution;
mod cursor;
mod projection;
mod runtime;
