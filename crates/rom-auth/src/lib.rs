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

/// Verified provider-profile distinction, independent of any stored User record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrincipalKind {
    /// An end user, requiring the host's explicit User-linking policy.
    Human,
    /// A machine identity, requiring the host's explicit service permissions.
    Service,
}

/// Immutable evidence constructed only after adapter verification.
///
/// No credential, email, arbitrary claim map, or serialized actor is retained.
/// `Debug` also omits the subject. This type implements neither `Serialize` nor
/// `Deserialize`; loading request JSON is not verification.
///
/// ```compile_fail
/// use rom_auth::{PrincipalKind, VerifiedIdentity};
/// let forged = VerifiedIdentity {
///     authority: "issuer".into(), subject: "admin".into(),
///     principal_kind: PrincipalKind::Human, valid_until: u64::MAX,
/// };
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedIdentity {
    authority: String,
    subject: String,
    principal_kind: PrincipalKind,
    valid_until: u64,
}
impl VerifiedIdentity {
    #[cfg(any(feature = "jwt", feature = "introspection"))]
    fn verified(
        authority: &str,
        subject: String,
        principal_kind: PrincipalKind,
        valid_until: u64,
    ) -> Self {
        Self {
            authority: authority.into(),
            subject,
            principal_kind,
            valid_until,
        }
    }
    /// Host-configured authority namespace, bound to this verifier's trusted issuer.
    pub fn authority(&self) -> &str {
        &self.authority
    }
    /// Opaque verified subject; unique only within its authority and principal kind.
    pub fn subject(&self) -> &str {
        &self.subject
    }
    /// Kind established by the configured provider profile, not guessed from email.
    pub fn principal_kind(&self) -> PrincipalKind {
        self.principal_kind
    }
    /// Exclusive Unix-seconds deadline. The host must deny use at or after it.
    pub fn valid_until(&self) -> u64 {
        self.valid_until
    }
}
impl std::fmt::Debug for VerifiedIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedIdentity")
            .field("authority", &self.authority)
            .field("principal_kind", &self.principal_kind)
            .field("valid_until", &self.valid_until)
            .finish_non_exhaustive()
    }
}

/// Safe failure categories: no bearer, client credential, raw body or upstream error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    /// Invalid signature, malformed data or invalid trusted key set.
    Invalid,
    /// A token or response exceeds its profile's fixed input bound.
    TooLarge,
    /// Unsupported token type, principal mapping, algorithm or proof requirement.
    WrongProfile,
    /// A bounded trusted key refresh did not resolve the requested key ID.
    UnknownKey,
    /// A key refresh was attempted before the allowed retry interval.
    RefreshLimited,
    /// Required provider or key acquisition failed or returned unsuccessful HTTP status.
    Unavailable,
    /// The configured introspection endpoint reported an inactive token.
    Inactive,
    /// Time claims fall outside the selected profile's validity window.
    Expired,
    /// Required issuer or intended-resource binding was missing or incorrect.
    Binding,
    /// Endpoint transport, credentials or location violate the selected endpoint policy.
    UnsafeEndpoint,
    /// Empty or ambiguous trusted host configuration.
    InvalidConfiguration,
}
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "authentication failed: {self:?}")
    }
}
impl std::error::Error for AuthError {}

#[cfg(any(feature = "jwt", feature = "introspection"))]
fn valid_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048 && value.trim() == value
}
