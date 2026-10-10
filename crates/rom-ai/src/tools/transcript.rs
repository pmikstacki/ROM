//! Bounded native continuation facts; original submitted messages never change.
use super::PreparedToolAction;
use crate::flow::codec::Validated;
use crate::{
    AiError, AiResult, CompletionRequest, Message, ToolCall,
    flow::{OwnerIdentity, ReservationKey},
    request::{MAX_TOOL_BYTES, bounded_value},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, fmt};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum ToolKind {
    Read,
    Action,
}

/// Presence of this envelope means completion, including a legitimate JSON null.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolResult {
    pub value: Value,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolStep {
    pub call: ToolCall,
    pub kind: ToolKind,
    pub actor: Option<OwnerIdentity>,
    pub prepared: Option<PreparedToolAction>,
    pub result: Option<ToolResult>,
    pub unknown: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_attempt: Option<super::attempt::ReadAttempt>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ToolTurn {
    pub source: ReservationKey,
    pub registry_version: u32,
    pub steps: Vec<ToolStep>,
}
impl ToolTurn {
    pub fn completed(&self) -> bool {
        self.steps.iter().all(|step| step.result.is_some())
    }
    pub fn completed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.result.is_some())
            .count()
    }
    pub fn next(&self) -> Option<&ToolStep> {
        self.steps.iter().find(|step| step.result.is_none())
    }
}
impl fmt::Debug for ToolTurn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ToolTurn { .. }")
    }
}

pub(crate) fn effective(
    original: &CompletionRequest,
    turns: &[ToolTurn],
    generation: u32,
) -> AiResult<CompletionRequest> {
    let mut messages = original.messages().to_vec();
    for turn in turns
        .iter()
        .filter(|turn| turn.source.attempt_ordinal() < generation)
    {
        if !turn.completed() {
            return Err(AiError::Conflict);
        }
        messages.push(Message::assistant_calls(
            turn.steps.iter().map(|step| step.call.clone()).collect(),
        )?);
        for step in &turn.steps {
            let result = step.result.as_ref().ok_or(AiError::Conflict)?;
            bounded_value(&result.value, MAX_TOOL_BYTES)?;
            messages.push(Message::tool_result(
                step.call.id(),
                serde_json::to_string(&result.value)?,
            ));
        }
    }
    let mut request = CompletionRequest::new(messages, original.max_output_tokens())?
        .with_tools(original.tools().to_vec())?;
    if let Some(schema) = original.schema() {
        request = request.with_schema(schema.clone())?;
    }
    request.validate()?;
    Ok(request)
}

pub(crate) fn source_checkpoint(generation: u32, turns: &[ToolTurn]) -> AiResult<u32> {
    let completed = turns
        .iter()
        .filter(|turn| turn.source.attempt_ordinal() < generation)
        .try_fold(0_u32, |count, turn| {
            count
                .checked_add(
                    u32::try_from(turn.completed_count()).map_err(|_| AiError::InvalidRequest)?,
                )
                .ok_or(AiError::InvalidRequest)
        })?;
    generation
        .checked_add(completed)
        .ok_or(AiError::InvalidRequest)
}

pub(crate) fn validate(
    original: &CompletionRequest,
    turns: &[ToolTurn],
    run_id: &str,
    policy_version: u32,
    registry_version: Option<u32>,
    generation_attempts: u32,
    tool_limit: u32,
) -> AiResult<u32> {
    if turns.len() > 8 || registry_version == Some(0) {
        return Err(AiError::InvalidRequest);
    }
    let mut ids = BTreeSet::new();
    let mut last_generation = 0;
    let mut completed = 0_u32;
    let mut total = 0_u32;
    for (index, turn) in turns.iter().enumerate() {
        turn.source.validate()?;
        if turn.source.run_id() != run_id
            || turn.source.policy_version() != policy_version
            || turn.source.attempt_ordinal() <= last_generation
            || turn.source.attempt_ordinal() > generation_attempts
            || turn.source.step()
                != source_checkpoint(turn.source.attempt_ordinal(), &turns[..index])?
            || Some(turn.registry_version) != registry_version
            || turn.steps.is_empty()
            || turn.steps.len() > 8
            || (index + 1 < turns.len() && !turn.completed())
        {
            return Err(AiError::InvalidRequest);
        }
        last_generation = turn.source.attempt_ordinal();
        let mut incomplete = false;
        for step in &turn.steps {
            if incomplete
                && (step.actor.is_some()
                    || step.prepared.is_some()
                    || step.result.is_some()
                    || step.unknown)
            {
                return Err(AiError::InvalidRequest);
            }
            step.call.validate()?;
            if let Some(attempt) = &step.read_attempt {
                attempt.validate()?;
                if step.kind != ToolKind::Read
                    || step.prepared.is_some()
                    || step.unknown
                    || (attempt.phase == super::attempt::ReadPhase::Completed)
                        != step.result.is_some()
                    || (attempt.phase != super::attempt::ReadPhase::Scheduled
                        && step.actor.is_none())
                {
                    return Err(AiError::InvalidRequest);
                }
            }
            if !ids.insert(step.call.id())
                || !original
                    .tools()
                    .iter()
                    .any(|tool| tool.name() == step.call.name())
            {
                return Err(AiError::InvalidOutput);
            }
            total += 1;
            if let Some(actor) = &step.actor {
                actor.validate_identity()?;
            }
            if let Some(prepared) = &step.prepared {
                prepared.validate()?;
                if step.kind != ToolKind::Action
                    || Some(&prepared.actor) != step.actor.as_ref()
                    || prepared.call_id() != step.call.id()
                    || prepared.tool_name != step.call.name()
                    || prepared.registry_version != turn.registry_version
                {
                    return Err(AiError::InvalidRequest);
                }
            }
            if let Some(result) = &step.result {
                if incomplete
                    || step.actor.is_none()
                    || (step.kind == ToolKind::Action && step.prepared.is_none())
                    || step.unknown
                {
                    return Err(AiError::InvalidRequest);
                }
                bounded_value(&result.value, MAX_TOOL_BYTES)?;
                completed += 1;
            } else {
                incomplete = true;
                if step.unknown && (step.kind != ToolKind::Action || step.prepared.is_none()) {
                    return Err(AiError::InvalidRequest);
                }
            }
        }
    }
    if total > tool_limit {
        return Err(AiError::BudgetExhausted);
    }
    let next_generation = generation_attempts
        .checked_add(1)
        .ok_or(AiError::InvalidRequest)?;
    let effective_generation = turns
        .last()
        .filter(|turn| !turn.completed())
        .map_or(next_generation, |turn| turn.source.attempt_ordinal());
    effective(original, turns, effective_generation)?;
    Ok(completed)
}
