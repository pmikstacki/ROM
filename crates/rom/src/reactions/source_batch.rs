//! Retained Source leases and acknowledgement-bound empty materializations.
use crate::*;
use crate::{DiagnosticOutcome as Outcome, DiagnosticStage as Stage};
use crate::{diagnostics::OperationRecord, execution::diagnostic_record};

pub(super) struct SourceOutput {
    pub(super) claim: WorkClaim,
    pub(super) recording: Option<OperationRecord>,
    pub(super) settled: bool,
}
impl SourceOutput {
    fn acknowledge(&mut self, outcome: Outcome) {
        diagnostic_record::stage(&mut self.recording, Stage::Materialize, outcome, 0);
        diagnostic_record::stage(&mut self.recording, Stage::Work, outcome, 0);
        self.settled = true;
    }
}

impl Runtime {
    pub(super) fn source_lease_live(&self, claim: &WorkClaim) -> bool {
        matches!(claim.work.state, WorkState::Leased { until, generation, .. }
            if until > self.0.clock.now() && generation == claim.work.generation)
    }
    pub(super) fn source_claim_current(
        &self,
        claim: &WorkClaim,
        legacy_singleton: bool,
    ) -> Result<bool> {
        if !self.source_lease_live(claim) {
            return Ok(false);
        }
        match self
            .0
            .storage
            .reaction_claim_live(&claim.key(), self.0.clock.now())?
        {
            Some(live) => Ok(live),
            None if legacy_singleton => Ok(true),
            None => Err(Error::Unsupported(
                "grouped claims require bounded claim reads".into(),
            )),
        }
    }

    fn validate_source_output(&self, claim: &WorkClaim) -> Result<()> {
        if !self.source_claim_current(claim, true)? {
            return Err(Error::Conflict);
        }
        let pending = &claim.work.pending;
        let def = self
            .0
            .reactions
            .get(&pending.definition)
            .filter(|d| d.version == pending.version && d.actor.key() == pending.service_key)
            .ok_or(Error::Unregistered)?;
        self.check_authority(&def.actor)?;
        let WorkPayload::Source(source) = &pending.payload else {
            return Err(Error::Storage);
        };
        let current = self.0.storage.load(&source.key)?;
        // Historical events remain valid under current authorization; no revision equality.
        self.require_complete(&def.actor, current.as_ref(), source)
    }

    // Called under the core gate. Confirmed fallback republishes obtained outputs,
    // never callbacks; each singleton rechecks current authority and lease time.
    fn publish_source_outputs(
        &self,
        outputs: &mut [SourceOutput],
        indices: &[usize],
    ) -> Result<()> {
        if indices.is_empty() {
            return Ok(());
        }
        let updates = indices
            .iter()
            .map(|index| WorkUpdate::Materialize {
                claim: outputs[*index].claim.key(),
                now: self.0.clock.now(),
                children: vec![],
            })
            .collect();
        match self.0.storage.reaction_updates_atomic(updates) {
            Ok(results) => {
                if results.len() != indices.len()
                    || results.iter().any(|result| *result != WorkResult::Changed)
                {
                    return Err(Error::Storage);
                }
                for index in indices {
                    outputs[*index].acknowledge(Outcome::Succeeded);
                }
            }
            Err(Error::Unknown) => {
                for index in indices {
                    outputs[*index].acknowledge(Outcome::Unknown);
                }
            }
            Err(Error::Unsupported(_)) | Err(Error::NotCommitted) => {
                for index in indices {
                    let output = &mut outputs[*index];
                    self.validate_source_output(&output.claim)?;
                    match self.0.storage.reaction_update(WorkUpdate::Materialize {
                        claim: output.claim.key(),
                        now: self.0.clock.now(),
                        children: vec![],
                    }) {
                        Ok(_) => output.acknowledge(Outcome::Succeeded),
                        Err(Error::Unknown) => {
                            output.acknowledge(Outcome::Unknown);
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(super) fn flush_source_outputs(&self, outputs: &mut Vec<SourceOutput>) -> Result<()> {
        if outputs.is_empty() {
            return Ok(());
        }
        let mut pending = std::mem::take(outputs);
        let guard = self.0.gate.lock().map_err(|_| Error::Panicked)?;
        let mut valid = vec![];
        let mut first_error = None;
        for index in 0..pending.len() {
            match self.validate_source_output(&pending[index].claim) {
                Ok(()) => valid.push(index),
                Err(error) => {
                    // The valid prefix acknowledges before this invalid output finishes.
                    if let Err(error) = self.publish_source_outputs(&mut pending, &valid) {
                        first_error.get_or_insert(error);
                    }
                    valid.clear();
                    let outcome = match error {
                        Error::Denied => WorkOutcome::Stop(StopReason::Denied),
                        Error::Conflict => WorkOutcome::Stop(StopReason::Conflict),
                        Error::Missing => WorkOutcome::Stop(StopReason::Missing),
                        Error::Unregistered => WorkOutcome::Stop(StopReason::DefinitionChanged),
                        Error::IdentityExpired => WorkOutcome::Stop(StopReason::Invalid),
                        _ => WorkOutcome::Retry,
                    };
                    let output = &mut pending[index];
                    if let Err(error) =
                        self.finish_claim_recorded(&output.claim, outcome, &mut output.recording)
                    {
                        first_error.get_or_insert(error);
                    }
                    output.settled = true;
                }
            }
        }
        if let Err(error) = self.publish_source_outputs(&mut pending, &valid) {
            first_error.get_or_insert(error);
        }
        for output in &mut pending {
            if !output.settled {
                // Acknowledged successes and ambiguities retain their own outcomes.
                // Unsubmitted outputs following a singleton ambiguity are retryable.
                output.acknowledge(
                    first_error
                        .as_ref()
                        .map_or(Outcome::Retryable, diagnostic_record::outcome),
                );
            }
        }
        drop(guard);
        for output in pending {
            diagnostic_record::publish(output.recording);
        }
        first_error.map_or(Ok(()), Err)
    }

    pub(super) fn process_source_prefix(&self, max_steps: usize) -> Result<usize> {
        let mut admitted = 0;
        let mut handled = 0;
        while admitted < max_steps {
            self.ensure_open()?;
            let claims = self
                .0
                .storage
                .reaction_claim_prefix(self.0.clock.now(), (max_steps - admitted).min(32))?;
            if claims.is_empty() {
                break;
            }
            admitted += claims.len();
            let legacy_singleton = claims.len() == 1;
            let mut outputs = vec![];
            let mut singles = false;
            let mut first_error = None;
            for claim in claims {
                let current = match self.source_claim_current(&claim, legacy_singleton) {
                    Ok(current) => current,
                    Err(error) => {
                        first_error.get_or_insert(error);
                        false
                    }
                };
                if !current {
                    if let Err(error) = self.flush_source_outputs(&mut outputs) {
                        first_error.get_or_insert(error);
                    }
                    singles = true;
                    continue;
                }
                if singles
                    || claim.resolution_only
                    || !matches!(claim.work.pending.payload, WorkPayload::Source(_))
                {
                    if let Err(error) = self.flush_source_outputs(&mut outputs) {
                        first_error.get_or_insert(error);
                    }
                    if let Err(error) = self.process_claim(claim) {
                        first_error.get_or_insert(error);
                    }
                    singles = true;
                } else {
                    match self.process_claim_with_outputs(claim, &mut outputs, true) {
                        Ok(barrier) => singles = barrier,
                        Err(error) => {
                            first_error.get_or_insert(error);
                            singles = true;
                        }
                    }
                }
                handled += 1;
            }
            if let Err(error) = self.flush_source_outputs(&mut outputs) {
                first_error.get_or_insert(error);
            }
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        Ok(handled)
    }
}
