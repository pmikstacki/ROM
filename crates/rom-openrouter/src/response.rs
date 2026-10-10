//! Bounded native completion decoding.
use rom_ai::{AiError, AiResult, AttemptEvidence, Completion, PreparedAttempt, ToolCall, Usage};
use serde::Deserialize;
use serde_json::{Value, value::RawValue};
#[derive(Deserialize)]
struct Envelope {
    model: Option<String>,
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<Value>,
    #[serde(default)]
    usage: Option<WireUsage>,
}
#[derive(Deserialize)]
struct Choice {
    finish_reason: Option<String>,
    message: NativeMessage,
}
#[derive(Deserialize)]
struct NativeMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<NativeCall>,
}
#[derive(Deserialize)]
struct NativeCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: Function,
}
#[derive(Deserialize)]
struct Function {
    name: String,
    arguments: String,
}
#[derive(Deserialize, Default)]
struct WireUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    completion_tokens_details: Option<Details>,
    cost: Option<Box<RawValue>>,
}
#[derive(Deserialize)]
struct Details {
    reasoning_tokens: Option<u64>,
}
impl WireUsage {
    fn decode(self) -> AiResult<Usage> {
        Ok(Usage {
            input_tokens: self.prompt_tokens,
            output_tokens: self.completion_tokens,
            reasoning_tokens: self
                .completion_tokens_details
                .and_then(|details| details.reasoning_tokens),
            cost: self
                .cost
                .map(|cost| crate::money::scaled(cost.get(), 9))
                .transpose()?,
        })
    }
}
pub(crate) fn decode(
    bytes: &[u8],
    attempt: &PreparedAttempt,
    evidence: AttemptEvidence,
) -> AiResult<Completion> {
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| AiError::InvalidOutput)?;
    if envelope.error.is_some() {
        return Err(AiError::UnknownOutcome);
    }
    if envelope.model.as_deref() != Some(attempt.route().model()) || envelope.choices.len() != 1 {
        return Err(AiError::InvalidOutput);
    }
    let usage = envelope.usage.unwrap_or_default().decode()?;
    let choice = envelope
        .choices
        .into_iter()
        .next()
        .ok_or(AiError::InvalidOutput)?;
    let completion = if choice.finish_reason.as_deref() == Some("tool_calls") {
        if choice.message.tool_calls.len() > 8 {
            return Err(AiError::InvalidOutput);
        }
        let mut calls = Vec::with_capacity(choice.message.tool_calls.len());
        for call in choice.message.tool_calls {
            if call.kind != "function"
                || !attempt
                    .request()
                    .tools()
                    .iter()
                    .any(|tool| tool.name() == call.function.name)
                || call.function.arguments.len() > 16 * 1024
            {
                return Err(AiError::InvalidOutput);
            }
            let arguments = serde_json::from_str(&call.function.arguments)
                .map_err(|_| AiError::InvalidOutput)?;
            calls.push(ToolCall::new(call.id, call.function.name, arguments)?);
        }
        Completion::tool_calls(calls, usage, evidence)?
    } else {
        if choice.finish_reason.as_deref() != Some("stop") || !choice.message.tool_calls.is_empty()
        {
            return Err(AiError::InvalidOutput);
        }
        let content = choice
            .message
            .content
            .filter(|value| !value.is_empty() && value.len() <= 64 * 1024)
            .ok_or(AiError::InvalidOutput)?;
        let value = if attempt.request().schema().is_some() {
            serde_json::from_str(&content).map_err(|_| AiError::InvalidOutput)?
        } else {
            Value::String(content)
        };
        Completion::output(value, usage, evidence)?
    };
    rom_ai::DispatchOutcome::Completed(completion.clone()).validate_for(attempt)?;
    Ok(completion)
}

/// Extract only bounded usage knowledge; an error envelope does not prove nonacceptance.
pub(crate) fn observed_usage(bytes: &[u8]) -> AiResult<Usage> {
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| AiError::InvalidOutput)?;
    envelope.usage.unwrap_or_default().decode()
}
