//! Current-authority claim routing, action recovery and mapper materialization.
use crate::*;
impl Runtime {
    pub(super) fn process_claim(&self, claim: WorkClaim) -> Result<()> {
        if matches!(claim.work.pending.payload, WorkPayload::Notification { .. }) {
            return self.process_notification(claim);
        }
        let pending = &claim.work.pending;
        let Some(def) = self
            .0
            .reactions
            .get(&pending.definition)
            .filter(|d| d.version == pending.version && d.actor.key() == pending.service_key)
        else {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::DefinitionChanged));
        };
        let result = (|| {
            self.check_authority(&def.actor)?;
            if let WorkPayload::Action(value) = &pending.payload {
                let invocation: Invocation =
                    serde_json::from_value(value.clone()).map_err(|_| Error::Storage)?;
                invocation.check_size(&def.actor, self.0.limits.command_bytes)?;
                let identity = invocation.durable_identity(&def.actor);
                if invocation.retry_epoch != pending.cause.retry_epoch {
                    return Err(Error::Storage);
                }
                let replay_exists = {
                    let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
                    self.resolve_frozen_action(&def.actor, &invocation)?
                };
                if replay_exists {
                    return self.finish_claim(&claim, WorkOutcome::Done);
                }
                if claim.resolution_only {
                    return self.finish_claim(
                        &claim,
                        WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                    );
                }
                self.run(
                    &def.actor,
                    invocation.into_command(),
                    identity,
                    Some((pending.cause.clone(), claim.key())),
                )?;
                // The native target commit also marks this claim Done. Replayed receipts take the path above.
                return Ok(());
            }
            if claim.resolution_only {
                return self.finish_claim(
                    &claim,
                    WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                );
            }
            let WorkPayload::Source(source) = &pending.payload else {
                unreachable!()
            };
            if source.value.is_none() {
                return Err(Error::Denied);
            }
            {
                let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
                self.check_authority(&def.actor)?;
                let current = self.0.storage.load(&source.key)?;
                self.require_complete(&def.actor, current.as_ref(), source)?;
            }
            let mapper = def.mapper.clone();
            let source = source.clone();
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            self.0.pool.spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| mapper(source)))
                    .unwrap_or(Err(Error::Panicked));
                let _ = send.send(result);
            });
            let targets = receive.recv().map_err(|_| Error::Panicked)??;
            if targets.len() > self.0.reaction_limits.max_fanout {
                return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Fanout));
            }
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.check_authority(&def.actor)?;
            let WorkPayload::Source(source) = &pending.payload else {
                unreachable!()
            };
            let current = self.0.storage.load(&source.key)?;
            self.require_complete(&def.actor, current.as_ref(), source)?;
            let mut children = vec![];
            for (index, (id, input)) in targets.into_iter().enumerate() {
                if id.is_empty() {
                    return Err(Error::invalid(&def.target, "reaction target"));
                }
                let target = self
                    .0
                    .storage
                    .load(&Key {
                        kind: def.target.clone(),
                        id: id.clone(),
                    })?
                    .ok_or(Error::Missing)?;
                let mut cause = pending.cause.clone();
                cause.parent = Some(pending.id.clone());
                cause.path.push(index.to_string());
                let work_id = json!([cause.root, cause.path]).to_string();
                let invocation = Invocation {
                    retry_epoch: cause.retry_epoch,
                    kind: def.target.clone(),
                    id,
                    expected: Some(target.revision),
                    idempotency: work_id.clone(),
                    operation: Operation::Action {
                        name: def.action.clone(),
                        input,
                    },
                };
                invocation.check_size(&def.actor, self.0.limits.command_bytes)?;
                children.push(PendingWork {
                    id: work_id,
                    cause,
                    definition: def.name.clone(),
                    version: def.version,
                    delivery_profile: DeliveryProfile::AtLeastOnce,
                    service_key: def.actor.key(),
                    payload: WorkPayload::Action(
                        serde_json::to_value(invocation).map_err(|_| Error::Storage)?,
                    ),
                });
            }
            self.0.storage.reaction_update(WorkUpdate::Materialize {
                claim: claim.key(),
                now: self.0.clock.now(),
                children,
            })?;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(Error::Unknown) => Ok(()),
            Err(e) => {
                let reason = match e {
                    Error::Denied => Some(StopReason::Denied),
                    Error::Conflict => Some(StopReason::Conflict),
                    Error::Missing => Some(StopReason::Missing),
                    Error::Invalid { .. }
                    | Error::TooLarge
                    | Error::IdentityMismatch
                    | Error::IdentityExpired => Some(StopReason::Invalid),
                    Error::Unregistered | Error::Unsupported(_) => {
                        Some(StopReason::DefinitionChanged)
                    }
                    Error::Panicked => Some(StopReason::CallbackPanicked),
                    _ => None,
                };
                self.finish_claim(&claim, reason.map_or(WorkOutcome::Retry, WorkOutcome::Stop))
            }
        }
    }
    pub(crate) fn finish_claim(&self, claim: &WorkClaim, outcome: WorkOutcome) -> Result<()> {
        match self.0.storage.reaction_update(WorkUpdate::Finish {
            claim: claim.key(),
            now: self.0.clock.now(),
            outcome,
        }) {
            Ok(_) | Err(Error::Conflict) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
