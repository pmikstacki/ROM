use crate::IdentityProvider;
use rom::{Error, Resource, Result};
use rom_auth::IdentityProfile;

/// Closed set of managed credential profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderProfile {
    /// RS256 OAuth access tokens for humans.
    JwtRs256Human,
    /// Active OAuth introspection for services.
    OAuthIntrospectionService,
    /// RS256 OpenID Connect authorization-code ID tokens for humans.
    OidcRs256Human,
}
impl ProviderProfile {
    pub(crate) fn identity_profile(self) -> IdentityProfile {
        match self {
            Self::JwtRs256Human => IdentityProfile::JwtRs256Human,
            Self::OAuthIntrospectionService => IdentityProfile::OAuthIntrospectionService,
            Self::OidcRs256Human => IdentityProfile::OidcRs256Human,
        }
    }
}
impl rom::Field for ProviderProfile {
    fn shape() -> rom::Shape {
        rom::Shape::String
    }
    fn encode(&self) -> rom::Value {
        rom::json!(self.identity_profile().as_str())
    }
    fn decode(value: rom::Value) -> Result<Self> {
        [
            Self::JwtRs256Human,
            Self::OAuthIntrospectionService,
            Self::OidcRs256Human,
        ]
        .into_iter()
        .find(|profile| value.as_str() == Some(profile.identity_profile().as_str()))
        .ok_or_else(|| Error::invalid(IdentityProvider::KIND, "profile"))
    }
}
