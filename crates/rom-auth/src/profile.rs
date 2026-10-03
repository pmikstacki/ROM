use crate::PrincipalKind;

/// Credential contract sealed into verified evidence by its adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityProfile {
    /// RS256 human OAuth access token.
    JwtRs256Human,
    /// Active OAuth service introspection result.
    OAuthIntrospectionService,
    /// RS256 human OpenID Connect ID token from the authorization-code flow.
    OidcRs256Human,
}
impl IdentityProfile {
    /// Stable profile identifier for host configuration and evidence correspondence.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JwtRs256Human => "jwt-rs256-human",
            Self::OAuthIntrospectionService => "oauth-introspection-service",
            Self::OidcRs256Human => "oidc-rs256-human",
        }
    }
    /// Principal kind fixed by this credential contract.
    pub fn principal_kind(self) -> PrincipalKind {
        match self {
            Self::JwtRs256Human | Self::OidcRs256Human => PrincipalKind::Human,
            Self::OAuthIntrospectionService => PrincipalKind::Service,
        }
    }
}
