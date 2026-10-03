//! Native identity Resources and explicit host linking of verified external identities.
//!
//! Register these ordinary Resources with explicit row/field policies. Install
//! [`IdentityGate`] on the runtime, read a [`ProviderActivation`] through an
//! authorized host identity, and use that activation's configuration when creating
//! the credential verifier. Binding is not provider verification or a login endpoint.
#![doc = include_str!("../README.md")]

mod binding;
mod gate;
mod resources;

pub use binding::{ActivatedIdentity, ProviderActivation, link_key, linked_user_id};
pub use gate::IdentityGate;
pub use resources::{IdentityLink, IdentityProvider, ProviderProfile, User};
