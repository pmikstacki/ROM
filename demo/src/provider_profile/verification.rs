//! Current activation checks and blocking verifier construction/use/destruction.
use super::{ApprovedProvider, SecretFiles};
use rom::{Clock, Error, Result};
use rom_auth::introspection::{EndpointPolicy, IntrospectionAdapter};
use rom_identity::{ActivatedIdentity, ProviderActivation, ProviderProfile};
use std::sync::Arc;

pub(super) fn verify(
    activation: ProviderActivation,
    provider: ApprovedProvider,
    secrets: SecretFiles,
    policy: EndpointPolicy,
    clock: Arc<dyn Clock>,
    token: String,
) -> Result<ActivatedIdentity> {
    let config = activation.config();
    if activation.authority() != provider.authority
        || config.profile != ProviderProfile::OAuthIntrospectionService
        || config.issuer != provider.issuer
        || config.audience != provider.audience
        || config.endpoint.as_deref() != Some(&provider.endpoint)
    {
        return Err(Error::Denied);
    }
    let secret = secrets.read(config.credential_ref.as_deref().ok_or(Error::Denied)?)?;
    activation.verify(|authority, config| {
        let mut verifier = IntrospectionAdapter::configured(
            authority,
            &config.issuer,
            &config.audience,
            &provider.endpoint,
            &provider.introspection_client,
            &secret,
            policy,
        )?;
        verifier.authenticate(&token, clock.now())
    })
}
