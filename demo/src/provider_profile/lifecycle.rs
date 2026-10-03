//! Admission and a repeatable drain condition owned independently of caller futures.
use rom::{Error, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};

struct State {
    closed: bool,
    active: usize,
}
pub(super) struct Jobs {
    permits: Arc<Semaphore>,
    state: Mutex<State>,
    changed: watch::Sender<usize>,
}
pub(super) struct Job {
    jobs: Arc<Jobs>,
    permit: Option<OwnedSemaphorePermit>,
}
impl Jobs {
    pub(super) fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            permits: Arc::new(Semaphore::new(capacity)),
            state: Mutex::new(State {
                closed: false,
                active: 0,
            }),
            changed: watch::channel(0).0,
        })
    }
    pub(super) fn check_open(&self) -> Result<()> {
        if self.state.lock().map_err(|_| Error::Denied)?.closed {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }
    pub(super) fn admit(self: &Arc<Self>) -> Result<Job> {
        // Registration and close share one short critical section; no I/O occurs here.
        let mut state = self.state.lock().map_err(|_| Error::Denied)?;
        if state.closed {
            return Err(Error::Closed);
        }
        let permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Overloaded)?;
        state.active += 1;
        self.changed.send_replace(state.active);
        Ok(Job {
            jobs: self.clone(),
            permit: Some(permit),
        })
    }
    pub(super) fn close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closed = true;
        self.permits.close();
    }
    pub(super) async fn drain(&self) -> Result<()> {
        let mut changed = self.changed.subscribe();
        loop {
            if *changed.borrow_and_update() == 0 {
                return Ok(());
            }
            changed.changed().await.map_err(|_| Error::Denied)?;
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        drop(self.permit.take());
        let mut state = self
            .jobs
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active -= 1;
        self.jobs.changed.send_replace(state.active);
    }
}
