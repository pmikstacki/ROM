use rom::{Error, Resource, Result};

/// Closed set of credential profiles implemented by this milestone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderProfile {
    /// RS256 access tokens with exact issuer and audience binding for humans.
    JwtRs256Human,
    /// Active OAuth introspection with service and resource binding.
    OAuthIntrospectionService,
}
impl rom::Field for ProviderProfile {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> rom::Value {
        rom::json!(match self {
            Self::JwtRs256Human => "jwt-rs256-human",
            Self::OAuthIntrospectionService => "oauth-introspection-service",
        })
    }
    fn decode(value: rom::Value) -> Result<Self> {
        match value.as_str() {
            Some("jwt-rs256-human") => Ok(Self::JwtRs256Human),
            Some("oauth-introspection-service") => Ok(Self::OAuthIntrospectionService),
            _ => Err(Error::invalid(IdentityProvider::KIND, "profile")),
        }
    }
}

/// A managed local profile; authority and permissions are separate from profile data.
#[derive(Clone, Debug, Resource)]
#[resource(name = "users")]
pub struct User {
    /// Whether linked external actors can use this profile.
    pub enabled: bool,
    /// Display data, never an authentication or automatic linking input.
    pub display_name: String,
}

/// Managed provider configuration. Credentials are references, never inline secrets.
#[derive(Clone, Debug, Resource)]
#[resource(name = "identity-providers")]
pub struct IdentityProvider {
    /// Whether this provider may establish actors.
    pub enabled: bool,
    /// Exactly `jwt-rs256-human` or `oauth-introspection-service` in this milestone.
    pub profile: ProviderProfile,
    /// Expected token issuer; the host must bind its verifier to this value.
    pub issuer: String,
    /// Expected resource audience; the host must bind its verifier to this value.
    pub audience: String,
    /// Host-approved introspection endpoint or configured trust-source identifier.
    pub endpoint: Option<String>,
    /// Host secret-store reference. Resolution and rotation belong to the host.
    pub credential_ref: Option<String>,
}

/// An explicit link, stored under [`crate::link_key`] using the same Resource pipeline.
#[derive(Clone, Debug, Resource)]
#[resource(name = "identity-links")]
pub struct IdentityLink {
    /// Provider Resource id and verified authority namespace.
    pub authority: String,
    /// Opaque provider subject; no email matching is performed.
    pub subject: String,
    /// Exactly `human` or `service`, as verified by the selected profile.
    pub principal_kind: String,
    /// Referenced User Resource id.
    pub user_id: String,
    /// Explicit link activation, independent of User/provider activation.
    pub enabled: bool,
}
