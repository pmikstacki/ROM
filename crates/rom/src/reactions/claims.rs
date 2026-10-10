//! Current-authority claim routing, action recovery and mapper materialization.
use crate::*;
use crate::{DiagnosticOutcome as Outcome, DiagnosticStage as Stage};
use crate::{diagnostics::OperationRecord, execution::diagnostic_record};
impl Runtime {
    pub(super) fn process_claim(&self, claim: WorkClaim) -> Result<()> {
        self.process_claim_with_outputs(claim, &mut vec![], false)
            .map(|_| ())
    }
    pub(super) fn process_claim_with_outputs(
        &self,
        claim: WorkClaim,
        outputs: &mut Vec<super::source_batch::SourceOutput>,
        defer_empty: bool,
    ) -> Result<bool> {
        if matches!(claim.work.pending.payload, WorkPayload::Notification { .. }) {
            return self.process_notification(claim).map(|_| true);
        }
        let mut recording = self.0.diagnostics.as_ref().map(|sink| sink.work(&claim));
        diagnostic_record::stage(&mut recording, Stage::Work, Outcome::Started, 0);
        let before = outputs.len();
        let result = self.process_claim_recorded(&claim, &mut recording, outputs, defer_empty);
        let recording_deferred = outputs.len() > before;
        if let Err(error) = &result {
            diagnostic_record::stage(
                &mut recording,
                Stage::Work,
                diagnostic_record::outcome(error),
                0,
            );
        }
        // The inner helper has released every core/native guard before publication.
        diagnostic_record::publish(recording);
        result.map(|_| !recording_deferred)
    }
    fn process_claim_recorded(
        &self,
        claim: &WorkClaim,
        recording: &mut Option<OperationRecord>,
        outputs: &mut Vec<super::source_batch::SourceOutput>,
        defer_empty: bool,
    ) -> Result<()> {
        let pending = &claim.work.pending;
        let Some(def) = self
            .0
            .reactions
            .get(&pending.definition)
            .filter(|d| d.version == pending.version && d.actor.key() == pending.service_key)
        else {
            self.flush_source_outputs(outputs)?;
            return self.finish_claim_recorded(
                claim,
                WorkOutcome::Stop(StopReason::DefinitionChanged),
                recording,
            );
        };
        let result = (|| {
            diagnostic_record::stage(recording, Stage::Authorization, Outcome::Started, 0);
            self.check_authority(&def.actor).inspect_err(|error| {
                diagnostic_record::stage(
                    recording,
                    Stage::Authorization,
                    diagnostic_record::outcome(error),
                    0,
                );
            })?;
            diagnostic_record::stage(recording, Stage::Authorization, Outcome::Succeeded, 0);
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
                    let mut receipt_recording = self.0.diagnostics.as_ref().map(|sink| {
                        sink.operation(
                            &identity,
                            Some(&pending.cause),
                            Some(&claim.key()),
                            claim.work.attempts,
                        )
                    });
                    diagnostic_record::stage(
                        &mut receipt_recording,
                        Stage::Receipt,
                        Outcome::Replay,
                        0,
                    );
                    // Frozen receipt/current-authority lookup has released its gate.
                    diagnostic_record::publish(receipt_recording);
                    return self.finish_claim_recorded(claim, WorkOutcome::Done, recording);
                }
                if claim.resolution_only {
                    return self.finish_claim_recorded(
                        claim,
                        WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                        recording,
                    );
                }
                let claim_key = claim.key();
                let mut child_recording = self.0.diagnostics.as_ref().map(|sink| {
                    sink.operation(
                        &identity,
                        Some(&pending.cause),
                        Some(&claim_key),
                        claim.work.attempts,
                    )
                });
                diagnostic_record::stage(&mut child_recording, Stage::Work, Outcome::Started, 0);
                self.run_observed(
                    &def.actor,
                    invocation.into_command(),
                    identity,
                    Some((pending.cause.clone(), claim_key)),
                    child_recording,
                )?;
                diagnostic_record::stage(recording, Stage::Work, Outcome::Succeeded, 0);
                // The native target commit also marks this claim Done. Replayed receipts take the path above.
                return Ok(());
            }
            if claim.resolution_only {
                return self.finish_claim_recorded(
                    claim,
                    WorkOutcome::Stop(claim.stop_reason.clone().ok_or(Error::Storage)?),
                    recording,
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
                if !self.source_claim_current(claim, true)? {
                    return Err(Error::Conflict);
                }
                self.check_authority(&def.actor)?;
                let current = self.0.storage.load(&source.key)?;
                self.require_complete(&def.actor, current.as_ref(), source)
                    .inspect_err(|error| {
                        diagnostic_record::stage(
                            recording,
                            Stage::Authorization,
                            diagnostic_record::outcome(error),
                            0,
                        );
                    })?;
            }
            let mapper = def.mapper.clone();
            let source = source.clone();
            diagnostic_record::stage(recording, Stage::Reaction, Outcome::Started, 0);
            let started = recording.as_ref().and_then(OperationRecord::timer);
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            self.0.pool.spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| mapper(source)))
                    .unwrap_or(Err(Error::Panicked));
                let _ = send.send(result);
            });
            let mapped = receive
                .recv()
                .map_err(|_| Error::Panicked)
                .and_then(|result| result);
            let targets = match mapped {
                Ok(targets) => {
                    if let Some(recording) = recording {
                        recording.stage_since(Stage::Reaction, Outcome::Succeeded, started, 0);
                    }
                    targets
                }
                Err(error) => {
                    if let Some(recording) = recording {
                        recording.stage_since(
                            Stage::Reaction,
                            diagnostic_record::outcome(&error),
                            started,
                            0,
                        );
                    }
                    return Err(error);
                }
            };
            if targets.len() > self.0.reaction_limits.max_fanout {
                self.flush_source_outputs(outputs)?;
                return self.finish_claim_recorded(
                    claim,
                    WorkOutcome::Stop(StopReason::Fanout),
                    recording,
                );
            }
            if !targets.is_empty() {
                self.flush_source_outputs(outputs)?;
            }
            let _guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
            if !self.source_claim_current(claim, true)? {
                return Err(Error::Conflict);
            }
            self.check_authority(&def.actor)?;
            let WorkPayload::Source(source) = &pending.payload else {
                unreachable!()
            };
            let current = self.0.storage.load(&source.key)?;
            self.require_complete(&def.actor, current.as_ref(), source)
                .inspect_err(|error| {
                    diagnostic_record::stage(
                        recording,
                        Stage::Authorization,
                        diagnostic_record::outcome(error),
                        0,
                    );
                })?;
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
                    not_before: None,
                    delivery_profile: DeliveryProfile::AtLeastOnce,
                    service_key: def.actor.key(),
                    payload: WorkPayload::Action(
                        serde_json::to_value(invocation).map_err(|_| Error::Storage)?,
                    ),
                });
            }
            diagnostic_record::stage(recording, Stage::Materialize, Outcome::Started, 0);
            if defer_empty && children.is_empty() {
                outputs.push(super::source_batch::SourceOutput {
                    claim: claim.clone(),
                    recording: recording.take(),
                    settled: false,
                });
                return Ok(());
            }
            self.0
                .storage
                .reaction_update(WorkUpdate::Materialize {
                    claim: claim.key(),
                    now: self.0.clock.now(),
                    children,
                })
                .inspect_err(|error| {
                    diagnostic_record::stage(
                        recording,
                        Stage::Materialize,
                        diagnostic_record::outcome(error),
                        0,
                    );
                })?;
            diagnostic_record::stage(recording, Stage::Materialize, Outcome::Succeeded, 0);
            diagnostic_record::stage(recording, Stage::Work, Outcome::Succeeded, 0);
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(Error::Unknown) => {
                self.flush_source_outputs(outputs)?;
                diagnostic_record::stage(recording, Stage::Work, Outcome::Unknown, 0);
                Ok(())
            }
            Err(e) => {
                self.flush_source_outputs(outputs)?;
                diagnostic_record::stage(recording, Stage::Work, diagnostic_record::outcome(&e), 0);
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
                self.finish_claim_recorded(
                    claim,
                    reason.map_or(WorkOutcome::Retry, WorkOutcome::Stop),
                    recording,
                )
            }
        }
    }
    pub(crate) fn finish_claim_recorded(
        &self,
        claim: &WorkClaim,
        outcome: WorkOutcome,
        recording: &mut Option<OperationRecord>,
    ) -> Result<()> {
        let observed = match &outcome {
            WorkOutcome::Done => Outcome::Succeeded,
            WorkOutcome::Retry => Outcome::Retryable,
            WorkOutcome::Stop(_) => Outcome::Stopped,
        };
        match self.0.storage.reaction_update(WorkUpdate::Finish {
            claim: claim.key(),
            now: self.0.clock.now(),
            outcome,
        }) {
            Ok(_) => {
                diagnostic_record::stage(recording, Stage::Work, observed, 0);
                Ok(())
            }
            Err(Error::Conflict) => {
                diagnostic_record::stage(recording, Stage::Work, Outcome::Conflict, 0);
                Ok(())
            }
            Err(e) => {
                diagnostic_record::stage(recording, Stage::Work, diagnostic_record::outcome(&e), 0);
                Err(e)
            }
        }
    }
}
