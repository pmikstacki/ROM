//! Restricted trusted read handle; no mutation or Runtime accessor.
use crate::ExecutionDeadline;
use std::{fmt, sync::Arc};

pub struct ReadContext {
    pub(crate) runtime: Arc<rom::Runtime>,
    pub(crate) actor: rom::Actor,
    pub(crate) deadline: ExecutionDeadline,
    pub(crate) lease: Option<Arc<super::attempt::ReadLease>>,
}
impl ReadContext {
    pub fn actor(&self) -> &rom::Actor {
        &self.actor
    }
    pub async fn read<R: rom::Resource>(&self, id: &str) -> rom::Result<rom::Snapshot<R>> {
        let milliseconds = self
            .deadline
            .remaining_ms()
            .map_err(|_| rom::Error::Closed)?;
        tokio::time::timeout(
            std::time::Duration::from_millis(milliseconds),
            self.runtime.read::<R>(&self.actor, id),
        )
        .await
        .map_err(|_| rom::Error::Closed)?
    }
    pub async fn query<R: rom::Resource>(
        &self,
        query: &rom::Query<R>,
    ) -> rom::Result<Vec<rom::Snapshot<R>>> {
        let limit = query.spec().limit.unwrap_or(64).min(64);
        if limit == 0 {
            return Err(rom::Error::TooLarge);
        }
        let bounded = query.clone().limit(limit);
        let milliseconds = self
            .deadline
            .remaining_ms()
            .map_err(|_| rom::Error::Closed)?;
        tokio::time::timeout(
            std::time::Duration::from_millis(milliseconds),
            self.runtime.query(&self.actor, &bounded),
        )
        .await
        .map_err(|_| rom::Error::Closed)?
    }
}
impl fmt::Debug for ReadContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReadContext { .. }")
    }
}
