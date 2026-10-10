//! Exact prepared-request wire encoding.
use rom_ai::{AiError, AiResult, MessageRole, ModelPrice, PreparedAttempt, RoutingTier};
use serde::Serialize;
use serde_json::{Value, json, value::RawValue};
#[derive(Serialize)]
struct Wire<'a> {
    model: &'a str,
    messages: Vec<Value>,
    max_tokens: u32,
    stream: bool,
    provider: Selection<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
}
#[derive(Serialize)]
struct Selection<'a> {
    require_parameters: bool,
    allow_fallbacks: bool,
    max_price: Prices,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    only: &'a [String],
}
#[derive(Serialize)]
struct Prices {
    prompt: Box<RawValue>,
    completion: Box<RawValue>,
    request: Box<RawValue>,
}
pub(crate) fn encode(attempt: &PreparedAttempt) -> AiResult<Vec<u8>> {
    attempt.validate()?;
    let request = attempt.request();
    let mut messages = Vec::with_capacity(request.messages().len());
    for message in request.messages() {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let mut wire = json!({"role": role, "content": message.content});
        if let Some(id) = &message.tool_call_id {
            wire["tool_call_id"] = json!(id);
        }
        if !message.tool_calls.is_empty() {
            let calls = message.tool_calls.iter().map(|call| Ok(json!({"id":call.id(),"type":"function","function":{"name":call.name(),"arguments":serde_json::to_string(call.arguments())?}}))).collect::<AiResult<Vec<_>>>()?;
            wire["tool_calls"] = json!(calls);
            if message.content.is_empty() {
                wire["content"] = Value::Null;
            }
        }
        messages.push(wire);
    }
    let caps = match attempt.route().tier() {
        RoutingTier::Free => ModelPrice::free(),
        RoutingTier::Paid => attempt
            .policy()
            .paid_caps()
            .ok_or(AiError::InvalidRequest)?,
    };
    let tools: Vec<_> = request.tools().iter().map(|tool| json!({"type":"function","function":{"name":tool.name(),"description":tool.description(),"parameters":tool.parameters()}})).collect();
    let wire = Wire {
        model: attempt.route().model(), messages, max_tokens: request.max_output_tokens(), stream: false,
        provider: Selection {require_parameters:true,allow_fallbacks:false,only:attempt.policy().allowed_providers(),max_price:Prices {prompt:crate::money::wire_usd(caps.prompt_per_million)?,completion:crate::money::wire_usd(caps.completion_per_million)?,request:crate::money::wire_usd(caps.request)?}},
        response_format:request.schema().map(|schema| json!({"type":"json_schema","json_schema":{"name":schema.name(),"strict":true,"schema":schema.schema()}})),
        parallel_tool_calls: (!tools.is_empty()).then_some(false), tools,
    };
    let bytes = serde_json::to_vec(&wire)?;
    if bytes.len() as u64 > request.input_bound()? {
        return Err(AiError::InvalidRequest);
    }
    Ok(bytes)
}
