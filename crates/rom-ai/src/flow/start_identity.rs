//! Callback ownership of START. A receipt replay belongs only to this invocation.
use crate::{AiError, AiResult};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_CALLBACK: AtomicU64 = AtomicU64::new(1);
/// Intentionally not Clone: create once at the trusted channel entry point.
pub(super) struct CallbackIdentity {
    delivery: String,
    attempt: u32,
    nonce: u64,
}
impl CallbackIdentity {
    pub(super) fn new(delivery: &str, attempt: u32) -> AiResult<Self> {
        Self::allocate(delivery, attempt, &NEXT_CALLBACK)
    }
    pub(super) fn allocate(delivery: &str, attempt: u32, counter: &AtomicU64) -> AiResult<Self> {
        if delivery.is_empty() || delivery.len() > 512 || attempt == 0 {
            return Err(AiError::InvalidRequest);
        }
        let nonce = counter
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| AiError::Closed)?;
        Ok(Self {
            delivery: delivery.into(),
            attempt,
            nonce,
        })
    }
    pub(super) fn start(&self, run: &str, prepared: &str, revision: u64) -> AiResult<String> {
        // Canonical typed JSON tuple provides unambiguous framing; no concatenation collisions.
        serde_json::to_string(&(
            "rom-ai-start-owner-v2",
            &self.delivery,
            self.attempt,
            self.nonce,
            run,
            prepared,
            revision,
        ))
        .map_err(|_| AiError::InvalidRequest)
    }
}
