//! Finite waiting positions over the same FIFO pool that owns active authentication work.
use crate::AuthOperation;
use rom::{Error, Result};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

pub(crate) struct AdmissionQueue {
    requests: Arc<Semaphore>,
    streams: Arc<Semaphore>,
    #[cfg(test)]
    request_capacity: usize,
    #[cfg(test)]
    stream_capacity: usize,
}

impl AdmissionQueue {
    pub(crate) fn new(capacity: usize) -> Self {
        // Five request waves contain the fixed32-call mixed profile plus four Session
        // reacquisitions even while stream work owns every active slot. Active capacity stays
        // unchanged. Host configuration validates capacity <= 256.
        // Invalid oversized internal capacities fail closed instead of overflowing a queue.
        let request_capacity = capacity
            .checked_mul(5)
            .filter(|value| *value <= Semaphore::MAX_PERMITS)
            .unwrap_or(0);
        // Requests cannot consume the stream waiting positions. Both use the same active FIFO.
        Self {
            requests: Arc::new(Semaphore::new(request_capacity)),
            streams: Arc::new(Semaphore::new(capacity)),
            #[cfg(test)]
            request_capacity,
            #[cfg(test)]
            stream_capacity: capacity,
        }
    }

    pub(crate) async fn acquire(
        &self,
        active: Arc<Semaphore>,
        operation: AuthOperation,
        wait: Duration,
    ) -> Result<OwnedSemaphorePermit> {
        match active.clone().try_acquire_owned() {
            Ok(permit) => return Ok(permit),
            Err(TryAcquireError::Closed) => return Err(Error::Closed),
            Err(TryAcquireError::NoPermits) => {}
        }
        let lane = if operation == AuthOperation::CurrentStream {
            &self.streams
        } else {
            &self.requests
        };
        let _position = lane
            .clone()
            .try_acquire_owned()
            .map_err(|error| match error {
                TryAcquireError::Closed => Error::Closed,
                TryAcquireError::NoPermits => Error::Overloaded,
            })?;
        let deadline = tokio::time::Instant::now()
            .checked_add(wait)
            .ok_or(Error::Overloaded)?;
        // Cancellation drops both the position and this single FIFO acquisition.
        // No task has been spawned and no authentication work has been polled yet.
        let permit = tokio::time::timeout_at(deadline, active.acquire_owned())
            .await
            .map_err(|_| Error::Overloaded)?
            .map_err(|_| Error::Closed)?;
        // timeout_at polls its inner future first. Refuse admission after the absolute deadline.
        if tokio::time::Instant::now() >= deadline {
            return Err(Error::Overloaded);
        }
        Ok(permit)
    }

    pub(crate) fn close(&self) {
        self.requests.close();
        self.streams.close();
    }

    #[cfg(test)]
    pub(crate) fn pending_counts(&self) -> (usize, usize) {
        (
            self.request_capacity - self.requests.available_permits(),
            self.stream_capacity - self.streams.available_permits(),
        )
    }
}
