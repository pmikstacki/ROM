use crate::{IdentityLink, IdentityProvider, User};
use rom::{Actor, AuthorizationRead, Error, Key, PrincipalKind, Resource, Result, Runtime};
use rom_auth::VerifiedIdentity;
use serde::{Deserialize, Serialize};

pub(super) fn kind_name(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::Embedded => "embedded",
        PrincipalKind::Human => "human",
        PrincipalKind::Service => "service",
    }
}
/// Collision-free tuple encoding; every subject belongs to one provider and kind.
pub fn link_key(authority: &str, kind: PrincipalKind, subject: &str) -> String {
    serde_json::json!([authority, kind_name(kind), subject]).to_string()
}
pub(super) fn load<R: Resource>(storage: &mut dyn AuthorizationRead, id: &str) -> Result<(u64, R)> {
    let row = storage
        .load(&Key {
            kind: R::KIND.into(),
            id: id.into(),
        })?
        .ok_or(Error::Denied)?;
    let value = row.value.ok_or(Error::Denied)?;
    Ok((row.revision, R::decode(value).map_err(|_| Error::Denied)?))
}
pub(super) fn profile(
    provider: &IdentityProvider,
    kind: PrincipalKind,
    verified_profile: &str,
) -> bool {
    provider.enabled
        && !provider.issuer.is_empty()
        && !provider.audience.is_empty()
        && provider.profile.identity_profile().as_str() == verified_profile
        && core_kind(provider.profile.identity_profile().principal_kind()) == kind
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stamp {
    pub(super) profile: String,
    pub(super) provider_revision: u64,
    pub(super) link_revision: u64,
    pub(super) user_id: String,
    pub(super) user_revision: u64,
}
/// Selected local User identifier for a stamped actor. This is metadata, not
/// verification: only use it in policies after the installed gate has checked
/// the actor. Request actor metadata must never be accepted as a trusted Actor.
pub fn linked_user_id(actor: &Actor) -> Option<String> {
    serde_json::from_str::<Stamp>(actor.host_stamp()?)
        .ok()
        .map(|s| s.user_id)
}

/// A provider configuration revision read before constructing its verifier.
/// An activation cannot be constructed by deserializing request input.
#[derive(Clone)]
pub struct ProviderActivation {
    authority: String,
    revision: u64,
    config: IdentityProvider,
}
impl ProviderActivation {
    /// Read through ordinary current Resource authorization using an explicit host actor.
    pub async fn read(runtime: &Runtime, host: &Actor, authority: &str) -> Result<Self> {
        let row = runtime.read::<IdentityProvider>(host, authority).await?;
        let config = row.value.ok_or(Error::Denied)?;
        if !config.enabled {
            return Err(Error::Denied);
        }
        Ok(Self {
            authority: authority.into(),
            revision: row.revision,
            config,
        })
    }
    /// Configure the verifier from these exact values before obtaining its proof.
    pub fn config(&self) -> &IdentityProvider {
        &self.config
    }
    /// Host-configured verifier authority namespace.
    pub fn authority(&self) -> &str {
        &self.authority
    }
    /// Captured configuration and activation revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Verify using this activation's configuration and capture its revision.
    /// This is a trusted host callback: it must freshly verify credentials against
    /// the provided configuration, never return a cached proof from another activation.
    pub fn verify<F>(&self, verify: F) -> Result<ActivatedIdentity>
    where
        F: FnOnce(
            &str,
            &IdentityProvider,
        ) -> std::result::Result<VerifiedIdentity, rom_auth::AuthError>,
    {
        let proof = verify(&self.authority, &self.config).map_err(|_| Error::Denied)?;
        let kind = core_kind(proof.principal_kind());
        if proof.authority() != self.authority
            || !profile(&self.config, kind, proof.profile().as_str())
        {
            return Err(Error::Denied);
        }
        Ok(ActivatedIdentity {
            authority: self.authority.clone(),
            revision: self.revision,
            proof,
        })
    }
}
fn core_kind(kind: rom_auth::PrincipalKind) -> PrincipalKind {
    match kind {
        rom_auth::PrincipalKind::Human => PrincipalKind::Human,
        rom_auth::PrincipalKind::Service => PrincipalKind::Service,
    }
}
/// Immutable evidence bound to the provider configuration used during verification.
/// No public constructor or Deserialize implementation; no bearer is retained.
///
/// ```compile_fail
/// let forged = rom_identity::ActivatedIdentity {
///     authority: "admin".into(), revision: 1, proof: unimplemented!()
/// };
/// ```
#[derive(Clone)]
pub struct ActivatedIdentity {
    authority: String,
    revision: u64,
    proof: VerifiedIdentity,
}
impl ActivatedIdentity {
    /// Resolve the current explicit link and User, preserving this evidence's
    /// provider revision, principal kind and exclusive expiry.
    pub async fn bind(&self, runtime: &Runtime) -> Result<Actor> {
        let proof = &self.proof;
        let kind = core_kind(proof.principal_kind());
        let authority = self.authority.clone();
        let revision = self.revision;
        let subject = proof.subject().to_owned();
        let expiry = proof.valid_until();
        let verified_profile = proof.profile().as_str();
        runtime
            .establish_actor(move |storage| {
                let (pr, provider) = load::<IdentityProvider>(storage, &authority)?;
                if pr != revision || !profile(&provider, kind, verified_profile) {
                    return Err(Error::Denied);
                }
                let (lr, link) =
                    load::<IdentityLink>(storage, &link_key(&authority, kind, &subject))?;
                if !link.enabled
                    || link.authority != authority
                    || link.subject != subject
                    || link.principal_kind != kind_name(kind)
                    || link.user_id.is_empty()
                {
                    return Err(Error::Denied);
                }
                let (ur, user) = load::<User>(storage, &link.user_id)?;
                if !user.enabled {
                    return Err(Error::Denied);
                }
                let stamp = serde_json::to_string(&Stamp {
                    profile: verified_profile.into(),
                    provider_revision: pr,
                    link_revision: lr,
                    user_id: link.user_id,
                    user_revision: ur,
                })
                .map_err(|_| Error::Denied)?;
                Ok(Actor::trusted(&authority, &subject)
                    .with_kind(kind)
                    .expires_at(expiry)
                    .with_host_stamp(&stamp))
            })
            .await
    }
}
