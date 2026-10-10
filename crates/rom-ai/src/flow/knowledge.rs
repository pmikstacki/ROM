//! Enrich missing knowledge for the exact held attempt without advancing the run.
use super::{
    AiRun, RunRecord, RunState,
    codec::{self, Validated, input},
};
use crate::{AiError, AiResult, AttemptEvidence, DispatchOutcome, Usage};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordAttemptEvidence {
    step: u32,
    now_unix_ms: u64,
    evidence: AttemptEvidence,
    usage: Usage,
}
impl RecordAttemptEvidence {
    pub fn new(
        step: u32,
        now_unix_ms: u64,
        evidence: AttemptEvidence,
        usage: Usage,
    ) -> AiResult<Self> {
        let value = Self {
            step,
            now_unix_ms,
            evidence,
            usage,
        };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for RecordAttemptEvidence {
    fn validate(&self) -> AiResult<()> {
        if !(1..=32).contains(&self.step) {
            return Err(AiError::InvalidRequest);
        }
        self.evidence.validate()
    }
}
input!(RecordAttemptEvidence);
impl fmt::Debug for RecordAttemptEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecordAttemptEvidence { .. }")
    }
}

fn known<T: PartialEq>(old: Option<T>, new: Option<T>) -> AiResult<Option<T>> {
    if old.is_some() && new.is_some() && old != new {
        return Err(AiError::Conflict);
    }
    Ok(old.or(new))
}
fn enrich(record: &mut RunRecord, input: &RecordAttemptEvidence) -> AiResult<bool> {
    input.validate()?;
    if record.state != RunState::AwaitingReconciliation || record.checkpoint != input.step {
        return Err(AiError::Conflict);
    }
    let attempt = record
        .active_attempt
        .as_ref()
        .ok_or(AiError::Conflict)?
        .prepared();
    DispatchOutcome::Uncertain {
        evidence: input.evidence.clone(),
        usage: input.usage.clone(),
        cause: AiError::UnknownOutcome,
    }
    .validate_for(attempt)?;
    let index = record
        .evidence_history
        .iter()
        .position(|evidence| evidence.attempt_id() == attempt.identity())
        .ok_or(AiError::Conflict)?;
    let old = &record.evidence_history[index];
    let old_usage = &record.usage_history[index];
    let evidence = AttemptEvidence::new(
        attempt.identity(),
        known(
            old.provider_request_id(),
            input.evidence.provider_request_id(),
        )?
        .map(str::to_owned),
        known(old.generation_id(), input.evidence.generation_id())?.map(str::to_owned),
    )?;
    let usage = Usage {
        input_tokens: known(old_usage.input_tokens, input.usage.input_tokens)?,
        output_tokens: known(old_usage.output_tokens, input.usage.output_tokens)?,
        reasoning_tokens: known(old_usage.reasoning_tokens, input.usage.reasoning_tokens)?,
        cost: known(old_usage.cost, input.usage.cost)?,
    };
    DispatchOutcome::Uncertain {
        evidence: evidence.clone(),
        usage: usage.clone(),
        cause: AiError::UnknownOutcome,
    }
    .validate_for(attempt)?;
    if &evidence == old && &usage == old_usage {
        return Ok(false);
    }
    record.observe_recovery(input.now_unix_ms)?;
    record.note_evidence(evidence, usage)?;
    // Preserve stage and sequence; keep the latest milestone consistent with observation time.
    record.mark();
    Ok(true)
}

pub const RECORD_ATTEMPT_EVIDENCE: rom::Action<AiRun, RecordAttemptEvidence> =
    rom::Action::new("record_attempt_evidence", |run, input| {
        let mut record = run.record().map_err(codec::rom_error)?;
        if enrich(&mut record, &input).map_err(codec::rom_error)? {
            run.store(record).map_err(codec::rom_error)?;
        }
        Ok(Vec::new())
    });

pub(crate) fn transition(after: &RunRecord, before: &RunRecord) -> AiResult<bool> {
    if before.state != RunState::AwaitingReconciliation
        || after.state != before.state
        || after.cancel_requested != before.cancel_requested
    {
        return Ok(false);
    }
    let Some(attempt) = before.active_attempt.as_ref() else {
        return Ok(false);
    };
    let Some(index) = after
        .evidence_history
        .iter()
        .position(|evidence| evidence.attempt_id() == attempt.prepared().identity())
    else {
        return Ok(false);
    };
    let Some(usage) = after.usage_history.get(index) else {
        return Ok(false);
    };
    let mut expected = before.clone();
    let input = RecordAttemptEvidence::new(
        before.checkpoint,
        after.last_observed_unix_ms,
        after.evidence_history[index].clone(),
        usage.clone(),
    )?;
    if !enrich(&mut expected, &input)? {
        return Ok(false);
    }
    Ok(&expected == after)
}
