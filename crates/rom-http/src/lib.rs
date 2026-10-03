//! HTTP wire binding. The host resolver establishes trusted identity; request JSON never does.
//! Every route is generic across registered Resource kinds. TLS and credential verification
//! belong to the host. Use `serve` to coordinate stream termination and runtime draining.
#![forbid(unsafe_code)]
mod error;
#[cfg(test)]
mod error_tests;
mod json;
mod observation;
mod request;
mod routes;
mod server;

pub use server::{AuthResolver, Http, Limits};
