//! Bounded source loading into accepted native Resource definitions.
#![doc = include_str!("../README.md")]
mod parser;
pub use parser::*;
mod ingestion;
pub use ingestion::*;
