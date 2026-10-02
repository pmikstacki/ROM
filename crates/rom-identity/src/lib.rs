//! Native identity Resources and explicit host linking of verified external identities.
//!
//! Register these ordinary Resources with explicit row/field policies. Install
//! [`IdentityGate`] on the runtime, read a [`ProviderActivation`] through an
//! authorized host identity, and use that activation's configuration when creating
//! the credential verifier. Binding is not provider verification or a login endpoint.
#![doc = include_str!("../README.md")]

use rom::{
    Actor, ActorGate, AuthorizationRead, Error, Key, PrincipalKind, Resource, Result, Runtime,
};
use rom_auth::VerifiedIdentity;
use serde::{Deserialize, Serialize};

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

/// An explicit link, stored under [`link_key`] using the same Resource pipeline.
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

fn kind_name(kind: PrincipalKind) -> &'static str {
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
fn load<R: Resource>(storage: &mut dyn AuthorizationRead, id: &str) -> Result<(u64, R)> {
    let row = storage
        .load(&Key {
            kind: R::KIND.into(),
            id: id.into(),
        })?
        .ok_or(Error::Denied)?;
    let value = row.value.ok_or(Error::Denied)?;
    Ok((row.revision, R::decode(value).map_err(|_| Error::Denied)?))
}
fn profile(provider: &IdentityProvider, kind: PrincipalKind) -> bool {
    provider.enabled
        && !provider.issuer.is_empty()
        && !provider.audience.is_empty()
        && matches!(
            (provider.profile, kind),
            (ProviderProfile::JwtRs256Human, PrincipalKind::Human)
                | (
                    ProviderProfile::OAuthIntrospectionService,
                    PrincipalKind::Service
                )
        )
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stamp {
    provider_revision: u64,
    link_revision: u64,
    user_id: String,
    user_revision: u64,
}
/// Selected local User identifier for a stamped actor. This is metadata, not
/// verification: only use it in policies after the installed gate has checked
/// the actor. Request actor metadata must never be accepted as a trusted Actor.
pub fn linked_user_id(actor: &Actor) -> Option<String> {
    serde_json::from_str::<Stamp>(actor.host_stamp()?)
        .ok()
        .map(|s| s.user_id)
}

/// Current-state identity gate. No trust namespaces are enabled by default.
#[derive(Default)]
pub struct IdentityGate {
    hosts: Vec<(String, PrincipalKind, String)>,
}
impl IdentityGate {
    /// Explicitly allow one non-human embedded/service identity, for host bootstrap
    /// or local workers. Never supply these values from request actor metadata.
    pub fn allow_host(
        mut self,
        authority: &str,
        kind: PrincipalKind,
        subject: &str,
    ) -> Result<Self> {
        if kind == PrincipalKind::Human || authority.is_empty() || subject.is_empty() {
            return Err(Error::Denied);
        }
        self.hosts.push((authority.into(), kind, subject.into()));
        Ok(self)
    }
}
impl ActorGate for IdentityGate {
    fn check(&self, actor: &Actor, storage: &mut dyn AuthorizationRead) -> Result<()> {
        if actor.host_stamp().is_none() {
            return if self.hosts.iter().any(|(a, k, s)| {
                a == &actor.authority && *k == actor.principal_kind() && s == &actor.subject
            }) {
                Ok(())
            } else {
                Err(Error::Denied)
            };
        }
        if actor.valid_until().is_none() {
            return Err(Error::Denied);
        }
        let stamp: Stamp = serde_json::from_str(actor.host_stamp().ok_or(Error::Denied)?)
            .map_err(|_| Error::Denied)?;
        let (pr, provider) = load::<IdentityProvider>(storage, &actor.authority)?;
        if pr != stamp.provider_revision || !profile(&provider, actor.principal_kind()) {
            return Err(Error::Denied);
        }
        let (lr, link) = load::<IdentityLink>(
            storage,
            &link_key(&actor.authority, actor.principal_kind(), &actor.subject),
        )?;
        if lr != stamp.link_revision
            || !link.enabled
            || link.authority != actor.authority
            || link.subject != actor.subject
            || link.principal_kind != kind_name(actor.principal_kind())
            || link.user_id != stamp.user_id
        {
            return Err(Error::Denied);
        }
        let (ur, user) = load::<User>(storage, &stamp.user_id)?;
        if ur != stamp.user_revision || !user.enabled {
            return Err(Error::Denied);
        }
        Ok(())
    }
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
        let kind = core_kind(&proof);
        if proof.authority() != self.authority || !profile(&self.config, kind) {
            return Err(Error::Denied);
        }
        Ok(ActivatedIdentity {
            authority: self.authority.clone(),
            revision: self.revision,
            proof,
        })
    }
}
fn core_kind(proof: &VerifiedIdentity) -> PrincipalKind {
    match proof.principal_kind() {
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
        let kind = core_kind(proof);
        let authority = self.authority.clone();
        let revision = self.revision;
        let subject = proof.subject().to_owned();
        let expiry = proof.valid_until();
        runtime
            .establish_actor(move |storage| {
                let (pr, provider) = load::<IdentityProvider>(storage, &authority)?;
                if pr != revision || !profile(&provider, kind) {
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
