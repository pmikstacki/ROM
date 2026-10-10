//! Transaction-local keyed Work persistence and explicit canonical maintenance.
pub(crate) mod batch;
mod bounded;
mod canonical;
pub(crate) mod claim_prefix;
mod operator_admission;
mod prepared;
mod read;
mod schema;
mod write;
pub(crate) use prepared::PreparedWork;

pub(crate) use canonical::reconstruct;
pub(crate) use read::{Reader, metadata};
pub(crate) use schema::{FORMAT, PREDECESSOR_FORMAT, initialize, validate_layout};
#[cfg(not(feature = "test-support"))]
pub(crate) use write::save_metadata;
pub(crate) use write::{replace_state, replace_state_for_layout};

pub(crate) use operator_admission::admit as admit_operator;

#[cfg(feature = "test-support")]
mod publication;
#[cfg(feature = "test-support")]
pub(crate) use publication::save_metadata_observed;
