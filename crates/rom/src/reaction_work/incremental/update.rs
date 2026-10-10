//! Existing five work transitions over the shared overlay.
use super::*;
use crate::reaction_work::control::{finish_retry_stop_reason, retry_stop_reason};
use crate::reaction_work::delivery_profile::{await_reconciliation, requires_reconciliation};
impl<R: WorkRead> Engine<'_, R> {
    pub(super) fn live_claim(&mut self, claim: &ClaimKey, now: u64) -> Result<WorkRecord> {
        let record = self.record(&claim.id)?.ok_or(Error::Missing)?;
        if now < record.pending.eligibility_floor()
            || !matches!(record.state, WorkState::Leased { until, generation, .. } if until > now && generation == claim.generation)
        {
            return Err(Error::Conflict);
        }
        Ok(record)
    }
    pub(super) fn update(&mut self, update: WorkUpdate) -> Result<WorkResult> {
        let limits = self.header.limits.clone().ok_or(Error::Storage)?;
        match update {
            WorkUpdate::DeliveryStarted { claim, now } => {
                let mut record = self.live_claim(&claim, now)?;
                if !matches!(record.pending.payload, WorkPayload::Notification { .. })
                    || matches!(
                        record.state,
                        WorkState::Leased {
                            resolution_only: Some(_),
                            ..
                        }
                    )
                {
                    return Err(Error::Conflict);
                }
                record.delivery = Some(DeliveryOutcome::Unknown);
                self.put(record)?;
                Ok(WorkResult::Changed)
            }
            WorkUpdate::DeliveryFinished {
                claim,
                now,
                outcome,
            } => {
                let mut record = self.live_claim(&claim, now)?;
                if !matches!(record.pending.payload, WorkPayload::Notification { .. }) {
                    return Err(Error::Conflict);
                }
                record.delivery = Some(outcome.clone());
                let finish = if requires_reconciliation(&record) {
                    WorkOutcome::Retry
                } else {
                    match outcome {
                        DeliveryOutcome::Accepted => WorkOutcome::Done,
                        DeliveryOutcome::Permanent => {
                            WorkOutcome::Stop(StopReason::DeliveryPermanent)
                        }
                        DeliveryOutcome::Panicked => {
                            WorkOutcome::Stop(StopReason::CallbackPanicked)
                        }
                        DeliveryOutcome::Unknown
                        | DeliveryOutcome::Retryable
                        | DeliveryOutcome::TimedOut => WorkOutcome::Retry,
                    }
                };
                self.put(record)?;
                self.update(WorkUpdate::Finish {
                    claim,
                    now,
                    outcome: finish,
                })
            }
            WorkUpdate::Claim { now } => {
                let mut after = None;
                let mut changed = false;
                while let Some(mut record) = self.candidate(now, after.as_deref())? {
                    after = Some(record.pending.id.clone());
                    if record.state == WorkState::Pending
                        && record.due > now
                        && now.saturating_sub(record.pending.cause.started_at)
                            >= limits.max_age_seconds
                    {
                        record.state = WorkState::Stopped(StopReason::Age);
                        self.put(record)?;
                        changed = true;
                        continue;
                    }
                    if !matches!(record.state, WorkState::Pending if record.due <= now)
                        && !matches!(record.state, WorkState::Leased { until, .. } if until <= now)
                    {
                        return Err(Error::Storage);
                    }
                    if requires_reconciliation(&record) {
                        await_reconciliation(&mut record)?;
                        self.put(record)?;
                        changed = true;
                        continue;
                    }
                    let root =
                        self.root(&record.pending.cause.root, record.pending.cause.retry_epoch)?;
                    if root.members == 0 {
                        return Err(Error::Storage);
                    }
                    let reason = retry_stop_reason(&record, &limits, root.used, now);
                    if reason.is_none() {
                        record.attempts = record.attempts.checked_add(1).ok_or(Error::TooLarge)?;
                    }
                    record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
                    record.state = WorkState::Leased {
                        until: now
                            .checked_add(limits.lease_seconds)
                            .ok_or(Error::TooLarge)?,
                        generation: record.generation,
                        resolution_only: reason.clone(),
                    };
                    self.put(record.clone())?;
                    return Ok(WorkResult::Claimed(Box::new(WorkClaim {
                        work: record,
                        resolution_only: reason.is_some(),
                        stop_reason: reason,
                    })));
                }
                Ok(if changed {
                    WorkResult::Changed
                } else {
                    WorkResult::Idle
                })
            }
            WorkUpdate::Materialize {
                claim,
                now,
                children,
            } => {
                let mut parent = self.live_claim(&claim, now)?;
                if matches!(
                    parent.state,
                    WorkState::Leased {
                        resolution_only: Some(_),
                        ..
                    }
                ) {
                    return Err(Error::Conflict);
                }
                if !matches!(parent.pending.payload, WorkPayload::Source(_))
                    || children.len() > limits.max_fanout
                {
                    return Err(Error::TooLarge);
                }
                if children.iter().any(|child| {
                    let pending = &parent.pending;
                    child.cause.retry_epoch != pending.cause.retry_epoch
                        || child.cause.root != pending.cause.root
                        || child.cause.depth != pending.cause.depth
                        || child.cause.started_at != pending.cause.started_at
                        || child.service_key != pending.service_key
                        || child.definition != pending.definition
                        || child.version != pending.version
                        || !matches!(child.payload, WorkPayload::Action(_))
                }) {
                    return Err(Error::Invalid {
                        kind: "reaction".into(),
                        field: "materialization".into(),
                    });
                }
                self.enqueue(&limits, children)?;
                parent.state = WorkState::Done;
                self.put(parent)?;
                Ok(WorkResult::Changed)
            }
            WorkUpdate::Finish {
                claim,
                now,
                outcome,
            } => {
                let mut record = self.live_claim(&claim, now)?;
                if outcome == WorkOutcome::Retry && requires_reconciliation(&record) {
                    await_reconciliation(&mut record)?;
                    self.put(record)?;
                    return Ok(WorkResult::Changed);
                }
                record.state = match outcome {
                    WorkOutcome::Done => WorkState::Done,
                    WorkOutcome::Stop(reason) => WorkState::Stopped(reason),
                    WorkOutcome::Retry => {
                        let root = self
                            .root(&record.pending.cause.root, record.pending.cause.retry_epoch)?;
                        if root.members == 0 {
                            return Err(Error::Storage);
                        }
                        if let Some(reason) =
                            finish_retry_stop_reason(&record, &limits, root.used, now)
                        {
                            WorkState::Stopped(reason)
                        } else {
                            let delay = limits
                                .retry_seconds
                                .checked_mul(1u64 << record.attempts.saturating_sub(1).min(10))
                                .ok_or(Error::TooLarge)?;
                            record.due = now
                                .checked_add(delay)
                                .ok_or(Error::TooLarge)?
                                .max(record.pending.eligibility_floor());
                            WorkState::Pending
                        }
                    }
                };
                self.put(record)?;
                Ok(WorkResult::Changed)
            }
        }
    }
}
