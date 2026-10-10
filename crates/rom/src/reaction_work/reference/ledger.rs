//! Atomic candidate validation and shared lifecycle state transitions.
use super::control::{finish_retry_stop_reason, retry_stop_reason};
use super::delivery_profile::{await_reconciliation, requires_reconciliation};
use super::*;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLedger {
    pub(super) limits: Option<ReactionLimits>,
    pub(super) work: BTreeMap<String, WorkRecord>,
    pub(super) roots: BTreeMap<String, u32>,
}
impl WorkLedger {
    pub(crate) fn validate_archive(&self) -> Result<()> {
        self.root_epochs()?;
        let Some(limits) = &self.limits else {
            return if self.work.is_empty() && self.roots.is_empty() {
                Ok(())
            } else {
                Err(Error::Storage)
            };
        };
        limits.validate()?;
        self.check_compatible_capacity()?;
        if self.roots.values().any(|used| *used > limits.max_work) {
            return Err(Error::Storage);
        }
        let mut used = BTreeMap::<String, u32>::new();
        for (id, record) in &self.work {
            if id.is_empty()
                || id != &record.pending.id
                || !self.roots.contains_key(&record.pending.cause.root)
                || (!matches!(record.pending.payload, WorkPayload::Notification { .. })
                    && record.pending.delivery_profile != DeliveryProfile::AtLeastOnce)
                || (record.state == WorkState::AwaitingReconciliation
                    && (!matches!(record.pending.payload, WorkPayload::Notification { .. })
                        || record.pending.delivery_profile
                            != DeliveryProfile::ReconcileBeforeRetry))
                || record
                    .pending
                    .not_before
                    .is_some_and(|floor| record.due < floor)
                || record.attempts > limits.max_attempts
                || record.generation < u64::from(record.attempts)
                || matches!(record.state, WorkState::Leased{generation,..} if generation != record.generation)
            {
                return Err(Error::Storage);
            }
            let sum = used.entry(record.pending.cause.root.clone()).or_default();
            *sum = sum.checked_add(record.attempts).ok_or(Error::TooLarge)?;
        }
        if used != self.roots {
            return Err(Error::Storage);
        }
        Ok(())
    }
    pub(crate) fn prepare_restore(&mut self) -> Result<()> {
        self.validate_archive()?;
        let mut next = self.clone();
        for record in next.work.values_mut() {
            if matches!(record.state, WorkState::Leased { .. }) {
                if requires_reconciliation(record) {
                    await_reconciliation(record)?;
                } else {
                    record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
                    record.state = WorkState::Pending;
                    record.due = record.pending.eligibility_floor();
                }
            }
        }
        self.publish(next)
    }
    pub fn records(&self) -> Vec<WorkRecord> {
        self.work.values().cloned().collect()
    }
    pub fn enqueue(&mut self, limits: &ReactionLimits, work: Vec<PendingWork>) -> Result<()> {
        limits.validate()?;
        self.check_compatible_capacity()?;
        if self.limits.as_ref().is_some_and(|prior| prior != limits) {
            return Err(Error::Unsupported(
                "reaction policy differs from persisted policy".into(),
            ));
        }
        let mut next = self.clone();
        next.limits = Some(limits.clone());
        for pending in work {
            if !matches!(pending.payload, WorkPayload::Notification { .. })
                && pending.delivery_profile != DeliveryProfile::AtLeastOnce
            {
                return Err(Error::Conflict);
            }
            if let Some(prior) = next.work.get(&pending.id) {
                if prior.pending != pending {
                    return Err(Error::IdentityMismatch);
                }
                continue;
            }
            next.roots.entry(pending.cause.root.clone()).or_insert(0);
            next.work.insert(
                pending.id.clone(),
                WorkRecord {
                    due: pending.eligibility_floor(),
                    delivery: None,
                    pending,
                    state: WorkState::Pending,
                    attempts: 0,
                    generation: 0,
                    revision: 0,
                },
            );
        }
        next.root_epochs()?;
        next.check_bounds()?;
        *self = next;
        Ok(())
    }
    /// Frozen payloads and record/root keys do not change during lifecycle updates.
    /// Reserve maximum numeric widths and the largest serialized state/outcome shapes
    /// even for terminal records, so later progress never competes with new admission.
    fn reserved_bytes(&self) -> Result<usize> {
        let mut reserved = self.clone();
        for count in reserved.roots.values_mut() {
            *count = u32::MAX;
        }
        for record in reserved.work.values_mut() {
            record.attempts = u32::MAX;
            record.generation = u64::MAX;
            record.revision = u64::MAX;
            record.due = u64::MAX;
            record.state = WorkState::Leased {
                until: u64::MAX,
                generation: u64::MAX,
                resolution_only: Some(StopReason::DefinitionChanged),
            };
            record.delivery = Some(DeliveryOutcome::Retryable);
        }
        Ok(serde_json::to_vec(&reserved)
            .map_err(|_| Error::Storage)?
            .len())
    }
    /// Pre-reservation experimental data is compatible only when its frozen payloads
    /// already leave full lifecycle headroom under the persisted limit. Never raise it.
    pub(crate) fn check_compatible_capacity(&self) -> Result<()> {
        match self.check_bounds() {
            Err(Error::Overloaded)=>Err(Error::Unsupported("persisted work ledger lacks lifecycle capacity; automatic migration is unavailable".into())),
            result=>result,
        }
    }
    pub(super) fn check_bounds(&self) -> Result<()> {
        if let Some(l) = &self.limits
            && (self.work.len() > l.max_records
                || self.reserved_bytes()? > l.max_bytes
                || serde_json::to_vec(self).map_err(|_| Error::Storage)?.len() > l.max_bytes)
        {
            return Err(Error::Overloaded);
        }
        Ok(())
    }
    pub fn apply(&mut self, update: WorkUpdate) -> Result<WorkResult> {
        self.check_compatible_capacity()?;
        let mut next = self.clone();
        let mut result = next.update(update)?;
        let next = self.prepare_candidate(next)?;
        if let WorkResult::Claimed(claim) = &mut result {
            claim.work = next
                .work
                .get(&claim.work.pending.id)
                .ok_or(Error::Storage)?
                .clone();
        }
        *self = next;
        Ok(result)
    }
    /// Publish a fully validated candidate, versioning each changed existing record once.
    /// Nested transitions and newly admitted records never consume a second revision.
    pub(super) fn publish(&mut self, next: Self) -> Result<()> {
        *self = self.prepare_candidate(next)?;
        Ok(())
    }
    fn prepare_candidate(&self, mut next: Self) -> Result<Self> {
        for (id, record) in &mut next.work {
            if let Some(prior) = self.work.get(id)
                && record != prior
            {
                record.revision = prior.revision.checked_add(1).ok_or(Error::TooLarge)?;
            }
        }
        next.check_bounds()?;
        Ok(next)
    }
    pub(super) fn validate_claim(&self, claim: &ClaimKey, now: u64) -> Result<&WorkRecord> {
        let record = self.work.get(&claim.id).ok_or(Error::Missing)?;
        if now < record.pending.eligibility_floor()
            || !matches!(record.state,WorkState::Leased{until,generation,..} if until>now&&generation==claim.generation)
        {
            return Err(Error::Conflict);
        }
        Ok(record)
    }
    fn update(&mut self, update: WorkUpdate) -> Result<WorkResult> {
        let Some(l) = self.limits.clone() else {
            return Ok(WorkResult::Idle);
        };
        match update {
            WorkUpdate::DeliveryStarted { claim, now } => {
                let record = self.validate_claim(&claim, now)?;
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
                self.work.get_mut(&claim.id).ok_or(Error::Storage)?.delivery =
                    Some(DeliveryOutcome::Unknown);
                Ok(WorkResult::Changed)
            }
            WorkUpdate::DeliveryFinished {
                claim,
                now,
                outcome,
            } => {
                let record = self.validate_claim(&claim, now)?;
                if !matches!(record.pending.payload, WorkPayload::Notification { .. }) {
                    return Err(Error::Conflict);
                }
                let record = self.work.get_mut(&claim.id).ok_or(Error::Storage)?;
                record.delivery = Some(outcome.clone());
                let finish = if requires_reconciliation(record) {
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
                self.update(WorkUpdate::Finish {
                    claim,
                    now,
                    outcome: finish,
                })
            }

            WorkUpdate::Claim { now } => {
                let mut changed = false;
                for record in self.work.values_mut() {
                    if record.state == WorkState::Pending
                        && record.due > now
                        && now.saturating_sub(record.pending.cause.started_at) >= l.max_age_seconds
                    {
                        record.state = WorkState::Stopped(StopReason::Age);
                        changed = true;
                        continue;
                    }
                    let eligible = match record.state {
                        WorkState::Pending => record.due <= now,
                        WorkState::Leased { until, .. } => until <= now,
                        _ => false,
                    };
                    if !eligible {
                        continue;
                    }
                    if requires_reconciliation(record) {
                        await_reconciliation(record)?;
                        changed = true;
                        continue;
                    }
                    let root = self
                        .roots
                        .get_mut(&record.pending.cause.root)
                        .ok_or(Error::Storage)?;
                    let reason = retry_stop_reason(record, &l, *root, now);
                    if reason.is_none() {
                        *root = root.checked_add(1).ok_or(Error::TooLarge)?;
                        record.attempts = record.attempts.checked_add(1).ok_or(Error::TooLarge)?;
                    }
                    record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
                    record.state = WorkState::Leased {
                        until: now.checked_add(l.lease_seconds).ok_or(Error::TooLarge)?,
                        generation: record.generation,
                        resolution_only: reason.clone(),
                    };
                    return Ok(WorkResult::Claimed(Box::new(WorkClaim {
                        work: record.clone(),
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
                let parent = self.validate_claim(&claim, now)?.clone();
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
                    || children.len() > l.max_fanout
                {
                    return Err(Error::TooLarge);
                }
                if children.iter().any(|c| {
                    c.cause.retry_epoch != parent.pending.cause.retry_epoch
                        || c.cause.root != parent.pending.cause.root
                        || c.cause.depth != parent.pending.cause.depth
                        || c.cause.started_at != parent.pending.cause.started_at
                        || c.service_key != parent.pending.service_key
                        || c.definition != parent.pending.definition
                        || c.version != parent.pending.version
                        || !matches!(c.payload, WorkPayload::Action(_))
                }) {
                    return Err(Error::Invalid {
                        kind: "reaction".into(),
                        field: "materialization".into(),
                    });
                }
                self.enqueue(&l, children)?;
                self.work.get_mut(&claim.id).ok_or(Error::Storage)?.state = WorkState::Done;
                Ok(WorkResult::Changed)
            }
            WorkUpdate::Finish {
                claim,
                now,
                outcome,
            } => {
                self.validate_claim(&claim, now)?;
                let record = self.work.get_mut(&claim.id).ok_or(Error::Storage)?;
                if outcome == WorkOutcome::Retry && requires_reconciliation(record) {
                    await_reconciliation(record)?;
                    return Ok(WorkResult::Changed);
                }
                record.state = match outcome {
                    WorkOutcome::Done => WorkState::Done,
                    WorkOutcome::Stop(reason) => WorkState::Stopped(reason),
                    WorkOutcome::Retry => {
                        let used = *self
                            .roots
                            .get(&record.pending.cause.root)
                            .ok_or(Error::Storage)?;
                        if let Some(reason) = finish_retry_stop_reason(record, &l, used, now) {
                            WorkState::Stopped(reason)
                        } else {
                            let delay = l
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
                Ok(WorkResult::Changed)
            }
        }
    }
}
