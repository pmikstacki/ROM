//! Ephemeral execution time never changes a durable attempt or grants authority.
use crate::{AiError, AiResult};
use std::{
    fmt,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct ExecutionDeadline {
    end: Instant,
}
impl ExecutionDeadline {
    pub(crate) fn end(&self) -> Instant {
        self.end
    }
    /// Create a bounded host deadline. Copies retain this original monotonic end.
    pub fn from_remaining(duration: Duration) -> AiResult<Self> {
        if duration.is_zero() || duration > Duration::from_secs(18) {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self {
            end: Instant::now()
                .checked_add(duration)
                .ok_or(AiError::InvalidRequest)?,
        })
    }
    /// Floor remaining time; less than one millisecond cannot authorize another call.
    pub fn remaining_ms(&self) -> AiResult<u64> {
        let remaining = self
            .end
            .saturating_duration_since(Instant::now())
            .as_millis();
        if remaining == 0 {
            return Err(AiError::DeadlineExceeded);
        }
        u64::try_from(remaining).map_err(|_| AiError::DeadlineExceeded)
    }
    /// Reserve time at the original end without restarting the clock.
    pub fn shortened(&self, margin: Duration) -> AiResult<Self> {
        let shortened = Self {
            end: self
                .end
                .checked_sub(margin)
                .ok_or(AiError::DeadlineExceeded)?,
        };
        shortened.remaining_ms()?;
        Ok(shortened)
    }
}
impl fmt::Debug for ExecutionDeadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ExecutionDeadline { .. }")
    }
}
