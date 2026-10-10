//! Explicit clock observations and bounded remaining time.
use crate::{AiError, AiResult};
use serde::{Deserialize, Serialize};

pub trait AiClock: Send + Sync {
    fn now_unix_ms(&self) -> u64;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deadline {
    expires_at_unix_ms: u64,
    remaining_ms: u64,
}
impl Deadline {
    pub fn remaining(now: u64, last_observed: u64, expires: u64) -> AiResult<Self> {
        if now < last_observed || now >= expires {
            return Err(AiError::DeadlineExceeded);
        }
        Ok(Self {
            expires_at_unix_ms: expires,
            remaining_ms: expires - now,
        })
    }
    pub fn expires_at_unix_ms(self) -> u64 {
        self.expires_at_unix_ms
    }
    pub fn remaining_ms(self) -> u64 {
        self.remaining_ms
    }
}
