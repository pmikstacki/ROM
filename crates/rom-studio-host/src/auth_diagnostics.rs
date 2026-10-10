//! Optional fixed-size authentication attribution, separate from authorization decisions.
use rom::{Error, Result};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Trusted Host call site, never derived from request headers or identity values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum AuthOperation {
    Unspecified,
    ProtectedMiddleware,
    GenericHttpResolver,
    Blob,
    Session,
    CurrentStream,
}
const OPERATIONS: usize = 6;
/// Boundary at which an authentication failure was observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum AuthStage {
    Admission,
    AcceptedWork,
    BeforeSpawn,
    SessionLookup,
    CredentialWait,
    OriginalExpiry,
    CachedBind,
    RenewDue,
    RenewAfterBind,
    KeyAcquisition,
    OriginalTokenVerification,
    FreshBind,
    FreshProofExpiry,
    CurrentBind,
    SessionRemoval,
    BlobReserveResult,
    BlobUploadResult,
    BlobUploadUnattached,
    BlobProjectionResult,
}
const STAGES: usize = 19;
/// Sanitized result class; no identity, credentials, arbitrary strings or error payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum AuthOutcome {
    Succeeded,
    Denied,
    Overloaded,
    Closed,
    Panicked,
    Other,
}
impl AuthOutcome {
    pub(crate) fn result<T>(result: &Result<T>) -> Self {
        match result {
            Ok(_) => Self::Succeeded,
            Err(Error::Denied) => Self::Denied,
            Err(Error::Overloaded) => Self::Overloaded,
            Err(Error::Closed) => Self::Closed,
            Err(Error::Panicked) => Self::Panicked,
            Err(_) => Self::Other,
        }
    }
}
/// Counts distinguish immediate capacity rejection from admitted work returning Overloaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct AuthOperationCounts {
    pub attempts: u64,
    pub admitted: u64,
    pub rejected_capacity: u64,
    pub rejected_closed: u64,
    pub before_spawn_failed: u64,
    pub completed: u64,
    pub downstream_overloaded: u64,
    pub denied: u64,
    pub panicked: u64,
    pub released: u64,
    /// Accepted work not yet recorded as released. Release follows the actual permit drop,
    /// so a concurrent admission can briefly overlap the earlier release accounting.
    /// This counter is not an exact semaphore utilization gauge.
    pub outstanding: u64,
    /// High-water mark of `outstanding` accounting, not the semaphore capacity.
    /// Use rejection counters to distinguish immediate admission and downstream overload.
    pub maximum_outstanding: u64,
}
/// Fixed per-stage outcomes remain available after the detailed failure capacity is exhausted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct AuthStageCounts {
    pub succeeded: u64,
    pub denied: u64,
    pub overloaded: u64,
    pub closed: u64,
    pub panicked: u64,
    pub other: u64,
    pub first_failure_micros: Option<u64>,
    pub first_failure_outcome: Option<AuthOutcome>,
}
/// One of the earliest 32 failures in process-local order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthFailure {
    pub sequence: u64,
    pub operation: AuthOperation,
    pub stage: AuthStage,
    pub outcome: AuthOutcome,
}
/// Bounded Host observations. This snapshot is not evidence of authorization or acceptance.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AuthSnapshot {
    pub schema_version: u8,
    pub operation_labels: [AuthOperation; OPERATIONS],
    pub stage_labels: [AuthStage; STAGES],
    pub operations: [AuthOperationCounts; OPERATIONS],
    pub stages: [AuthStageCounts; STAGES],
    pub failures: Vec<AuthFailure>,
    pub omitted_failures: u64,
    pub overflowed: bool,
}
impl AuthSnapshot {
    pub fn stage(&self, stage: AuthStage) -> &AuthStageCounts {
        &self.stages[stage as usize]
    }
    pub fn operation(&self, operation: AuthOperation) -> &AuthOperationCounts {
        &self.operations[operation as usize]
    }
}
struct State {
    operations: [AuthOperationCounts; OPERATIONS],
    stages: [AuthStageCounts; STAGES],
    failures: [Option<AuthFailure>; 32],
    failure_count: usize,
    omitted_failures: u64,
    sequence: u64,
    overflowed: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            operations: [AuthOperationCounts::default(); OPERATIONS],
            stages: [AuthStageCounts::default(); STAGES],
            failures: [None; 32],
            failure_count: 0,
            omitted_failures: 0,
            sequence: 0,
            overflowed: false,
        }
    }
}
fn increment(value: &mut u64, overflowed: &mut bool) {
    match value.checked_add(1) {
        Some(next) => *value = next,
        None => *overflowed = true,
    }
}
impl State {
    fn failure(
        &mut self,
        operation: AuthOperation,
        stage: AuthStage,
        outcome: AuthOutcome,
        elapsed: u128,
    ) {
        let c = &mut self.stages[stage as usize];
        let count = match outcome {
            AuthOutcome::Succeeded => &mut c.succeeded,
            AuthOutcome::Denied => &mut c.denied,
            AuthOutcome::Overloaded => &mut c.overloaded,
            AuthOutcome::Closed => &mut c.closed,
            AuthOutcome::Panicked => &mut c.panicked,
            AuthOutcome::Other => &mut c.other,
        };
        increment(count, &mut self.overflowed);
        if outcome != AuthOutcome::Succeeded && c.first_failure_micros.is_none() {
            c.first_failure_micros = Some(u64::try_from(elapsed).unwrap_or_else(|_| {
                self.overflowed = true;
                u64::MAX
            }));
            c.first_failure_outcome = Some(outcome);
        }
        if outcome == AuthOutcome::Succeeded {
            return;
        }
        increment(&mut self.sequence, &mut self.overflowed);
        if self.failure_count < self.failures.len() {
            self.failures[self.failure_count] = Some(AuthFailure {
                sequence: self.sequence,
                operation,
                stage,
                outcome,
            });
            self.failure_count += 1;
        } else {
            increment(&mut self.omitted_failures, &mut self.overflowed);
        }
    }
}
pub(crate) struct Capture {
    state: Mutex<State>,
    available: AtomicBool,
    started: std::time::Instant,
}
impl Capture {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State::default()),
            available: AtomicBool::new(true),
            started: std::time::Instant::now(),
        })
    }
    fn change(&self, change: impl FnOnce(&mut State)) {
        if !self.available.load(Ordering::Relaxed) {
            return;
        }
        match self.state.lock() {
            Ok(mut state) => change(&mut state),
            Err(_) => self.available.store(false, Ordering::Relaxed),
        }
    }
    pub(crate) fn record(&self, operation: AuthOperation, stage: AuthStage, outcome: AuthOutcome) {
        self.change(|s| {
            s.failure(
                operation,
                stage,
                outcome,
                self.started.elapsed().as_micros(),
            )
        });
    }
    pub(crate) fn attempt(&self, operation: AuthOperation) {
        self.change(|s| {
            increment(
                &mut s.operations[operation as usize].attempts,
                &mut s.overflowed,
            )
        });
    }
    pub(crate) fn rejected(&self, operation: AuthOperation, outcome: AuthOutcome) {
        self.change(|s| {
            let c = &mut s.operations[operation as usize];
            match outcome {
                AuthOutcome::Closed => increment(&mut c.rejected_closed, &mut s.overflowed),
                _ => increment(&mut c.rejected_capacity, &mut s.overflowed),
            }
            s.failure(
                operation,
                AuthStage::Admission,
                outcome,
                self.started.elapsed().as_micros(),
            );
        });
    }
    pub(crate) fn before_spawn(&self, operation: AuthOperation, outcome: AuthOutcome) {
        self.change(|s| {
            increment(
                &mut s.operations[operation as usize].before_spawn_failed,
                &mut s.overflowed,
            );
            s.failure(
                operation,
                AuthStage::BeforeSpawn,
                outcome,
                self.started.elapsed().as_micros(),
            );
        });
    }
    pub(crate) fn admitted(&self, operation: AuthOperation) {
        self.change(|s| {
            let c = &mut s.operations[operation as usize];
            increment(&mut c.admitted, &mut s.overflowed);
            increment(&mut c.outstanding, &mut s.overflowed);
            c.maximum_outstanding = c.maximum_outstanding.max(c.outstanding);
        });
    }
    pub(crate) fn completed(&self, operation: AuthOperation, outcome: AuthOutcome) {
        self.change(|s| {
            let c = &mut s.operations[operation as usize];
            increment(&mut c.completed, &mut s.overflowed);
            match outcome {
                AuthOutcome::Overloaded => {
                    increment(&mut c.downstream_overloaded, &mut s.overflowed)
                }
                AuthOutcome::Denied => increment(&mut c.denied, &mut s.overflowed),
                AuthOutcome::Panicked => increment(&mut c.panicked, &mut s.overflowed),
                _ => {}
            }
            s.failure(
                operation,
                AuthStage::AcceptedWork,
                outcome,
                self.started.elapsed().as_micros(),
            );
        });
    }
    pub(crate) fn released(&self, operation: AuthOperation) {
        self.change(|s| {
            let c = &mut s.operations[operation as usize];
            increment(&mut c.released, &mut s.overflowed);
            if let Some(next) = c.outstanding.checked_sub(1) {
                c.outstanding = next;
            } else {
                s.overflowed = true;
            }
        });
    }
    pub(crate) fn snapshot(&self) -> Result<AuthSnapshot> {
        if !self.available.load(Ordering::Relaxed) {
            return Err(Error::Panicked);
        }
        let state = self.state.lock().map_err(|_| {
            self.available.store(false, Ordering::Relaxed);
            Error::Panicked
        })?;
        Ok(AuthSnapshot {
            schema_version: 2,
            operation_labels: [
                AuthOperation::Unspecified,
                AuthOperation::ProtectedMiddleware,
                AuthOperation::GenericHttpResolver,
                AuthOperation::Blob,
                AuthOperation::Session,
                AuthOperation::CurrentStream,
            ],
            stage_labels: [
                AuthStage::Admission,
                AuthStage::AcceptedWork,
                AuthStage::BeforeSpawn,
                AuthStage::SessionLookup,
                AuthStage::CredentialWait,
                AuthStage::OriginalExpiry,
                AuthStage::CachedBind,
                AuthStage::RenewDue,
                AuthStage::RenewAfterBind,
                AuthStage::KeyAcquisition,
                AuthStage::OriginalTokenVerification,
                AuthStage::FreshBind,
                AuthStage::FreshProofExpiry,
                AuthStage::CurrentBind,
                AuthStage::SessionRemoval,
                AuthStage::BlobReserveResult,
                AuthStage::BlobUploadResult,
                AuthStage::BlobUploadUnattached,
                AuthStage::BlobProjectionResult,
            ],
            operations: state.operations,
            stages: state.stages,
            failures: state.failures.iter().flatten().copied().collect(),
            omitted_failures: state.omitted_failures,
            overflowed: state.overflowed,
        })
    }
}

#[cfg(test)]
#[path = "auth_diagnostics_capture_tests.rs"]
mod tests;
