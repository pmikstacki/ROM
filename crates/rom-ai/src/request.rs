//! Bounded input; Debug never renders message, schema or tool payloads.
use crate::{AiError, AiResult, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const MAX_PROMPT_BYTES: usize = 32 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_TOOLS: usize = 16;
pub const MAX_TOOL_CALLS_PER_RESPONSE: usize = 8;
pub const MAX_TOOL_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}
impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
    pub fn assistant_calls(tool_calls: Vec<ToolCall>) -> AiResult<Self> {
        validate_calls(&tool_calls)?;
        Ok(Self {
            role: MessageRole::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls,
        })
    }
    pub fn tool_result(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            tool_call_id: Some(id.into()),
            tool_calls: Vec::new(),
        }
    }
}
fn validate_calls(calls: &[ToolCall]) -> AiResult<()> {
    if calls.is_empty() || calls.len() > MAX_TOOL_CALLS_PER_RESPONSE {
        return Err(AiError::InvalidRequest);
    }
    let mut ids = std::collections::BTreeSet::new();
    for call in calls {
        call.validate()?;
        if !ids.insert(call.id()) {
            return Err(AiError::InvalidRequest);
        }
    }
    Ok(())
}
impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputSchema {
    version: u32,
    name: String,
    schema: Value,
}
impl OutputSchema {
    pub fn new(version: u32, name: impl Into<String>, schema: Value) -> AiResult<Self> {
        let value = Self {
            version,
            name: name.into(),
            schema,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn schema(&self) -> &Value {
        &self.schema
    }
    fn validate(&self) -> AiResult<()> {
        if self.version == 0 || !valid_name(&self.name, 64) || !self.schema.is_object() {
            return Err(AiError::InvalidRequest);
        }
        bounded_value(&self.schema, MAX_TOOL_BYTES)
    }
}
impl fmt::Debug for OutputSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputSchema")
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDescriptor {
    name: String,
    description: String,
    parameters: Value,
}
impl ToolDescriptor {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> AiResult<Self> {
        let value = Self {
            name: name.into(),
            description: description.into(),
            parameters,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn parameters(&self) -> &Value {
        &self.parameters
    }
    fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.name, 64)
            || self.description.len() > 1024
            || !self.parameters.is_object()
        {
            return Err(AiError::InvalidRequest);
        }
        bounded_value(&self.parameters, MAX_TOOL_BYTES)
    }
}
impl fmt::Debug for ToolDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ToolDescriptor { .. }")
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionRequest {
    messages: Vec<Message>,
    schema: Option<OutputSchema>,
    tools: Vec<ToolDescriptor>,
    max_output_tokens: u32,
}
impl CompletionRequest {
    pub fn new(messages: Vec<Message>, max_output_tokens: u32) -> AiResult<Self> {
        let value = Self {
            messages,
            schema: None,
            tools: Vec::new(),
            max_output_tokens,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn with_schema(mut self, schema: OutputSchema) -> AiResult<Self> {
        self.schema = Some(schema);
        self.validate()?;
        Ok(self)
    }
    pub fn with_tools(mut self, tools: Vec<ToolDescriptor>) -> AiResult<Self> {
        self.tools = tools;
        self.validate()?;
        Ok(self)
    }
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn schema(&self) -> Option<&OutputSchema> {
        self.schema.as_ref()
    }
    pub fn tools(&self) -> &[ToolDescriptor] {
        &self.tools
    }
    pub fn max_output_tokens(&self) -> u32 {
        self.max_output_tokens
    }
    /// UTF-8 request bytes provide a conservative input bound; adapters add protocol framing.
    pub fn input_bound(&self) -> AiResult<u64> {
        Ok(
            u64::try_from(serde_json::to_vec(self)?.len()).map_err(|_| AiError::InvalidRequest)?
                + 1024,
        )
    }
    pub fn validate(&self) -> AiResult<()> {
        if self.messages.is_empty()
            || self.messages.len() > 128
            || self.max_output_tokens == 0
            || self.max_output_tokens > 16_384
            || self.tools.len() > MAX_TOOLS
        {
            return Err(AiError::InvalidRequest);
        }
        let mut pending = std::collections::BTreeSet::new();
        let mut seen = std::collections::BTreeSet::new();
        for message in &self.messages {
            if message.content.len() > MAX_PROMPT_BYTES
                || (message.role == MessageRole::Tool) != message.tool_call_id.is_some()
                || message
                    .tool_call_id
                    .as_ref()
                    .is_some_and(|id| !valid_name(id, 128))
            {
                return Err(AiError::InvalidRequest);
            }
            if !message.tool_calls.is_empty() {
                if message.role != MessageRole::Assistant || !pending.is_empty() {
                    return Err(AiError::InvalidRequest);
                }
                validate_calls(&message.tool_calls)?;
                for call in &message.tool_calls {
                    if !seen.insert(call.id()) {
                        return Err(AiError::InvalidRequest);
                    }
                    pending.insert(call.id());
                }
            } else if let Some(id) = &message.tool_call_id {
                if !pending.remove(id.as_str()) {
                    return Err(AiError::InvalidRequest);
                }
            } else if !pending.is_empty() {
                return Err(AiError::InvalidRequest);
            }
        }
        if !pending.is_empty() {
            return Err(AiError::InvalidRequest);
        }
        if let Some(schema) = &self.schema {
            schema.validate()?;
        }
        let mut names = std::collections::BTreeSet::new();
        for tool in &self.tools {
            tool.validate()?;
            if !names.insert(tool.name()) {
                return Err(AiError::InvalidRequest);
            }
        }
        bounded_json(self, MAX_PROMPT_BYTES)
    }
}
impl fmt::Debug for CompletionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompletionRequest")
            .field("message_count", &self.messages.len())
            .field("tool_count", &self.tools.len())
            .field("max_output_tokens", &self.max_output_tokens)
            .finish_non_exhaustive()
    }
}
pub(crate) fn valid_name(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.:/".contains(&b))
}
pub(crate) fn bounded_json(value: &impl Serialize, max: usize) -> AiResult<()> {
    if serde_json::to_vec(value)?.len() > max {
        return Err(AiError::InvalidRequest);
    }
    Ok(())
}

pub(crate) fn bounded_value(value: &Value, max: usize) -> AiResult<()> {
    let mut pending = vec![(value, 0_u32)];
    let mut lower_bytes = 0_usize;
    let mut nodes = 0_usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if depth > 32 || nodes > 32_768 {
            return Err(AiError::InvalidRequest);
        }
        let bytes = match value {
            Value::String(text) => text.len(),
            Value::Array(items) => {
                if items.len() > 32_768 {
                    return Err(AiError::InvalidRequest);
                }
                pending.extend(items.iter().map(|item| (item, depth + 1)));
                2
            }
            Value::Object(fields) => {
                if fields.len() > 32_768 {
                    return Err(AiError::InvalidRequest);
                }
                for (key, item) in fields {
                    lower_bytes = lower_bytes
                        .checked_add(key.len())
                        .ok_or(AiError::InvalidRequest)?;
                    pending.push((item, depth + 1));
                }
                2
            }
            _ => 1,
        };
        lower_bytes = lower_bytes
            .checked_add(bytes)
            .ok_or(AiError::InvalidRequest)?;
        if lower_bytes > max {
            return Err(AiError::InvalidRequest);
        }
    }
    bounded_json(value, max)
}
