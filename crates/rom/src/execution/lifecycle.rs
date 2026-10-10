//! Admission, supervised blocking work, observation and shutdown.
use super::{Runtime, Work};
use crate::{Actor, Error, Result};
use crate::{DiagnosticOutcome, DiagnosticStage, execution::diagnostic_record};
use serde::Serialize;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
/// Whether intake accepts work or shutdown is draining/completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum IntakeState {
    Open,
    Draining,
    Stopped,
}
/// Payload-free host operations snapshot. Permit availability is advisory under
/// concurrent work; the intake/owned-work pair is sampled under the lifecycle lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimeStatus {
    pub intake: IntakeState,
    pub failed: bool,
    /// Tracked I/O batches and worker-loop lifetimes, not detached host tasks.
    pub owned_work: usize,
    pub available_action_permits: usize,
    pub available_io_permits: usize,
    pub available_subscription_permits: usize,
    pub registered_resources: usize,
    pub registered_reactions: usize,
    pub registered_channels: usize,
}
impl RuntimeStatus {
    /// Ready means configured intake is open. Capacity/backpressure is reported separately.
    pub fn is_ready(&self) -> bool {
        self.intake == IntakeState::Open && !self.failed
    }
}
/// Conservative host policy. Counts include work whose caller stopped waiting.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub actions: usize,
    pub io_jobs: usize,
    pub subscriptions: usize,
    pub snapshot_rows: usize,
    pub snapshot_bytes: usize,
    pub command_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            actions: 8,
            io_jobs: 8,
            subscriptions: 64,
            snapshot_rows: 1024,
            snapshot_bytes: 1024 * 1024,
            command_bytes: 16 * 1024,
        }
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        drop(self._io.take());
        let mut state = self.lifecycle.lock().unwrap();
        state.active -= 1;
        self.drained.send_modify(|v| *v = v.wrapping_add(1));
    }
}
pub(crate) fn acquire(pool: &Arc<Semaphore>) -> Result<OwnedSemaphorePermit> {
    pool.clone().try_acquire_owned().map_err(|e| match e {
        tokio::sync::TryAcquireError::Closed => Error::Closed,
        tokio::sync::TryAcquireError::NoPermits => Error::Overloaded,
    })
}
impl Runtime {
    pub(crate) fn acquire_action(&self) -> Result<OwnedSemaphorePermit> {
        acquire(&self.0.admission).map_err(|error| {
            if matches!(error, Error::Overloaded) {
                self.0.core_overloads.record_action_no_permits();
            }
            error
        })
    }
    pub(crate) fn track_worker(&self) -> Result<Work> {
        let mut state = self.0.lifecycle.lock().map_err(|_| Error::Panicked)?;
        if let Some(e) = &state.terminal {
            return Err(e.clone());
        }
        if state.closed {
            return Err(Error::Closed);
        }
        state.active += 1;
        Ok(Work {
            lifecycle: self.0.lifecycle.clone(),
            drained: self.0.drained.clone(),
            _io: None,
        })
    }
    /// Host-only operational state with no actor, Resource identity or payload data.
    /// Stopped is published only after tracked work releases its adapter-owning references.
    pub fn status(&self) -> Result<RuntimeStatus> {
        let state = self.0.lifecycle.lock().map_err(|_| Error::Panicked)?;
        let intake = if !state.closed {
            IntakeState::Open
        } else if state.active > 0 {
            IntakeState::Draining
        } else {
            IntakeState::Stopped
        };
        Ok(RuntimeStatus {
            intake,
            failed: state.terminal.is_some(),
            owned_work: state.active,
            available_action_permits: self.0.admission.available_permits(),
            available_io_permits: self.0.io.available_permits(),
            available_subscription_permits: self.0.subscriptions.available_permits(),
            registered_resources: self.0.registry.len(),
            registered_reactions: self.0.reactions.len(),
            registered_channels: self.0.channels.len(),
        })
    }
    pub fn available_capacity(&self) -> usize {
        self.0.admission.available_permits()
    }
    pub fn available_io_capacity(&self) -> usize {
        self.0.io.available_permits()
    }
    pub(crate) fn ensure_open(&self) -> Result<()> {
        let state = self.0.lifecycle.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closed {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }
    pub(super) fn invalidate(&self) {
        self.0.generation.fetch_add(1, Ordering::SeqCst);
        self.0.changes.send_modify(|v| *v = v.wrapping_add(1));
    }
    pub(super) fn fail_terminal(&self) {
        let mut state = self.0.lifecycle.lock().unwrap();
        state.terminal = Some(Error::Panicked);
        state.closed = true;
        self.0.admission.close();
        self.0.io.close();
        self.0.subscriptions.close();
        drop(state);
        self.invalidate();
    }
    pub(crate) async fn io<T: Send + 'static, F: FnOnce(&Runtime) -> Result<T> + Send + 'static>(
        &self,
        f: F,
    ) -> Result<T> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            if let Some(error) = &state.terminal {
                return Err(error.clone());
            }
            if state.closed {
                return Err(Error::Closed);
            }
            let permit = acquire(&self.0.io).map_err(|error| {
                if matches!(error, Error::Overloaded) {
                    self.0.core_overloads.record_io_no_permits();
                }
                error
            })?;
            state.active += 1;
            let work = Work {
                lifecycle: self.0.lifecycle.clone(),
                drained: self.0.drained.clone(),
                _io: Some(permit),
            };
            let runtime = self.clone();
            tokio::task::spawn_blocking(move || {
                let result = match catch_unwind(AssertUnwindSafe(|| f(&runtime))) {
                    Ok(result) => result,
                    Err(_) => {
                        runtime.fail_terminal();
                        Err(Error::Panicked)
                    }
                };
                // Release the adapter-owning Runtime before publishing the drain condition.
                drop(runtime);
                drop(work);
                let _ = sender.send(result);
            });
        }
        receiver.await.map_err(|_| Error::Panicked)?
    }
    pub(crate) async fn observe<
        T: Send + 'static,
        F: Fn(&Runtime) -> Result<T> + Send + Sync + 'static,
    >(
        &self,
        actor: &Actor,
        f: F,
    ) -> Result<T> {
        self.check_actor(actor)?;
        let f = Arc::new(f);
        for _ in 0..8 {
            let a = actor.clone();
            let operation = f.clone();
            let (result, generation) = self
                .io(move |runtime| {
                    let _guard = runtime.0.gate.lock().map_err(|_| Error::Panicked)?;
                    runtime.check_authority(&a)?;
                    let result = operation(runtime)?;
                    runtime.check_authority(&a)?;
                    Ok((result, runtime.0.generation.load(Ordering::SeqCst)))
                })
                .await?;
            self.check_actor(actor)?;
            self.ensure_open()?;
            #[cfg(test)]
            super::overload_generation::before_generation_compare(
                self,
                super::overload_generation::Boundary::Observation,
            )
            .await?;
            if generation == self.0.generation.load(Ordering::SeqCst) {
                return Ok(result);
            }
        }
        self.0
            .core_overloads
            .record_observation_generation_exhausted();
        Err(Error::Overloaded)
    }
    /// Every caller observes the same runtime-owned drain condition. Cancelling a waiter cannot lose work.
    /// Diagnostics describe this waiter. Cancellation can leave a Started observation without a terminal record.
    /// Use `status` for current runtime state; diagnostic records are bounded and lossy.
    pub async fn shutdown(&self) -> Result<()> {
        let mut changed = self.0.drained.subscribe();
        {
            let mut state = self.0.lifecycle.lock().unwrap();
            state.closed = true;
            self.0.admission.close();
            self.0.io.close();
            self.0.subscriptions.close();
        }
        self.invalidate();
        let mut recording = self
            .0
            .diagnostics
            .as_ref()
            .map(|sink| sink.shutdown_record());
        let started = recording.as_ref().and_then(|record| record.timer());
        diagnostic_record::stage(
            &mut recording,
            DiagnosticStage::Shutdown,
            DiagnosticOutcome::Started,
            0,
        );
        diagnostic_record::publish(recording);
        let result = loop {
            let finished = {
                let state = self.0.lifecycle.lock().unwrap();
                if state.active == 0 {
                    Some(state.terminal.clone().map_or(Ok(()), Err))
                } else {
                    None
                }
            };
            if let Some(result) = finished {
                break result;
            }
            if changed.changed().await.is_err() {
                break Err(Error::Panicked);
            }
        };
        if let Some(sink) = &self.0.diagnostics {
            let mut recording = sink.shutdown_record();
            recording.stage_since(
                DiagnosticStage::Shutdown,
                result
                    .as_ref()
                    .map_or_else(diagnostic_record::outcome, |_| DiagnosticOutcome::Succeeded),
                started,
                0,
            );
            recording.publish();
        }
        result
    }
}
