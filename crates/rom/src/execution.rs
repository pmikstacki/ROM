//! Runtime composition, supervision and resource execution.
mod authority;
mod builder;
mod calculation;
mod command;
pub(crate) mod diagnostic_record;
mod lifecycle;
mod overload;
mod state;

#[cfg(test)]
mod authority_batch_tests;
#[cfg(test)]
mod overload_fixture;
#[cfg(test)]
mod overload_generation;
#[cfg(test)]
mod overload_tests;

pub use builder::Builder;
pub(crate) use lifecycle::acquire;
pub use lifecycle::{IntakeState, Limits, RuntimeStatus};
pub use state::Runtime;
pub(crate) use state::{Inner, Work};
