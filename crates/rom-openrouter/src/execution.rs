//! HTTP timing is ephemeral; it does not modify durable provider-call identity.
use rom_ai::{AiError, AiResult, Deadline, ExecutionDeadline};
use std::time::Duration;

pub(crate) fn deadline(execution: &ExecutionDeadline, expiry: Option<u64>) -> AiResult<Deadline> {
    let observed = execution.shortened(Duration::from_millis(250))?;
    let now = crate::errors::now_ms()?;
    let end = now
        .checked_add(observed.remaining_ms()?)
        .ok_or(AiError::DeadlineExceeded)?;
    Deadline::remaining(now, now, expiry.map_or(end, |expiry| expiry.min(end)))
}
