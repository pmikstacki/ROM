//! Object-store adapters for the maintained ROM blob port.
#![doc = include_str!("../README.md")]

mod adapter;
mod configuration;
mod errors;

pub use adapter::Adapter;
pub use configuration::{EndpointPolicy, S3Config};
