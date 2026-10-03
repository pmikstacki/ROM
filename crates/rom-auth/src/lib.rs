//! Optional credential verifiers for ROM hosts.
//!
//! Enable `jwt` for a bounded RS256 human access-token profile, or `introspection`
//! for a configured OAuth introspection service profile. No features are enabled
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

#[cfg(any(feature = "jwt", feature = "introspection"))]
mod claims;
mod error;
mod identity;

#[cfg(any(feature = "jwt", feature = "introspection"))]
use claims::{present_claim, valid_name};
pub use error::AuthError;
pub use identity::{PrincipalKind, VerifiedIdentity};
