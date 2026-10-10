//! Opt-in observations contain categories and bounded opaque correlation only.
use crate::{AiError, AiResult, request::valid_name};
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationKind {
    Prepared,
    Completed,
    Rejected,
    Waiting,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowObservation {
    run_id: String,
    step: u32,
    attempt: u32,
    elapsed_ms: u64,
    category: ObservationKind,
}
impl FlowObservation {
    pub fn new(
        run_id: impl Into<String>,
        step: u32,
        attempt: u32,
        elapsed_ms: u64,
        category: ObservationKind,
    ) -> AiResult<Self> {
        let run_id = run_id.into();
        if !valid_name(&run_id, 64) || step > 32 || attempt > 8 {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self {
            run_id,
            step,
            attempt,
            elapsed_ms,
            category,
        })
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn step(&self) -> u32 {
        self.step
    }
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }
    pub fn category(&self) -> ObservationKind {
        self.category
    }
}
pub trait FlowObserver: Send + Sync {
    fn observe(&self, observation: &FlowObservation);
}
/// Host observers must be bounded. Panic isolation cannot preempt a blocking callback.
pub fn observe_safely(observer: Option<&dyn FlowObserver>, observation: &FlowObservation) {
    if let Some(observer) = observer {
        let _ = catch_unwind(AssertUnwindSafe(|| observer.observe(observation)));
    }
}
