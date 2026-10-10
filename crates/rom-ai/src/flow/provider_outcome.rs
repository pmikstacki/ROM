//! Process one validated provider observation after the dispatch future is dropped.
use super::worker::{committed_attempt, hold_observed_unknown, hold_unknown, publish_completion};
use super::{
    HostState, RunState,
    callback_phase::{Outcome, RateLimit, Step},
};
use crate::{AiError, AiResult, Completion};
use std::sync::Arc;
pub(super) async fn apply(
    state: Arc<HostState>,
    runtime: Arc<rom::Runtime>,
    input: Box<Outcome>,
) -> AiResult<Step> {
    let Outcome {
        run,
        attempt,
        outcome,
    } = *input;
    let run = run.as_str();
    let attempt = &attempt;
    let completion = match outcome {
        crate::DispatchOutcome::Completed(completion) => completion,
        crate::DispatchOutcome::NotAccepted {
            evidence,
            retry_after_ms,
        } => {
            if evidence.generation_id().is_some() {
                let usage = crate::Usage::default();
                return hold_observed_unknown(
                    &state,
                    &runtime,
                    run,
                    attempt,
                    Some((&evidence, &usage)),
                )
                .await
                .map(|()| Step::Done);
            }
            let now = state.clock.now_unix_ms();
            let retry_at = now
                .checked_add(retry_after_ms)
                .ok_or(AiError::DeadlineExceeded)?;
            return Ok(Step::RateLimit(Box::new(RateLimit {
                run: run.to_owned(),
                attempt: attempt.clone(),
                evidence,
                retry_at,
            })));
        }
        crate::DispatchOutcome::Uncertain {
            evidence, usage, ..
        } => {
            return hold_observed_unknown(
                &state,
                &runtime,
                run,
                attempt,
                Some((&evidence, &usage)),
            )
            .await
            .map(|()| Step::Done);
        }
    };
    if completion.validate().is_err() {
        return hold_unknown(&state, &runtime, run, attempt)
            .await
            .map(|()| Step::Done);
    }
    match completion {
        output @ Completion::Output { .. } => {
            match publish_completion(&state, &runtime, run, attempt, output, true).await {
                Err(AiError::InvalidOutput | AiError::UnsupportedCapability) => {
                    return hold_unknown(&state, &runtime, run, attempt)
                        .await
                        .map(|()| Step::Done);
                }
                result => result?,
            }
        }
        Completion::ToolCalls {
            calls,
            evidence,
            usage,
        } => {
            if state.tools.is_some() && usage.cost.is_some() {
                let admitted = crate::tools::worker::admit(
                    &state,
                    &runtime,
                    run,
                    attempt,
                    calls,
                    evidence.clone(),
                    usage.clone(),
                )
                .await;
                if let Err(error) = admitted {
                    let (_, current) = committed_attempt(&state, &runtime, run, attempt).await?;
                    if matches!(
                        current.state(),
                        RunState::Executing
                            | RunState::CancelRequested
                            | RunState::AwaitingReconciliation
                    ) {
                        // A rejected tool plan does not invalidate already-validated provider knowledge.
                        return hold_observed_unknown(
                            &state,
                            &runtime,
                            run,
                            attempt,
                            Some((&evidence, &usage)),
                        )
                        .await
                        .map(|()| Step::Done);
                    }
                    // Admission may have committed before an acknowledgement or settlement failed.
                    // Preserve the durable batch and its original next tick instead of replacing it.
                    return Err(error);
                }
                return Ok(Step::Done);
            }
            return hold_observed_unknown(
                &state,
                &runtime,
                run,
                attempt,
                Some((&evidence, &usage)),
            )
            .await
            .map(|()| Step::Done);
        }
    }
    Ok(Step::Done)
}
