//! One callback, one permit, and one active phase future. Completed phases never poll successors.
use super::{FlowTick, HostState, callback_phase::Step, start_identity::CallbackIdentity};
use crate::{AiError, AiResult, ExecutionDeadline};
use std::{future::Future, pin::Pin, sync::Arc};

struct Context {
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    execution: ExecutionDeadline,
    start_owner: CallbackIdentity,
}
type PhaseFuture<'a> = Pin<Box<dyn Future<Output = AiResult<Step>> + Send + 'a>>;

pub(super) async fn run(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    tick: FlowTick,
    execution: ExecutionDeadline,
    start_owner: CallbackIdentity,
) -> AiResult<()> {
    let permit = state
        .provider_permits
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AiError::Closed)?;
    let context = Context {
        state,
        runtime,
        execution,
        start_owner,
    };
    drive(permit, Step::Tick(tick), |step| make_phase(&context, step)).await
}

// Ownership spans the whole loop, including pending phases and error propagation.
// Tests inject phase futures at this private boundary; production uses make_phase unchanged.
async fn drive<'a>(
    _permit: tokio::sync::OwnedSemaphorePermit,
    mut next: Step,
    mut factory: impl FnMut(Step) -> PhaseFuture<'a>,
) -> AiResult<()> {
    loop {
        if matches!(next, Step::Done) {
            return Ok(());
        }
        let mut active = factory(next);
        let result = active.as_mut().await;
        // This drop is before successor construction, including on an error result.
        drop(active);
        next = result?;
    }
}

// This non-async factory keeps concrete child-future construction out of the driver's poll body.
// Acyclic transitions end this callback after at most eight phases; exact-command retries
// remain local to their original phase. No yield, task, provider retry or new timer is added.
fn make_phase(context: &Context, step: Step) -> PhaseFuture<'_> {
    match step {
        Step::Tick(tick) => Box::pin(super::worker::tick_phase(
            context.state.clone(),
            context.runtime.clone(),
            tick,
            context.execution.clone(),
            &context.start_owner,
        )),
        Step::Continue(run) => Box::pin(async move {
            super::continuation_worker::prepare_phase(
                context.state.clone(),
                context.runtime.clone(),
                &run,
                context.execution.clone(),
            )
            .await
        }),
        Step::Start(input) => Box::pin(async move {
            super::continuation_worker::start_phase(
                context.state.clone(),
                context.runtime.clone(),
                input.record,
                input.revision,
                context.execution.clone(),
                &context.start_owner,
            )
            .await
        }),
        Step::Dispatch(input) => Box::pin(async move {
            super::worker::dispatch_phase(
                context.state.clone(),
                context.runtime.clone(),
                &input.run,
                &input.attempt,
                input.now,
                context.execution.clone(),
            )
            .await
        }),
        Step::Outcome(input) => Box::pin(super::provider_outcome::apply(
            context.state.clone(),
            context.runtime.clone(),
            input,
        )),
        Step::RateLimit(input) => Box::pin(async move {
            super::worker::rate_limit_phase(
                &context.state,
                &context.runtime,
                &input.run,
                &input.attempt,
                &input.evidence,
                input.retry_at,
            )
            .await
        }),
        Step::Confirm(input) => Box::pin(async move {
            super::continuation_worker::confirm_phase(
                &context.state,
                &context.runtime,
                super::continuation_worker::ConfirmationRequest {
                    run: &input.run,
                    attempt: &input.attempt,
                    evidence: &input.evidence,
                    proof: input.proof,
                    now: input.now,
                    due: input.due,
                },
            )
            .await
            .map(Step::Settle)
        }),
        Step::Settle(input) => Box::pin(async move {
            super::account_settlement::apply(&context.state, &context.runtime, input)
                .await
                .map(|()| Step::Done)
        }),
        Step::Done => Box::pin(async { Ok(Step::Done) }),
    }
}

#[cfg(test)]
mod tests;
