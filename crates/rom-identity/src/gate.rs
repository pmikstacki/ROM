use crate::binding::{Stamp, kind_name, link_key, load, profile};
use crate::{IdentityLink, IdentityProvider, User};
use rom::{Actor, ActorGate, AuthorizationRead, Error, PrincipalKind, Result};

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
