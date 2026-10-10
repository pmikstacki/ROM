//! Durable work identities, limits and lifecycle protocol.
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
    #[serde(default)]
    pub retry_epoch: u64,
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
pub enum DeliveryProfile {
    AtLeastOnce,
    ProviderDeduplicated,
    ReconcileBeforeRetry,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingWork {
    pub id: String,
    pub cause: Cause,
    pub definition: String,
    pub version: u32,
    pub service_key: String,
    pub delivery_profile: DeliveryProfile,
    /// Immutable eligibility floor. It cannot extend the original work budgets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_before: Option<u64>,
    pub payload: WorkPayload,
}
impl PendingWork {
    pub(super) fn eligibility_floor(&self) -> u64 {
        self.cause.started_at.max(self.not_before.unwrap_or(0))
    }
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
    AwaitingReconciliation,
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
    /// Monotonic lifecycle revision, independent of claim fencing generation.
    pub revision: u64,
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
