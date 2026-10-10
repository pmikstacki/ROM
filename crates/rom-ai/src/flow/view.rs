//! Bounded authorized projections exclude service keys and private request payloads.
use super::{
    codec::Validated,
    resource::{OwnerIdentity, RunCounters, RunRecord, RunState},
};
use crate::{AiError, AiResult, request::valid_name};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunHandle(pub String);
impl RunHandle {
    pub fn new(id: impl Into<String>) -> AiResult<Self> {
        let value = Self(id.into());
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.0, 64) {
            return Err(AiError::InvalidRequest);
        }
        Ok(())
    }
}
impl fmt::Debug for RunHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RunHandle { .. }")
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMilestone {
    pub(crate) sequence: u32,
    pub(crate) state: RunState,
    pub(crate) observed_at_unix_ms: u64,
}
impl RunMilestone {
    pub fn sequence(&self) -> u32 {
        self.sequence
    }
    pub fn state(&self) -> &RunState {
        &self.state
    }
    pub fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RunView {
    run_id: String,
    revision: u64,
    state: RunState,
    milestones: Vec<RunMilestone>,
    counters: RunCounters,
    failure: Option<AiError>,
    cancel_requested: bool,
    output: Option<Value>,
    read_progress: Option<super::ReadProgress>,
}
impl RunView {
    /// The trusted callback must check current owner and tool grants, not cached identity alone.
    /// Durable read progress cannot certify physical activity or permission to retry.
    pub fn project<F>(
        record: &RunRecord,
        revision: u64,
        actor: &rom::Actor,
        authorize: F,
    ) -> AiResult<Self>
    where
        F: FnOnce(&rom::Actor, &OwnerIdentity) -> AiResult<()>,
    {
        authorize(actor, record.owner())?;
        record.validate()?;
        if revision == 0 {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self {
            run_id: record.run_id().into(),
            revision,
            state: record.state().clone(),
            milestones: record.milestones.clone(),
            counters: record.counters().clone(),
            failure: record.failure.clone(),
            cancel_requested: record.cancel_requested,
            output: if record.state() == &RunState::Completed && !record.cancel_requested {
                record.validated_output.clone()
            } else {
                None
            },
            read_progress: crate::tools::read_progress::durable(record)?,
        })
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn state(&self) -> &RunState {
        &self.state
    }
    pub fn milestones(&self) -> &[RunMilestone] {
        &self.milestones
    }
    pub fn counters(&self) -> &RunCounters {
        &self.counters
    }
    pub fn failure(&self) -> Option<&AiError> {
        self.failure.as_ref()
    }
    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }
    pub fn output(&self) -> Option<&Value> {
        self.output.as_ref()
    }
    pub fn read_progress(&self) -> Option<&super::ReadProgress> {
        self.read_progress.as_ref()
    }
    pub(crate) fn annotate_read_progress(
        &mut self,
        progress: Option<super::ReadProgress>,
    ) -> AiResult<()> {
        if self
            .read_progress
            .as_ref()
            .map(super::ReadProgress::ordinal)
            != progress.as_ref().map(super::ReadProgress::ordinal)
        {
            return Err(AiError::InvalidRequest);
        }
        self.read_progress = progress;
        Ok(())
    }
}
impl fmt::Debug for RunView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunView")
            .field("state", &self.state)
            .field("counters", &self.counters)
            .field("cancel_requested", &self.cancel_requested)
            .finish_non_exhaustive()
    }
}
