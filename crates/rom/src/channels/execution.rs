//! Supervised notification delivery under current source authority.
use super::RegisteredChannel;
use crate::*;
use crate::{DiagnosticOutcome as Outcome, DiagnosticStage as Stage};
use crate::{diagnostics::OperationRecord, execution::diagnostic_record};
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
        let mut recording = self.0.diagnostics.as_ref().map(|sink| sink.work(&claim));
        diagnostic_record::stage(&mut recording, Stage::Work, Outcome::Started, 0);
        let result = self.process_notification_recorded(&claim, &mut recording);
        if let Err(error) = &result {
            diagnostic_record::stage(
                &mut recording,
                Stage::Work,
                diagnostic_record::outcome(error),
                0,
            );
        }
        // The guarded inner call has returned before the nonblocking publication.
        diagnostic_record::publish(recording);
        result
    }
    fn process_notification_recorded(
        &self,
        claim: &WorkClaim,
        recording: &mut Option<OperationRecord>,
    ) -> Result<()> {
        let pending = &claim.work.pending;
        let Ok(def) = self.notification_definition(pending) else {
            return self.finish_claim_recorded(
                claim,
                WorkOutcome::Stop(StopReason::DefinitionChanged),
                recording,
            );
        };
        if claim.resolution_only {
            return self.finish_claim_recorded(
                claim,
                WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                recording,
            );
        }
        let WorkPayload::Notification { payload, .. } = &pending.payload else {
            return Err(Error::Storage);
        };
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Started, 0);
        if (def.validate)(payload).is_err() {
            diagnostic_record::stage(recording, Stage::Validation, Outcome::Invalid, 0);
            return self.finish_claim_recorded(
                claim,
                WorkOutcome::Stop(StopReason::Invalid),
                recording,
            );
        }
        diagnostic_record::stage(recording, Stage::Validation, Outcome::Succeeded, 0);
        let authority = (|| {
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            diagnostic_record::stage(recording, Stage::Authorization, Outcome::Started, 0);
            if let Err(error) = self.authorize_notification(&def, pending) {
                diagnostic_record::stage(
                    recording,
                    Stage::Authorization,
                    diagnostic_record::outcome(&error),
                    0,
                );
                return Err(error);
            }
            diagnostic_record::stage(recording, Stage::Authorization, Outcome::Succeeded, 0);
            diagnostic_record::stage(recording, Stage::DeliveryStart, Outcome::Started, 0);
            self.0
                .storage
                .reaction_update(WorkUpdate::DeliveryStarted {
                    claim: claim.key(),
                    now: self.0.clock.now(),
                })
                .inspect_err(|error| {
                    diagnostic_record::stage(
                        recording,
                        Stage::DeliveryStart,
                        diagnostic_record::outcome(error),
                        0,
                    );
                })?;
            diagnostic_record::stage(recording, Stage::DeliveryStart, Outcome::Succeeded, 0);
            Ok(())
        })();
        if let Err(error) = authority {
            return match error {
                Error::Denied => self.finish_claim_recorded(
                    claim,
                    WorkOutcome::Stop(StopReason::Denied),
                    recording,
                ),
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
        diagnostic_record::stage(recording, Stage::ExternalAttempt, Outcome::Started, 0);
        let started = recording.as_ref().and_then(OperationRecord::timer);
        let outcome = match super::supervision::run(timeout, move || send(delivery)) {
            super::supervision::Callback::Returned(outcome) => outcome,
            super::supervision::Callback::Panicked => DeliveryOutcome::Panicked,
            super::supervision::Callback::TimedOut => DeliveryOutcome::TimedOut,
        };
        let observed = delivery_observation(&outcome);
        if let Some(recording) = recording {
            recording.stage_since(Stage::ExternalAttempt, observed, started, 0);
        }
        diagnostic_record::stage(recording, Stage::DeliveryFinish, Outcome::Started, 0);
        match self
            .0
            .storage
            .reaction_update(WorkUpdate::DeliveryFinished {
                claim: claim.key(),
                now: self.0.clock.now(),
                outcome,
            }) {
            Ok(_) => {
                diagnostic_record::stage(recording, Stage::DeliveryFinish, Outcome::Succeeded, 0);
                diagnostic_record::stage(recording, Stage::Work, observed, 0);
                Ok(())
            }
            Err(Error::Conflict) => {
                diagnostic_record::stage(recording, Stage::DeliveryFinish, Outcome::Conflict, 0);
                diagnostic_record::stage(recording, Stage::Work, Outcome::Conflict, 0);
                Ok(())
            }
            Err(error) => {
                diagnostic_record::stage(
                    recording,
                    Stage::DeliveryFinish,
                    diagnostic_record::outcome(&error),
                    0,
                );
                Err(error)
            }
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

fn delivery_observation(outcome: &DeliveryOutcome) -> Outcome {
    match outcome {
        DeliveryOutcome::Accepted => Outcome::Succeeded,
        DeliveryOutcome::Retryable => Outcome::Retryable,
        DeliveryOutcome::Permanent => Outcome::Permanent,
        DeliveryOutcome::Unknown => Outcome::Unknown,
        DeliveryOutcome::TimedOut => Outcome::TimedOut,
        DeliveryOutcome::Panicked => Outcome::Panicked,
    }
}
