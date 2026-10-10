//! Bounded provisional responses; only the flow validator can authorize domain output.
use crate::{
    AiError, AiResult, UsdNanos,
    request::{
        MAX_OUTPUT_BYTES, MAX_TOOL_BYTES, MAX_TOOL_CALLS_PER_RESPONSE, bounded_json, bounded_value,
        valid_name,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub cost: Option<UsdNanos>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptEvidence {
    attempt_id: String,
    provider_request_id: Option<String>,
    generation_id: Option<String>,
}
impl AttemptEvidence {
    pub fn new(
        attempt_id: impl Into<String>,
        provider_request_id: Option<String>,
        generation_id: Option<String>,
    ) -> AiResult<Self> {
        let value = Self {
            attempt_id: attempt_id.into(),
            provider_request_id,
            generation_id,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }
    pub fn provider_request_id(&self) -> Option<&str> {
        self.provider_request_id.as_deref()
    }
    pub fn generation_id(&self) -> Option<&str> {
        self.generation_id.as_deref()
    }
    pub fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.attempt_id, 160)
            || self
                .provider_request_id
                .as_ref()
                .is_some_and(|id| !valid_name(id, 128))
            || self
                .generation_id
                .as_ref()
                .is_some_and(|id| !valid_name(id, 128))
        {
            return Err(AiError::InvalidOutput);
        }
        Ok(())
    }
}
impl fmt::Debug for AttemptEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AttemptEvidence { .. }")
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: Value,
}
impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> AiResult<Self> {
        let value = Self {
            id: id.into(),
            name: name.into(),
            arguments,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn arguments(&self) -> &Value {
        &self.arguments
    }
    pub fn validate(&self) -> AiResult<()> {
        if !valid_name(&self.id, 128) || !valid_name(&self.name, 64) || !self.arguments.is_object()
        {
            return Err(AiError::InvalidOutput);
        }
        bounded_value(&self.arguments, MAX_TOOL_BYTES).map_err(|_| AiError::InvalidOutput)
    }
}
impl fmt::Debug for ToolCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ToolCall { .. }")
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Completion {
    Output {
        value: Value,
        usage: Usage,
        evidence: AttemptEvidence,
    },
    ToolCalls {
        calls: Vec<ToolCall>,
        usage: Usage,
        evidence: AttemptEvidence,
    },
}
impl Completion {
    pub fn output(value: Value, usage: Usage, evidence: AttemptEvidence) -> AiResult<Self> {
        let result = Self::Output {
            value,
            usage,
            evidence,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn tool_calls(
        calls: Vec<ToolCall>,
        usage: Usage,
        evidence: AttemptEvidence,
    ) -> AiResult<Self> {
        let result = Self::ToolCalls {
            calls,
            usage,
            evidence,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> AiResult<()> {
        match self {
            Self::Output {
                value, evidence, ..
            } => {
                evidence.validate()?;
                bounded_value(value, MAX_OUTPUT_BYTES).map_err(|_| AiError::InvalidOutput)?;
            }
            Self::ToolCalls {
                calls, evidence, ..
            } => {
                evidence.validate()?;
                if calls.is_empty() || calls.len() > MAX_TOOL_CALLS_PER_RESPONSE {
                    return Err(AiError::InvalidOutput);
                }
                let mut ids = std::collections::BTreeSet::new();
                for call in calls {
                    call.validate()?;
                    if !ids.insert(call.id()) {
                        return Err(AiError::InvalidOutput);
                    }
                }
            }
        }
        bounded_json(self, MAX_OUTPUT_BYTES).map_err(|_| AiError::InvalidOutput)
    }
}
impl fmt::Debug for Completion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Output { .. } => f.write_str("Completion::Output { .. }"),
            Self::ToolCalls { calls, .. } => f
                .debug_struct("Completion::ToolCalls")
                .field("count", &calls.len())
                .finish_non_exhaustive(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Reconciliation {
    Accepted { completion: Completion },
    NotAccepted,
    Unresolved,
}
