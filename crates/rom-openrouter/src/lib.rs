//! Optional bounded HTTP adapter. Credentials and provider payloads stay outside ROM core.
mod catalog;
mod config;
mod credentials;
mod endpoints;
mod errors;
mod execution;
mod headers;
mod money;
mod nonacceptance;
mod reconcile;
mod request;
mod response;
mod transport;

pub use config::OpenRouterConfig;
pub use credentials::{CredentialSource, SecretToken};
pub use transport::OpenRouter;
