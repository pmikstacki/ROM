//! Consume registration, construct Runtime, attach it, then expose the client.
use super::{client::FlowClient, host::HostState, projection::map_error};
use crate::{AiError, AiResult};
use std::{fmt, sync::Arc};
pub struct FlowBuilder {
    pub(crate) builder: rom::Builder,
    pub(crate) state: Arc<HostState>,
    pub(crate) limits: rom::Limits,
}
impl FlowBuilder {
    pub fn limits(mut self, limits: rom::Limits) -> AiResult<Self> {
        if !(1..=32).contains(&limits.actions)
            || !(1..=32).contains(&limits.io_jobs)
            || limits.command_bytes == 0
            || limits.command_bytes > 1024 * 1024
            || limits.subscriptions == 0
            || limits.snapshot_rows == 0
            || limits.snapshot_bytes == 0
        {
            return Err(AiError::InvalidRequest);
        }
        self.limits = limits;
        self.builder = self.builder.limits(limits);
        Ok(self)
    }
    pub fn build(
        self,
        storage: Arc<dyn rom::Storage>,
        pool: Arc<rayon::ThreadPool>,
    ) -> AiResult<(rom::Runtime, FlowClient)> {
        let runtime = Arc::new(self.builder.build(storage, pool).map_err(map_error)?);
        self.state
            .runtime
            .set(Arc::downgrade(&runtime))
            .map_err(|_| AiError::Conflict)?;
        self.state.provider_permits.add_permits(self.limits.io_jobs);
        Ok((
            runtime.as_ref().clone(),
            FlowClient {
                runtime,
                state: self.state,
            },
        ))
    }
}
impl fmt::Debug for FlowBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FlowBuilder { .. }")
    }
}
