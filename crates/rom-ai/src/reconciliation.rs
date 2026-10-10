//! Trusted reconciliation observations retain financial knowledge without fabricating output.
use crate::{
    AiError, AiResult, AttemptEvidence, DispatchOutcome, PreparedAttempt, Reconciliation, Usage,
};
use std::fmt;

/// Adapter evidence is not a wire credential or an authorization decision.
#[derive(Clone, PartialEq, Eq)]
pub enum ReconciliationObservation {
    Resolved(Reconciliation),
    Uncertain {
        evidence: AttemptEvidence,
        usage: Usage,
    },
}
impl ReconciliationObservation {
    /// Neutral coherence only; current durable knowledge and authority still require validation.
    pub fn validate_for(
        &self,
        prepared: &PreparedAttempt,
        original: &AttemptEvidence,
    ) -> AiResult<()> {
        prepared.validate()?;
        original.validate()?;
        if original.attempt_id() != prepared.identity() {
            return Err(AiError::InvalidOutput);
        }
        let evidence = match self {
            Self::Resolved(Reconciliation::Accepted { completion }) => {
                DispatchOutcome::Completed(completion.clone()).validate_for(prepared)?;
                match completion {
                    crate::Completion::Output { evidence, .. }
                    | crate::Completion::ToolCalls { evidence, .. } => Some(evidence),
                }
            }
            Self::Uncertain { evidence, usage } => {
                DispatchOutcome::Uncertain {
                    evidence: evidence.clone(),
                    usage: usage.clone(),
                    cause: AiError::UnknownOutcome,
                }
                .validate_for(prepared)?;
                Some(evidence)
            }
            Self::Resolved(Reconciliation::NotAccepted | Reconciliation::Unresolved) => None,
        };
        if let Some(evidence) = evidence {
            for (old, new) in [
                (original.generation_id(), evidence.generation_id()),
                (
                    original.provider_request_id(),
                    evidence.provider_request_id(),
                ),
            ] {
                if old.is_some() && new.is_some() && old != new {
                    return Err(AiError::InvalidOutput);
                }
            }
        }
        Ok(())
    }
}
impl fmt::Debug for ReconciliationObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReconciliationObservation { .. }")
    }
}
