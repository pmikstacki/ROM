//! Current authority and payload-free error conversion for trusted host APIs.
use super::{OwnerIdentity, RunRecord, RunView, host::HostState};
use crate::{AiError, AiResult};
use std::sync::Arc;
pub(crate) fn map_error(error: rom::Error) -> AiError {
    match error {
        rom::Error::Denied => AiError::Denied,
        rom::Error::Closed => AiError::Closed,
        rom::Error::Conflict | rom::Error::IdentityMismatch => AiError::Conflict,
        rom::Error::TooLarge | rom::Error::Overloaded => AiError::BudgetExhausted,
        _ => AiError::Storage,
    }
}
pub(crate) async fn current(runtime: &rom::Runtime, actor: &rom::Actor) -> AiResult<rom::Actor> {
    let candidate = actor.clone();
    runtime
        .establish_actor(move |_| Ok(candidate.clone()))
        .await
        .map_err(map_error)
}
pub(crate) async fn dispatch_actor(
    state: Arc<HostState>,
    runtime: &rom::Runtime,
    owner: &OwnerIdentity,
) -> AiResult<rom::Actor> {
    let frozen = owner.clone();
    let authority = state.authority.clone();
    let actor = runtime
        .establish_actor(move |reads| authority.resolve(&frozen, reads))
        .await
        .map_err(map_error)?;
    if !owner.matches(&actor) {
        return Err(AiError::Denied);
    }
    state.authority.inspect(&actor, owner)?;
    Ok(actor)
}
pub(crate) async fn dispatch_attempt(
    state: Arc<HostState>,
    runtime: &rom::Runtime,
    owner: &OwnerIdentity,
    prepared: &crate::PreparedAttempt,
) -> AiResult<rom::Actor> {
    prepared.validate()?;
    let frozen = owner.clone();
    let attempt = prepared.clone();
    let authority = state.authority.clone();
    let actor = runtime
        .establish_actor(move |reads| {
            let actor = authority.resolve(&frozen, reads)?;
            if !frozen.matches(&actor) {
                return Err(rom::Error::Denied);
            }
            authority.attempt(&actor, &frozen, &attempt, reads)?;
            Ok(actor)
        })
        .await
        .map_err(map_error)?;
    state.authority.inspect(&actor, owner)?;
    Ok(actor)
}
pub(crate) async fn view(
    state: &HostState,
    runtime: &rom::Runtime,
    actor: &rom::Actor,
    record: &RunRecord,
    revision: u64,
) -> AiResult<RunView> {
    let actor = current(runtime, actor).await?;
    state.authority.inspect(&actor, record.owner())?;
    let execution = crate::ExecutionDeadline::from_remaining(std::time::Duration::from_secs(2))?;
    if record.state() == &super::RunState::Completed && !record.cancel_requested() {
        // Result disclosure is a fresh grant decision, not a cached owner identity.
        // HostState is shared by the client; this helper only needs current policy/read access.
        crate::tools::worker::authorize_transcript(state, runtime, record, &execution).await?;
    }
    let progress =
        crate::tools::read_progress::authorized(state, runtime, &actor, record, &execution).await?;
    let mut view = RunView::project(record, revision, &actor, |actor, owner| {
        state.authority.inspect(actor, owner)
    })?;
    view.annotate_read_progress(progress)?;
    Ok(view)
}
