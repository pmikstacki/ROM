//! Optional Studio host. The Resource runtime remains independent of this crate.
#![doc = include_str!("../README.md")]

mod assets;
mod authentication;
mod blob_capabilities;
mod blobs;
mod configuration;
mod csrf;
mod lifecycle;
mod login;
mod observation_gate;
mod oidc;
mod router;
mod session;
mod session_observation;
mod settings;

pub use configuration::{HostConfig, HostLimits, OidcProviderConfig};
pub use router::StudioHost;
pub use settings::StudioSettings;

#[cfg(test)]
mod assets_tests;
#[cfg(test)]
mod csrf_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod login_tests;
#[cfg(test)]
mod oidc_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod shutdown_tests;
