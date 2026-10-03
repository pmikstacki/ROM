//! Exact frozen action receipt resolution shared by workers and operator recovery.
use crate::*;
impl Runtime {
    /// Call while holding the runtime gate, outside native storage guards.
    pub(crate) fn resolve_frozen_action(
        &self,
        actor: &Actor,
        invocation: &Invocation,
    ) -> Result<bool> {
        self.check_authority(actor)?;
        invocation.check_size(actor, self.0.limits.command_bytes)?;
        let identity = invocation.durable_identity(actor);
        let Some(receipt) = self.retry_receipt(&identity, invocation.retry_epoch, true)? else {
            return Ok(false);
        };
        let key = Key {
            kind: invocation.kind.clone(),
            id: invocation.id.clone(),
        };
        if receipt.identity != identity {
            return Err(Error::Storage);
        }
        if receipt.row.key != key {
            return Err(Error::IdentityMismatch);
        }
        let definition = self.0.registry.get(&key.kind).ok_or(Error::Unregistered)?;
        let current = self.0.storage.load(&key)?;
        if current.as_ref().is_some_and(|row| row.key != key) {
            return Err(Error::Storage);
        }
        let row = self.replay_outcome(
            actor,
            definition.as_ref(),
            current.as_ref(),
            receipt,
            &invocation.clone().into_command(),
        )?;
        self.require_complete(actor, current.as_ref(), &row)?;
        Ok(true)
    }
}
