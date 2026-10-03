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
    pub(crate) fn verified(
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
