//! Frozen public Runtime invocation; decoded identity is never a credential.
use crate::{
    AiError, AiResult, ToolCall,
    flow::OwnerIdentity,
    request::{MAX_TOOL_BYTES, bounded_value, valid_name},
};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedToolAction {
    pub(crate) registry_version: u32,
    pub(crate) tool_name: String,
    pub(crate) resource_version: u32,
    pub(crate) call_id: String,
    pub(crate) actor: OwnerIdentity,
    pub(crate) invocation: rom::Invocation,
}
impl PreparedToolAction {
    pub fn call_id(&self) -> &str {
        &self.call_id
    }
    pub fn target(&self) -> &str {
        &self.invocation.id
    }
    pub fn expected_revision(&self) -> u64 {
        self.invocation.expected.unwrap_or(0)
    }
    pub fn idempotency(&self) -> &str {
        &self.invocation.idempotency
    }
    pub fn retry_epoch(&self) -> u64 {
        self.invocation.retry_epoch
    }
    pub fn validate(&self) -> AiResult<()> {
        self.actor.validate_identity()?;
        crate::request::bounded_json(self, 32 * 1024)?;
        if self.registry_version == 0
            || self.resource_version == 0
            || !valid_name(&self.tool_name, 64)
            || !valid_name(&self.call_id, 128)
            || !valid_name(&self.invocation.id, 256)
            || !valid_name(&self.invocation.kind, 256)
            || !valid_name(&self.invocation.idempotency, 256)
            || self
                .invocation
                .expected
                .is_none_or(|revision| revision == 0)
        {
            return Err(AiError::InvalidRequest);
        }
        match &self.invocation.operation {
            rom::Operation::Action { name, input } if valid_name(name, 256) => {
                bounded_value(input, MAX_TOOL_BYTES)
            }
            _ => Err(AiError::InvalidRequest),
        }
    }
}
impl fmt::Debug for PreparedToolAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparedToolAction { .. }")
    }
}
pub(crate) type Prepare =
    Arc<dyn Fn(&ToolCall, &str, &rom::Actor, u64) -> AiResult<PreparedToolAction> + Send + Sync>;
pub(crate) fn prepare<R: rom::Resource, I: rom::Input>(
    version: u32,
    name: String,
    action: rom::Action<R, I>,
) -> Prepare {
    Arc::new(move |call, run_id, actor, epoch| {
        call.validate()?;
        bounded_value(call.arguments(), MAX_TOOL_BYTES)?;
        if call.name() != name || !valid_name(run_id, 64) {
            return Err(AiError::InvalidOutput);
        }
        let object = call.arguments().as_object().ok_or(AiError::InvalidOutput)?;
        if object.len() != 3 {
            return Err(AiError::InvalidOutput);
        }
        let target = object
            .get("target")
            .and_then(serde_json::Value::as_str)
            .ok_or(AiError::InvalidOutput)?;
        let revision = object
            .get("expected_revision")
            .and_then(serde_json::Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or(AiError::InvalidOutput)?;
        let input = I::decode(object.get("input").ok_or(AiError::InvalidOutput)?.clone())
            .map_err(|_| AiError::InvalidOutput)?;
        let command = rom::Command::action(target, action, input)
            .at_revision(revision)
            .idempotency(&format!("ai-tool-{run_id}-{}", call.id()))
            .retry_epoch(epoch);
        let prepared = PreparedToolAction {
            registry_version: version,
            tool_name: name.clone(),
            resource_version: R::descriptor().version,
            call_id: call.id().into(),
            actor: OwnerIdentity::from_actor(actor)?,
            invocation: command.into(),
        };
        prepared.validate()?;
        Ok(prepared)
    })
}
