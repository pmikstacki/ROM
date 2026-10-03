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
