use crate::auth_diagnostics::{AuthOperation, AuthOutcome, AuthSnapshot, AuthStage, Capture};
use crate::authentication::admission::AdmissionQueue;
use rom::{Error, Result};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::{
    sync::{Semaphore, oneshot},
    task::JoinHandle,
};

struct Jobs {
    tasks: Vec<JoinHandle<()>>,
    failed: bool,
}
pub(crate) struct Supervisor {
    admission: Arc<Semaphore>,
    waiting: AdmissionQueue,
    jobs: Mutex<Jobs>,
    capture: Option<Arc<Capture>>,
}
impl Supervisor {
    #[cfg(test)]
    pub(crate) fn new(capacity: usize) -> Self {
        Self::observed(capacity, None)
    }
    pub(crate) fn observed(capacity: usize, capture: Option<Arc<Capture>>) -> Self {
        Self {
            capture,
            admission: Arc::new(Semaphore::new(capacity)),
            waiting: AdmissionQueue::new(capacity),
            jobs: Mutex::new(Jobs {
                tasks: Vec::new(),
                failed: false,
            }),
        }
    }
    pub(crate) async fn run<T, F>(&self, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        self.run_tagged(AuthOperation::Unspecified, work).await
    }
    pub(crate) fn record(&self, operation: AuthOperation, stage: AuthStage, outcome: AuthOutcome) {
        if let Some(capture) = &self.capture {
            capture.record(operation, stage, outcome);
        }
    }
    pub(crate) fn authentication_diagnostics(&self) -> Result<Option<AuthSnapshot>> {
        self.capture
            .as_ref()
            .map(|capture| capture.snapshot())
            .transpose()
    }
    pub(crate) async fn run_tagged<T, F>(&self, operation: AuthOperation, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        if let Some(capture) = &self.capture {
            capture.attempt(operation);
        }
        let permit = self.admission.clone().try_acquire_owned().map_err(|_| {
            let error = if self.admission.is_closed() {
                Error::Closed
            } else {
                Error::Overloaded
            };
            self.rejected(operation, error)
        })?;
        self.accept(operation, permit, work).await
    }
    pub(crate) async fn run_queued_tagged<T, F>(
        &self,
        operation: AuthOperation,
        wait: std::time::Duration,
        work: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        if let Some(capture) = &self.capture {
            capture.attempt(operation);
        }
        let permit = self
            .waiting
            .acquire(self.admission.clone(), operation, wait)
            .await
            .map_err(|error| self.rejected(operation, error))?;
        self.accept(operation, permit, work).await
    }
    fn rejected(&self, operation: AuthOperation, error: Error) -> Error {
        if let Some(capture) = &self.capture {
            capture.rejected(operation, AuthOutcome::result::<()>(&Err(error.clone())));
        }
        error
    }
    #[cfg(test)]
    pub(crate) fn queued_counts(&self) -> (usize, usize) {
        self.waiting.pending_counts()
    }
    async fn accept<T, F>(
        &self,
        operation: AuthOperation,
        permit: tokio::sync::OwnedSemaphorePermit,
        work: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T>> + Send + 'static,
    {
        let (send, receive) = oneshot::channel();
        {
            let mut jobs = self.jobs.lock().map_err(|_| {
                if let Some(capture) = &self.capture {
                    capture.before_spawn(operation, AuthOutcome::Panicked);
                }
                Error::Panicked
            })?;
            if self.admission.is_closed() {
                if let Some(capture) = &self.capture {
                    capture.before_spawn(operation, AuthOutcome::Closed);
                }
                return Err(Error::Closed);
            }
            // Reap finished tasks without losing a panic from an earlier callback.
            let mut cx = Context::from_waker(Waker::noop());
            let mut index = 0;
            while index < jobs.tasks.len() {
                if jobs.tasks[index].is_finished()
                    && let Poll::Ready(result) = Pin::new(&mut jobs.tasks[index]).poll(&mut cx)
                {
                    jobs.failed |= result.is_err();
                    drop(jobs.tasks.swap_remove(index));
                    continue;
                }
                index += 1;
            }
            let mut accepted = AcceptedAuth::new(permit, self.capture.clone(), operation);
            jobs.tasks.push(tokio::spawn(async move {
                let result = work.await;
                accepted.complete(AuthOutcome::result(&result));
                drop(accepted);
                let _ = send.send(result);
            }));
        }
        receive.await.map_err(|_| Error::Panicked)?
    }
    pub(crate) fn close(&self) {
        self.admission.close();
        self.waiting.close();
    }
    pub(crate) async fn drain(&self) -> Result<()> {
        self.close();
        poll_fn(|cx| {
            let Ok(mut jobs) = self.jobs.lock() else {
                return Poll::Ready(Err(Error::Panicked));
            };
            let mut index = 0;
            while index < jobs.tasks.len() {
                match Pin::new(&mut jobs.tasks[index]).poll(cx) {
                    Poll::Ready(result) => {
                        if result.is_err() {
                            jobs.failed = true;
                        }
                        drop(jobs.tasks.swap_remove(index));
                    }
                    Poll::Pending => index += 1,
                }
            }
            if jobs.tasks.is_empty() {
                Poll::Ready(if jobs.failed {
                    Err(Error::Panicked)
                } else {
                    Ok(())
                })
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

// This guard follows the accepted task, never its cancelled caller.
struct AcceptedAuth {
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
    capture: Option<Arc<Capture>>,
    operation: AuthOperation,
    completed: bool,
}
impl AcceptedAuth {
    fn new(
        permit: tokio::sync::OwnedSemaphorePermit,
        capture: Option<Arc<Capture>>,
        operation: AuthOperation,
    ) -> Self {
        if let Some(capture) = &capture {
            capture.admitted(operation);
        }
        Self {
            permit: Some(permit),
            capture,
            operation,
            completed: false,
        }
    }
    fn complete(&mut self, outcome: AuthOutcome) {
        if let Some(capture) = &self.capture {
            capture.completed(self.operation, outcome);
        }
        self.completed = true;
    }
}
impl Drop for AcceptedAuth {
    fn drop(&mut self) {
        if !self.completed {
            self.complete(AuthOutcome::Panicked);
        }
        drop(self.permit.take());
        if let Some(capture) = &self.capture {
            capture.released(self.operation);
        }
    }
}

#[cfg(test)]
#[path = "auth_diagnostics_supervision_tests.rs"]
mod auth_diagnostics_supervision_tests;
