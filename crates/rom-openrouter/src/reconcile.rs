//! Conservative generation lookup.
use rom_ai::{
    AiError, AiResult, AttemptEvidence, Deadline, PreparedAttempt, Reconciliation,
    ReconciliationObservation, Usage,
};
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct MetadataEnvelope {
    data: Metadata,
}
#[derive(Deserialize)]
struct Metadata {
    id: String,
    model: String,
    request_id: Option<String>,
    total_cost: Option<Box<RawValue>>,
    native_tokens_prompt: Option<u64>,
    native_tokens_completion: Option<u64>,
    native_tokens_reasoning: Option<u64>,
}
pub(crate) fn deadline() -> AiResult<Deadline> {
    let now = crate::errors::now_ms()?;
    Deadline::remaining(
        now,
        now,
        now.checked_add(2000).ok_or(AiError::DeadlineExceeded)?,
    )
}
pub(crate) fn observed(
    bytes: &[u8],
    prepared: &PreparedAttempt,
    original: &AttemptEvidence,
) -> AiResult<ReconciliationObservation> {
    let envelope: MetadataEnvelope =
        serde_json::from_slice(bytes).map_err(|_| AiError::InvalidOutput)?;
    let metadata = envelope.data;
    if Some(metadata.id.as_str()) != original.generation_id()
        || metadata.model != prepared.route().model()
    {
        return Err(AiError::InvalidOutput);
    }
    let evidence = AttemptEvidence::new(
        prepared.identity(),
        metadata
            .request_id
            .or_else(|| original.provider_request_id().map(str::to_owned)),
        Some(metadata.id),
    )?;
    let usage = Usage {
        input_tokens: metadata.native_tokens_prompt,
        output_tokens: metadata.native_tokens_completion,
        reasoning_tokens: metadata.native_tokens_reasoning,
        cost: metadata
            .total_cost
            .map(|cost| crate::money::scaled(cost.get(), 9))
            .transpose()?,
    };
    let observed = ReconciliationObservation::Uncertain { evidence, usage };
    observed.validate_for(prepared, original)?;
    Ok(observed)
}
pub(crate) fn metadata(bytes: &[u8], evidence: &AttemptEvidence) -> AiResult<Reconciliation> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| AiError::InvalidOutput)?;
    if value
        .get("data")
        .and_then(|value| value.get("id"))
        .and_then(serde_json::Value::as_str)
        != evidence.generation_id()
    {
        return Err(AiError::InvalidOutput);
    }
    // A matching generation is not a reconstructable completion or proof of nonacceptance.
    Ok(Reconciliation::Unresolved)
}
