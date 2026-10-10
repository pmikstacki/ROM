//! Preparation failures publish no provider evidence and never change account money.
use super::{
    AiRun, RunState,
    codec::{self, Validated, input},
};
use crate::{AiError, AiResult};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailPreparation {
    now_unix_ms: u64,
    error: AiError,
}
impl FailPreparation {
    pub fn new(now_unix_ms: u64, error: AiError) -> AiResult<Self> {
        let value = Self { now_unix_ms, error };
        value.validate()?;
        Ok(value)
    }
}
impl Validated for FailPreparation {
    fn validate(&self) -> AiResult<()> {
        if self.error == AiError::Conflict
            || matches!(self.error, AiError::RateLimited { retry_after_ms }
                if !(1..=300_000).contains(&retry_after_ms))
        {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
impl fmt::Debug for FailPreparation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FailPreparation { .. }")
    }
}
input!(FailPreparation);

pub const FAIL_PREPARATION: rom::Action<AiRun, FailPreparation> =
    rom::Action::new("fail_preparation", |run, input| {
        input.validate().map_err(codec::rom_error)?;
        let mut record = run.record().map_err(codec::rom_error)?;
        if record.state() != &RunState::Queued
            || !record.queue_admitted
            || record.checkpoint() != 0
            || record.active_attempt().is_some()
            || record.counters().generation_attempts() != 0
            || record.cancel_requested()
        {
            return Err(rom::Error::Conflict);
        }
        record
            .observe_recovery(input.now_unix_ms)
            .map_err(codec::rom_error)?;
        record.failure = Some(input.error);
        record.state = RunState::Failed;
        record.mark();
        run.store(record).map_err(codec::rom_error)?;
        Ok(vec![])
    });
