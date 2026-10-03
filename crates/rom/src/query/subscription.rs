//! Shared admission and acknowledgement state for live query observations.
use crate::{Actor, Error, Result, Runtime, execution};
use tokio::sync::{OwnedSemaphorePermit, watch};

pub(crate) struct Subscription {
    changes: watch::Receiver<u64>,
    delivered_generation: Option<u64>,
    _permit: OwnedSemaphorePermit,
}

impl Subscription {
    pub(crate) fn begin(runtime: &Runtime, actor: &Actor) -> Result<Self> {
        runtime.ensure_open()?;
        runtime.check_actor(actor)?;
        let permit = execution::acquire(&runtime.0.subscriptions)?;
        let changes = runtime.0.changes.subscribe();
        Ok(Self {
            changes,
            delivered_generation: None,
            _permit: permit,
        })
    }

    pub(crate) async fn pending(&mut self, runtime: &Runtime, actor: &Actor) -> Result<u64> {
        loop {
            runtime.check_actor(actor)?;
            runtime.ensure_open()?;
            // Clearing watch's marker does not acknowledge a failed or cancelled query.
            let generation = *self.changes.borrow_and_update();
            if self.delivered_generation == Some(generation) {
                self.changes.changed().await.map_err(|_| Error::Closed)?;
                continue;
            }
            return Ok(generation);
        }
    }

    /// Acknowledge only after successful selection and authorization. Use the
    /// generation captured before the query so changes during it remain pending.
    pub(crate) fn acknowledge(&mut self, generation: u64) {
        self.delivered_generation = Some(generation);
    }
}
