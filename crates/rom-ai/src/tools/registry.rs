//! Immutable-at-install declarations; registered functions never come from model output.
use super::{ReadContext, action, schema};
use crate::{
    AiError, AiFuture, AiResult, ToolDescriptor,
    request::{MAX_TOOL_BYTES, MAX_TOOLS, bounded_value},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt, sync::Arc};

type Read = Arc<dyn Fn(ReadContext, Value) -> AiFuture<'static, Value> + Send + Sync>;
pub(crate) struct RegisteredRead {
    pub(crate) descriptor: ToolDescriptor,
    pub(crate) read: Read,
}
pub struct ToolRegistry {
    pub(crate) version: u32,
    pub(crate) reads: BTreeMap<String, RegisteredRead>,
    pub(crate) actions: BTreeMap<String, RegisteredAction>,
}
pub(crate) struct RegisteredAction {
    pub(crate) descriptor: ToolDescriptor,
    pub(crate) prepare: action::Prepare,
    pub(crate) kind: &'static str,
    pub(crate) resource_version: u32,
    pub(crate) action_name: &'static str,
}
impl RegisteredAction {
    pub(crate) fn validate_prepared(
        &self,
        version: u32,
        run_id: &str,
        call: &crate::ToolCall,
        prepared: &super::PreparedToolAction,
    ) -> AiResult<()> {
        prepared.validate()?;
        let arguments = call.arguments().as_object().ok_or(AiError::InvalidOutput)?;
        if prepared.registry_version != version
            || prepared.resource_version != self.resource_version
            || prepared.tool_name != call.name()
            || prepared.call_id() != call.id()
            || prepared.invocation.kind != self.kind
            || prepared.invocation.idempotency != format!("ai-tool-{run_id}-{}", call.id())
            || arguments.get("target").and_then(Value::as_str) != Some(prepared.target())
            || arguments.get("expected_revision").and_then(Value::as_u64)
                != Some(prepared.expected_revision())
            || !matches!(&prepared.invocation.operation,rom::Operation::Action{name,..} if name==self.action_name)
        {
            return Err(AiError::Conflict);
        }
        Ok(())
    }
}
impl ToolRegistry {
    pub(crate) fn validate_requested(&self, request: &crate::CompletionRequest) -> AiResult<()> {
        let installed = self.descriptors();
        if request
            .tools()
            .iter()
            .any(|requested| !installed.contains(requested))
        {
            return Err(AiError::UnsupportedCapability);
        }
        Ok(())
    }
    pub fn new(version: u32) -> AiResult<Self> {
        if version == 0 {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self {
            version,
            reads: BTreeMap::new(),
            actions: BTreeMap::new(),
        })
    }
    /// Register a trusted bounded, side-effect-free read callback.
    /// Mutations must use Actions; captured closures are not sandboxed.
    /// Join child calculation work before returning and do not escape mutating work.
    /// Recovery may repeat the exact call after physical execution ends, under fresh grants.
    pub fn read<I: rom::Input, O: rom::Input, F>(
        &mut self,
        name: &str,
        description: &str,
        read: F,
    ) -> AiResult<()>
    where
        F: Fn(ReadContext, I) -> AiFuture<'static, O> + Send + Sync + 'static,
    {
        if self.reads.len() + self.actions.len() >= MAX_TOOLS
            || self.reads.contains_key(name)
            || self.actions.contains_key(name)
        {
            return Err(AiError::InvalidRequest);
        }
        let descriptor = ToolDescriptor::new(
            name,
            description,
            json!({
                "type":"object","properties":{"input":schema::input::<I>()?},
                "required":["input"],"additionalProperties":false
            }),
        )?;
        let read = Arc::new(read);
        self.reads.insert(
            name.into(),
            RegisteredRead {
                descriptor,
                read: Arc::new(move |context, arguments| {
                    let read = read.clone();
                    Box::pin(async move {
                        bounded_value(&arguments, MAX_TOOL_BYTES)?;
                        let object = arguments.as_object().ok_or(AiError::InvalidOutput)?;
                        if object.len() != 1 {
                            return Err(AiError::InvalidOutput);
                        }
                        let input =
                            I::decode(object.get("input").ok_or(AiError::InvalidOutput)?.clone())
                                .map_err(|_| AiError::InvalidOutput)?;
                        let output = read(context, input).await?.encode();
                        bounded_value(&output, MAX_TOOL_BYTES)
                            .map_err(|_| AiError::InvalidOutput)?;
                        Ok(output)
                    })
                }),
            },
        );
        Ok(())
    }
    pub fn descriptors(&self) -> Vec<ToolDescriptor> {
        self.reads
            .values()
            .map(|entry| entry.descriptor.clone())
            .chain(self.actions.values().map(|entry| entry.descriptor.clone()))
            .collect()
    }
    pub fn action<R: rom::Resource, I: rom::Input>(
        &mut self,
        name: &str,
        description: &str,
        action: rom::Action<R, I>,
    ) -> AiResult<()> {
        if self.reads.len() + self.actions.len() >= MAX_TOOLS
            || self.reads.contains_key(name)
            || self.actions.contains_key(name)
        {
            return Err(AiError::InvalidRequest);
        }
        let descriptor = ToolDescriptor::new(
            name,
            description,
            json!({
                "type":"object","properties":{
                    "target":{"type":"string","minLength":1,"maxLength":256},
                    "expected_revision":{"type":"integer","minimum":1,"maximum":u64::MAX},
                    "input":schema::input::<I>()?
                },"required":["target","expected_revision","input"],"additionalProperties":false
            }),
        )?;
        self.actions.insert(
            name.into(),
            RegisteredAction {
                descriptor,
                prepare: action::prepare(self.version, name.into(), action),
                kind: R::KIND,
                resource_version: R::descriptor().version,
                action_name: action.name(),
            },
        );
        Ok(())
    }
}
impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("version", &self.version)
            .field("count", &(self.reads.len() + self.actions.len()))
            .finish_non_exhaustive()
    }
}
