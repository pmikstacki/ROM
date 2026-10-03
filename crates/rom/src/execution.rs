//! Runtime composition, supervision and resource execution.
mod authority;
mod builder;
mod command;
mod lifecycle;
mod state;

pub use builder::Builder;
pub(crate) use lifecycle::acquire;
pub use lifecycle::{IntakeState, Limits, RuntimeStatus};
pub use state::Runtime;
pub(crate) use state::{Inner, Work};
