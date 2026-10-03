//! Opt-in reference host with explicit provisioning and supervised authentication.
//! Pass the same clock to this host and the Runtime builder. The caller must provide
//! trusted parent directories and immutable versioned secret files. Close and drain
//! authentication before shutting down Runtime. Serving never provisions automatically.

mod application;
mod authentication;
mod command;
mod lifecycle;
mod model;
mod provisioning;
mod secrets;
mod serving;
mod verification;

pub use application::{LocalMode, build, declarations};
pub use authentication::HostAuth;
pub use command::{ProfileConfig, run_command};
pub use model::{ApprovedProvider, AuthLimits};
pub use provisioning::{Provisioning, configuration_reader, maintain, provision};
pub use secrets::SecretFiles;
pub use serving::serve;
