//! Native scalar index maintenance, integrity and query execution.
mod catalog;
mod derivation;
mod encoding;
mod layout;
mod metadata;
mod planner;
mod query;
mod validation;

pub(crate) use catalog::{initialize, rebuild, register, replace};
pub(crate) use encoding::encode;
pub(crate) use query::query_read;
#[cfg(feature = "test-support")]
pub(crate) use query::query_read_observed;
pub(crate) use validation::validate;

#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod encoding_tests;
#[cfg(test)]
mod query_tests;
