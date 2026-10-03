//! Owned persistence reads and pure query-strategy selection.
mod protocol;
mod selection;
#[cfg(test)]
mod tests;
mod validation;

pub use protocol::*;
pub(crate) use selection::scalar_shape;
pub use selection::select_query_strategy;
pub(crate) use validation::validate_query_read;
