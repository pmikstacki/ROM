//! Optional credential verifiers for ROM hosts.
//!
//! Enable `jwt` for RS256 human access tokens, `oidc` for RS256 human ID tokens,
//! or `introspection` for configured OAuth service introspection. No features are enabled
//! by default. This crate has no dependency on ROM's actor, Resource or persistence
//! implementation. A [`VerifiedIdentity`] is evidence, not an authorization grant.
//!
//! The host must preserve its principal kind and exclusive expiry when constructing
//! an actor, resolve explicit User bindings through ordinary Resources, and apply
//! current authorization on every operation. This crate intentionally supplies no
//! conversion to an actor that would discard expiry or principal kind.
#![deny(missing_docs)]

#[cfg(feature = "introspection")]
pub mod introspection;
#[cfg(feature = "jwt")]
pub mod jwt;
#[cfg(feature = "oidc")]
pub mod oidc;

#[cfg(any(feature = "jwt", feature = "introspection"))]
mod claims;
mod error;
mod identity;
#[cfg(feature = "jwt")]
mod keys;
mod profile;

#[cfg(any(feature = "jwt", feature = "introspection"))]
use claims::{present_claim, valid_name};
pub use error::AuthError;
pub use identity::{PrincipalKind, VerifiedIdentity};
#[cfg(feature = "oidc")]
pub use oidc::OidcIdTokenAdapter;
pub use profile::IdentityProfile;
