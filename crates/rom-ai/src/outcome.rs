//! Trusted adapter observations retain bounded knowledge without granting authority.
use crate::{AiError, AiResult, AttemptEvidence, Completion, PreparedAttempt, Usage};
use std::fmt;

/// This is a trusted adapter result, not a deserializable dispatch credential.
#[derive(Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    Completed(Completion),
    NotAccepted {
        evidence: AttemptEvidence,
        retry_after_ms: u64,
    },
    Uncertain {
        evidence: AttemptEvidence,
        usage: Usage,
        cause: AiError,
    },
}
impl DispatchOutcome {
    /// Validate neutral coherence only. Current authority and committed account proof remain mandatory.
    pub fn validate_for(&self, attempt: &PreparedAttempt) -> AiResult<()> {
        attempt.validate()?;
        let (evidence, usage) = match self {
            Self::Completed(completion) => {
                completion.validate()?;
                match completion {
                    Completion::Output {
                        evidence, usage, ..
                    }
                    | Completion::ToolCalls {
                        evidence, usage, ..
                    } => (evidence, Some(usage)),
                }
            }
            Self::NotAccepted {
                evidence,
                retry_after_ms,
            } => {
                if !(1..=300_000).contains(retry_after_ms) {
                    return Err(AiError::InvalidOutput);
                }
                (evidence, None)
            }
            Self::Uncertain {
                evidence, usage, ..
            } => (evidence, Some(usage)),
        };
        evidence.validate()?;
        if evidence.attempt_id() != attempt.identity() {
            return Err(AiError::InvalidOutput);
        }
        if let Some(usage) = usage {
            let output_limit = u64::from(attempt.request().max_output_tokens());
            if usage
                .cost
                .is_some_and(|cost| cost > attempt.route().maximum_cost())
                || usage
                    .input_tokens
                    .is_some_and(|count| count > attempt.request().input_bound().unwrap_or(0))
                || usage
                    .output_tokens
                    .is_some_and(|count| count > output_limit)
                || usage
                    .reasoning_tokens
                    .is_some_and(|count| count > usage.output_tokens.unwrap_or(output_limit))
            {
                return Err(AiError::InvalidOutput);
            }
        }
        Ok(())
    }
}
impl fmt::Debug for DispatchOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DispatchOutcome { .. }")
    }
}
