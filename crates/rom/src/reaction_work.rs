//! Shared durable work state machine. Adapters apply each update in one native transaction.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionLimits {
    pub max_depth: u32,
    pub max_work: u32,
    pub max_attempts: u32,
    pub max_age_seconds: u64,
    pub retry_seconds: u64,
    pub lease_seconds: u64,
    pub max_fanout: usize,
    pub max_records: usize,
    pub max_bytes: usize,
}
impl Default for ReactionLimits {
    fn default() -> Self {
        Self {
            max_depth: 16,
            max_work: 256,
            max_attempts: 3,
            max_age_seconds: 3600,
            retry_seconds: 1,
            lease_seconds: 30,
            max_fanout: 16,
            max_records: 1024,
            max_bytes: 1024 * 1024,
        }
    }
}
impl ReactionLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_depth == 0
            || self.max_work == 0
            || self.max_attempts == 0
            || self.max_age_seconds == 0
            || self.retry_seconds == 0
            || self.lease_seconds == 0
            || self.max_fanout == 0
            || self.max_records == 0
            || self.max_bytes == 0
        {
            return Err(Error::Unsupported("reaction limits must be nonzero".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    pub root: String,
    pub parent: Option<String>,
    pub depth: u32,
    pub started_at: u64,
    pub path: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkPayload {
    Source(Row),
    Action(Value),
    Notification { source: Row, payload: Value },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingWork {
    pub id: String,
    pub cause: Cause,
    pub definition: String,
    pub version: u32,
    pub service_key: String,
    pub payload: WorkPayload,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    Depth,
    WorkBudget,
    Attempts,
    Age,
    Fanout,
    Denied,
    Conflict,
    Invalid,
    Missing,
    DefinitionChanged,
    CallbackPanicked,
    Unavailable,
    DeliveryPermanent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkState {
    Pending,
    Leased {
        until: u64,
        generation: u64,
        resolution_only: Option<StopReason>,
    },
    Done,
    Stopped(StopReason),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRecord {
    pub pending: PendingWork,
    pub state: WorkState,
    pub attempts: u32,
    pub generation: u64,
    pub due: u64,
    #[serde(default)]
    pub delivery: Option<DeliveryOutcome>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimKey {
    pub id: String,
    pub generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkClaim {
    pub work: WorkRecord,
    pub resolution_only: bool,
    pub stop_reason: Option<StopReason>,
}
impl WorkClaim {
    pub fn key(&self) -> ClaimKey {
        ClaimKey {
            id: self.work.pending.id.clone(),
            generation: self.work.generation,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkOutcome {
    Done,
    Retry,
    Stop(StopReason),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkUpdate {
    DeliveryStarted {
        claim: ClaimKey,
        now: u64,
    },
    DeliveryFinished {
        claim: ClaimKey,
        now: u64,
        outcome: DeliveryOutcome,
    },
    Claim {
        now: u64,
    },
    Materialize {
        claim: ClaimKey,
        now: u64,
        children: Vec<PendingWork>,
    },
    Finish {
        claim: ClaimKey,
        now: u64,
        outcome: WorkOutcome,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkResult {
    Idle,
    Claimed(Box<WorkClaim>),
    Changed,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLedger {
    limits: Option<ReactionLimits>,
    work: BTreeMap<String, WorkRecord>,
    roots: BTreeMap<String, u32>,
}
impl WorkLedger {
    pub(crate) fn validate_archive(&self) -> Result<()> {
        let Some(limits) = &self.limits else {
            return if self.work.is_empty() && self.roots.is_empty() {
                Ok(())
            } else {
                Err(Error::Storage)
            };
        };
        limits.validate()?;
        self.check_bounds()?;
        if self.roots.values().any(|used| *used > limits.max_work) {
            return Err(Error::Storage);
        }
        let mut used = BTreeMap::<String, u32>::new();
        for (id, record) in &self.work {
            if id.is_empty()
                || id != &record.pending.id
                || !self.roots.contains_key(&record.pending.cause.root)
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
        for record in self.work.values_mut() {
            if matches!(record.state, WorkState::Leased { .. }) {
                record.generation = record.generation.checked_add(1).ok_or(Error::TooLarge)?;
                record.state = WorkState::Pending;
                record.due = 0;
            }
        }
        self.check_bounds()
    }
    pub fn records(&self) -> Vec<WorkRecord> {
        self.work.values().cloned().collect()
    }
    pub fn enqueue(&mut self, limits: &ReactionLimits, work: Vec<PendingWork>) -> Result<()> {
        limits.validate()?;
        if self.limits.as_ref().is_some_and(|prior| prior != limits) {
            return Err(Error::Unsupported(
                "reaction policy differs from persisted policy".into(),
            ));
        }
        let mut next = self.clone();
        next.limits = Some(limits.clone());
        for pending in work {
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
                    due: pending.cause.started_at,
                    delivery: None,
                    pending,
                    state: WorkState::Pending,
                    attempts: 0,
                    generation: 0,
                },
            );
        }
        next.check_bounds()?;
        *self = next;
        Ok(())
    }
    fn check_bounds(&self) -> Result<()> {
        if let Some(l) = &self.limits
            && (self.work.len() > l.max_records
                || serde_json::to_vec(self).map_err(|_| Error::Storage)?.len() > l.max_bytes)
        {
            return Err(Error::Overloaded);
        }
        Ok(())
    }
    pub fn apply(&mut self, update: WorkUpdate) -> Result<WorkResult> {
        let mut next = self.clone();
        let result = next.update(update)?;
        next.check_bounds()?;
        *self = next;
        Ok(result)
    }
    fn validate_claim(&self, claim: &ClaimKey, now: u64) -> Result<&WorkRecord> {
        let record = self.work.get(&claim.id).ok_or(Error::Missing)?;
        if !matches!(record.state,WorkState::Leased{until,generation,..} if until>now&&generation==claim.generation)
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
                let finish = match outcome {
                    DeliveryOutcome::Accepted => WorkOutcome::Done,
                    DeliveryOutcome::Permanent => WorkOutcome::Stop(StopReason::DeliveryPermanent),
                    DeliveryOutcome::Panicked => WorkOutcome::Stop(StopReason::CallbackPanicked),
                    DeliveryOutcome::Unknown
                    | DeliveryOutcome::Retryable
                    | DeliveryOutcome::TimedOut => WorkOutcome::Retry,
                };
                self.work.get_mut(&claim.id).ok_or(Error::Storage)?.delivery = Some(outcome);
                self.update(WorkUpdate::Finish {
                    claim,
                    now,
                    outcome: finish,
                })
            }

            WorkUpdate::Claim { now } => {
                for record in self.work.values_mut() {
                    let eligible = match record.state {
                        WorkState::Pending => record.due <= now,
                        WorkState::Leased { until, .. } => until <= now,
                        _ => false,
                    };
                    if !eligible {
                        continue;
                    }
                    let root = self
                        .roots
                        .get_mut(&record.pending.cause.root)
                        .ok_or(Error::Storage)?;
                    let reason = if record.pending.cause.depth > l.max_depth {
                        Some(StopReason::Depth)
                    } else if now.saturating_sub(record.pending.cause.started_at)
                        >= l.max_age_seconds
                    {
                        Some(StopReason::Age)
                    } else if record.attempts >= l.max_attempts {
                        Some(StopReason::Attempts)
                    } else if *root >= l.max_work {
                        Some(StopReason::WorkBudget)
                    } else {
                        None
                    };
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
                Ok(WorkResult::Idle)
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
                    c.cause.root != parent.pending.cause.root
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
                record.state = match outcome {
                    WorkOutcome::Done => WorkState::Done,
                    WorkOutcome::Stop(reason) => WorkState::Stopped(reason),
                    WorkOutcome::Retry => {
                        let used = *self
                            .roots
                            .get(&record.pending.cause.root)
                            .ok_or(Error::Storage)?;
                        if record.attempts >= l.max_attempts {
                            WorkState::Stopped(StopReason::Attempts)
                        } else if used >= l.max_work {
                            WorkState::Stopped(StopReason::WorkBudget)
                        } else if now.saturating_sub(record.pending.cause.started_at)
                            >= l.max_age_seconds
                        {
                            WorkState::Stopped(StopReason::Age)
                        } else {
                            let delay = l
                                .retry_seconds
                                .checked_mul(1u64 << record.attempts.saturating_sub(1).min(10))
                                .ok_or(Error::TooLarge)?;
                            record.due = now.checked_add(delay).ok_or(Error::TooLarge)?;
                            WorkState::Pending
                        }
                    }
                };
                Ok(WorkResult::Changed)
            }
        }
    }
}
