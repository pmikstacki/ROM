//! Bounded RS256 ID-token verification for a host-owned authorization-code flow.
//!
//! The host owns discovery approval, code redemption, PKCE, one-use state and nonce,
//! and issuer-bound key acquisition. This adapter makes no network requests and
//! accepts no token-selected trust source. It retains no token or nonce.
//! ID-token evidence is distinct from OAuth access-token evidence.
//!
//! ```no_run
//! use rom_auth::{AuthError, OidcIdTokenAdapter, VerifiedIdentity};
//! use rom_auth::{jwt::TrustedKeys, oidc::OidcTokenBindings};
//! fn verify<K: TrustedKeys>(source: K, id_token: &str, retained_nonce: &str,
//!     access_token: &str, redeemed_code: &str, now: u64)
//!     -> Result<VerifiedIdentity, AuthError>
//! {
//!     OidcIdTokenAdapter::configured("employees", "https://issuer.example",
//!         "studio-client", source)?.authenticate(id_token, retained_nonce,
//!             OidcTokenBindings { access_token: Some(access_token),
//!                 authorization_code: Some(redeemed_code) }, now)
//! }
//! ```

mod adapter;
mod bindings;
mod claims;
mod header;
pub use adapter::OidcIdTokenAdapter;
pub use bindings::OidcTokenBindings;

/// Maximum local proof lifetime in seconds; token and key expiry can shorten it.
pub const MAX_PROOF_SECONDS: u64 = 30;
