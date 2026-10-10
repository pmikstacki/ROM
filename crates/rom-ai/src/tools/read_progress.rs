//! Read progress is a bounded advisory snapshot, never retry authority.
use super::{attempt::ReadPhase, transcript::ToolKind};
use crate::{
    AiError, AiResult,
    flow::{RunRecord, RunState},
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// An authorized snapshot of one pending pure read, never a dispatch permission.
pub enum ReadStatus {
    /// The durable read is queued; this snapshot does not authorize a callback.
    Queued,
    /// The trusted host observed a live physical callback or calculation descendant.
    Active,
    /// The trusted host observed no live lease for the retained unresolved read.
    AwaitingRecovery,
    /// The projection has durable metadata but cannot observe physical ownership.
    ActivityUnknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
/// Finite progress excludes tool names, call identities, arguments and results.
pub struct ReadProgress {
    status: ReadStatus,
    ordinal: u32,
}
impl ReadProgress {
    fn new(status: ReadStatus, ordinal: u32) -> AiResult<Self> {
        if !(1..=32).contains(&ordinal) {
            return Err(AiError::InvalidRequest);
        }
        Ok(Self { status, ordinal })
    }
    pub fn status(&self) -> ReadStatus {
        self.status
    }
    /// The persisted callback ordinal is between one and thirty-two.
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }
}
pub(crate) fn durable(record: &RunRecord) -> AiResult<Option<ReadProgress>> {
    if record.state() != &RunState::ToolsPending || record.cancel_requested() {
        return Ok(None);
    }
    let Some(step) = record
        .tool_turns
        .last()
        .and_then(|turn| turn.next())
        .filter(|step| step.kind == ToolKind::Read)
    else {
        return Ok(None);
    };
    let ordinal = step
        .read_attempt
        .as_ref()
        .map_or(1, |attempt| attempt.ordinal);
    let queued = step
        .read_attempt
        .as_ref()
        .map_or(step.actor.is_none(), |attempt| {
            attempt.phase == ReadPhase::Scheduled
        });
    ReadProgress::new(
        if queued {
            ReadStatus::Queued
        } else {
            ReadStatus::ActivityUnknown
        },
        ordinal,
    )
    .map(Some)
}

/// Current policy checks precede this advisory annotation; no callback is invoked here.
pub(crate) async fn authorized(
    state: &crate::flow::HostState,
    runtime: &rom::Runtime,
    actor: &rom::Actor,
    record: &RunRecord,
    execution: &crate::ExecutionDeadline,
) -> AiResult<Option<ReadProgress>> {
    let Some(mut progress) = durable(record)? else {
        return Ok(None);
    };
    tokio::time::timeout(
        std::time::Duration::from_millis(execution.remaining_ms()?),
        async {
            state.authority.inspect(actor, record.owner())?;
            super::worker::authorize_transcript(state, runtime, record, execution).await?;
            super::worker::registry(state, record)?;
            let step = record
                .tool_turns
                .last()
                .and_then(|turn| turn.next())
                .ok_or(AiError::Conflict)?;
            super::worker::current_call(
                state,
                runtime,
                record.owner(),
                &step.call,
                step.actor.as_ref(),
            )
            .await?;
            let prepared = record.active_attempt().ok_or(AiError::Conflict)?.prepared();
            let active = state.read_activity.is_active(
                record.run_id(),
                prepared.identity(),
                step.call.id(),
            )?;
            progress.status = if active {
                ReadStatus::Active
            } else if progress.status == ReadStatus::ActivityUnknown {
                ReadStatus::AwaitingRecovery
            } else {
                ReadStatus::Queued
            };
            Ok(Some(progress))
        },
    )
    .await
    .map_err(|_| AiError::UnknownOutcome)?
}
