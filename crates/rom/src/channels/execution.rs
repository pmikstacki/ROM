//! Supervised notification delivery under current source authority.
use super::RegisteredChannel;
use crate::*;
impl Runtime {
    /// Run a finite batch of reaction and notification work under the shared supervisor.
    pub async fn process_work(&self, max_steps: usize) -> Result<usize> {
        self.process_reactions(max_steps).await
    }
    /// Start the one generic runtime-owned work loop. Shutdown drains accepted deliveries.
    pub fn start_work(&self) -> Result<ReactionWorker> {
        self.start_reactions()
    }
    pub(crate) fn process_notification(&self, claim: WorkClaim) -> Result<()> {
        let pending = &claim.work.pending;
        let Ok(def) = self.notification_definition(pending) else {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::DefinitionChanged));
        };
        if claim.resolution_only {
            return self.finish_claim(
                &claim,
                WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
            );
        }
        let WorkPayload::Notification { payload, .. } = &pending.payload else {
            return Err(Error::Storage);
        };
        if (def.validate)(payload).is_err() {
            return self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Invalid));
        }
        let authority = (|| {
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            self.authorize_notification(&def, pending)?;
            self.0
                .storage
                .reaction_update(WorkUpdate::DeliveryStarted {
                    claim: claim.key(),
                    now: self.0.clock.now(),
                })?;
            Ok(())
        })();
        if let Err(error) = authority {
            return match error {
                Error::Denied => self.finish_claim(&claim, WorkOutcome::Stop(StopReason::Denied)),
                Error::Conflict => Ok(()),
                _ => Err(error),
            };
        }
        let send = def.send.clone();
        let delivery = Delivery {
            id: pending.id.clone(),
            attempt: claim.work.attempts,
            payload: payload.clone(),
        };
        let timeout = self.0.delivery_timeout;
        let outcome = match super::supervision::run(timeout, move || send(delivery)) {
            super::supervision::Callback::Returned(outcome) => outcome,
            super::supervision::Callback::Panicked => DeliveryOutcome::Panicked,
            super::supervision::Callback::TimedOut => DeliveryOutcome::TimedOut,
        };
        match self
            .0
            .storage
            .reaction_update(WorkUpdate::DeliveryFinished {
                claim: claim.key(),
                now: self.0.clock.now(),
                outcome,
            }) {
            Ok(_) | Err(Error::Conflict) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

impl Runtime {
    pub(crate) fn notification_definition(
        &self,
        pending: &PendingWork,
    ) -> Result<Arc<RegisteredChannel>> {
        self.0
            .channels
            .get(&pending.definition)
            .filter(|definition| definition.matches(pending))
            .cloned()
            .ok_or_else(|| Error::Unsupported("frozen channel definition unavailable".into()))
    }
    /// Call under the core gate to bind source disclosure and service authority to a control/send.
    pub(crate) fn authorize_notification(
        &self,
        definition: &RegisteredChannel,
        pending: &PendingWork,
    ) -> Result<()> {
        self.check_authority(&definition.actor)?;
        let WorkPayload::Notification { source, .. } = &pending.payload else {
            return Err(Error::Storage);
        };
        if source.value.is_none() {
            return Err(Error::Denied);
        }
        let current = self.0.storage.load(&source.key)?;
        self.require_complete(&definition.actor, current.as_ref(), source)
    }
}
