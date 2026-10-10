//! Current authority checks and disclosure gates shared by reads and writes.
use super::Runtime;
use crate::{
    Access, Actor, AuthorizationRead, Descriptor, Error, Registered, Resource, Result, Row, Value,
    policy,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, atomic::Ordering},
};
impl Runtime {
    /// Cheap observer lifecycle, expiry and local-revocation check for transport
    /// keepalives. This does not evaluate the authoritative actor gate or Resource
    /// policies; only the ordinary observation APIs authorize data delivery.
    pub fn observation_status(&self, actor: &Actor) -> Result<()> {
        self.ensure_open()?;
        self.check_actor(actor)
    }
    pub(crate) fn check_actor(&self, actor: &Actor) -> Result<()> {
        let now = match catch_unwind(AssertUnwindSafe(|| self.0.clock.now())) {
            Ok(now) => now,
            Err(_) => {
                self.fail_terminal();
                return Err(Error::Panicked);
            }
        };
        if actor.valid_until().is_some_and(|end| now >= end)
            || actor
                .source
                .as_ref()
                .is_some_and(|permit| now >= permit.valid_until)
        {
            return Err(Error::Denied);
        }
        if self
            .0
            .denied
            .lock()
            .map_err(|_| Error::Panicked)?
            .contains(&actor.key())
        {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
    /// Only call from bounded I/O work, holding the commit gate when consistency
    /// with other managed Resource mutations matters.
    pub(crate) fn check_authority(&self, actor: &Actor) -> Result<()> {
        self.check_actor(actor)?;
        if let Some(permit) = &actor.source {
            let current = policy::GateRead {
                storage: self.0.storage.as_ref(),
                reads: 1,
                bytes: self.0.limits.command_bytes,
            }
            .load(&permit.condition.key)?;
            if current
                .as_ref()
                .is_none_or(|row| row.revision != permit.condition.revision || row.value.is_none())
            {
                return Err(Error::Conflict);
            }
        }
        if let Some(gate) = &self.0.actor_gate {
            gate.check(
                actor,
                &mut policy::GateRead {
                    storage: self.0.storage.as_ref(),
                    reads: 8,
                    bytes: self.0.limits.command_bytes,
                },
            )?;
        }
        Ok(())
    }
    pub fn revoke(&self, actor: &Actor) {
        self.0.denied.lock().unwrap().insert(actor.key());
        self.invalidate();
    }
    /// Trusted host metadata inspection; a transport must not equate this with public discovery.
    pub fn descriptor<R: Resource>(&self, actor: &Actor) -> Result<Descriptor> {
        self.check_actor(actor)?;
        self.0
            .registry
            .get(R::KIND)
            .map(|d| d.descriptor())
            .ok_or(Error::Unregistered)
    }
    /// Trusted host identity integration, not a transport endpoint. Resolves a
    /// candidate through bounded read-only I/O, then applies current actor checks.
    /// The callback may be retried after a concurrent managed Resource mutation.
    pub async fn establish_actor<F>(&self, resolve: F) -> Result<Actor>
    where
        F: Fn(&mut dyn AuthorizationRead) -> Result<Actor> + Send + Sync + 'static,
    {
        let resolve = Arc::new(resolve);
        for _ in 0..8 {
            let resolve = resolve.clone();
            let (actor, generation) = self
                .io(move |runtime| {
                    let _guard = runtime.0.gate.lock().map_err(|_| Error::Panicked)?;
                    let actor = resolve(&mut policy::GateRead {
                        storage: runtime.0.storage.as_ref(),
                        reads: 8,
                        bytes: runtime.0.limits.command_bytes,
                    })?;
                    runtime.check_authority(&actor)?;
                    Ok((actor, runtime.0.generation.load(Ordering::SeqCst)))
                })
                .await?;
            self.check_actor(&actor)?;
            self.ensure_open()?;
            #[cfg(test)]
            super::overload_generation::before_generation_compare(
                self,
                super::overload_generation::Boundary::Actor,
            )
            .await?;
            if generation == self.0.generation.load(Ordering::SeqCst) {
                return Ok(actor);
            }
        }
        self.0.core_overloads.record_actor_generation_exhausted();
        Err(Error::Overloaded)
    }
    pub(crate) fn disclose(
        &self,
        actor: &Actor,
        _guard: &(),
        def: &dyn Registered,
        current: Option<&Row>,
        outcome: &Row,
    ) -> Result<()> {
        self.check_authority(actor)?;
        if current.is_some_and(|r| r.value.is_none()) && outcome.value.is_some() {
            return Err(Error::Denied);
        }
        // A deletion still discloses an identity and revision. Require both current
        // and historical row authorization; legacy tombstones without context deny.
        for row in current.into_iter().chain(std::iter::once(outcome)) {
            let value = row.authorization_value().ok_or(Error::Denied)?;
            if !def.allows(actor, Access::Read, value) {
                return Err(Error::Denied);
            }
        }
        Ok(())
    }
    pub(crate) fn authorize_read(
        &self,
        actor: &Actor,
        def: &dyn Registered,
        row: &Row,
    ) -> Result<()> {
        self.disclose(actor, &(), def, Some(row), row)?;
        self.require_complete(actor, Some(row), row)
    }
    pub(super) fn authorize_fields(
        &self,
        actor: &Actor,
        def: &dyn Registered,
        prior: Option<&Value>,
        proposed: Option<&Value>,
        explicit: bool,
    ) -> Result<()> {
        for field in def.descriptor().fields {
            if !explicit
                && prior.and_then(|v| v.get(&field.name))
                    == proposed.and_then(|v| v.get(&field.name))
            {
                continue;
            }
            for value in [prior, proposed].into_iter().flatten() {
                if !def.allows_field(actor, Access::Write, &field.name, value) {
                    return Err(Error::Denied);
                }
            }
        }
        Ok(())
    }
}
