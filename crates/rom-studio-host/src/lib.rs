//! Optional Studio host. The Resource runtime remains independent of this crate.
#![doc = include_str!("../README.md")]

mod assets;
mod auth_diagnostics;
mod authentication;
mod backchannel;
mod blob_capabilities;
mod blobs;
mod bootstrap;
mod configuration;
mod csrf;
mod jwks;
mod lifecycle;
mod login;
mod observation_gate;
mod oidc;
mod proof_handoff;
mod router;
mod session;
mod session_observation;
mod settings;

pub use auth_diagnostics::{
    AuthFailure, AuthOperation, AuthOperationCounts, AuthOutcome, AuthSnapshot, AuthStage,
    AuthStageCounts,
};
pub use backchannel::TrustedLoopbackBackchannel;
pub use bootstrap::{BrowserStore, StudioBootstrap};
pub use configuration::{HostConfig, HostLimits, OidcProviderConfig};
pub use router::StudioHost;
pub use settings::StudioSettings;

#[cfg(test)]
mod assets_tests;
#[cfg(test)]
mod auth_diagnostics_tests;
#[cfg(test)]
mod backchannel_tests;
#[cfg(test)]
mod bootstrap_tests;
#[cfg(test)]
mod csrf_tests;
#[cfg(test)]
mod jwks_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod login_tests;
#[cfg(test)]
mod oidc_provider_metadata_tests;
#[cfg(test)]
mod oidc_tests;
#[cfg(test)]
mod proof_handoff_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod shutdown_tests;
