//! Conservative HTTP 429 knowledge; no retry after accepted or incomplete evidence.
use rom_ai::{AiError, AiResult, AttemptEvidence, DispatchOutcome, PreparedAttempt, Usage};
use serde::Deserialize;

pub(crate) fn classify(
    attempt: &PreparedAttempt,
    evidence: AttemptEvidence,
    body: &AiResult<Vec<u8>>,
    retry: Option<&str>,
    sse: bool,
    contradictory_identity: bool,
) -> AiResult<DispatchOutcome> {
    let refusal = body
        .as_ref()
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Refusal>(bytes).ok());
    let mut usage = body
        .as_ref()
        .ok()
        .and_then(|bytes| crate::response::observed_usage(bytes).ok())
        .unwrap_or_default();
    let observed = DispatchOutcome::Uncertain {
        evidence: evidence.clone(),
        usage: usage.clone(),
        cause: AiError::UnknownOutcome,
    };
    if observed.validate_for(attempt).is_err() {
        usage = Usage::default();
    }
    let valid_refusal = refusal
        .as_ref()
        .is_some_and(|value| value.error.code == 429);
    if sse || contradictory_identity || evidence.generation_id().is_some() || !valid_refusal {
        return Ok(DispatchOutcome::Uncertain {
            evidence,
            usage,
            cause: AiError::UnknownOutcome,
        });
    }
    let retry_after_ms = match crate::errors::retry_floor(retry, crate::errors::now_ms()?) {
        Ok(floor) => floor,
        Err(_) => {
            return Ok(DispatchOutcome::Uncertain {
                evidence,
                usage,
                cause: AiError::UnknownOutcome,
            });
        }
    };
    let result = DispatchOutcome::NotAccepted {
        evidence,
        retry_after_ms,
    };
    result.validate_for(attempt)?;
    Ok(result)
}

// Named fields make duplicate refusal keys invalid; no last-value-wins acceptance proof.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Refusal {
    error: RefusalError,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RefusalError {
    code: u16,
    #[serde(default, rename = "message")]
    _message: Option<String>,
    #[serde(default, rename = "metadata")]
    _metadata: Option<serde_json::Value>,
}
