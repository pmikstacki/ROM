//! Public failure categories contain no provider payload or diagnostic text.
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub type AiResult<T> = Result<T, AiError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AiError {
    InvalidRequest,
    UnsupportedCapability,
    Denied,
    DeadlineExceeded,
    BudgetExhausted,
    InvalidOutput,
    RateLimited { retry_after_ms: u64 },
    ProviderUnavailable,
    Conflict,
    UnknownOutcome,
    Closed,
    Storage,
}
impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::Denied => "denied",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::BudgetExhausted => "budget_exhausted",
            Self::InvalidOutput => "invalid_output",
            Self::RateLimited { .. } => "rate_limited",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::Conflict => "conflict",
            Self::UnknownOutcome => "unknown_outcome",
            Self::Closed => "closed",
            Self::Storage => "storage",
        })
    }
}
impl Error for AiError {}
impl From<serde_json::Error> for AiError {
    fn from(_: serde_json::Error) -> Self {
        Self::InvalidRequest
    }
}
