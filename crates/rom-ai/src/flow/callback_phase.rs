//! Ephemeral phase handoffs. No serialization, provider retry, or new durable authority.
use super::{FlowTick, NonacceptanceProof, RunRecord};
use crate::{AttemptEvidence, DispatchOutcome, PreparedAttempt, UsdNanos};

// Deliberately not Clone. In particular, Dispatch is moved only after this callback's START.
pub(super) enum Step {
    Tick(FlowTick),
    Continue(String),
    Start(Box<Start>),
    Dispatch(Box<Dispatch>),
    Outcome(Box<Outcome>),
    RateLimit(Box<RateLimit>),
    Confirm(Box<Confirmation>),
    Settle(Settlement),
    Done,
}
pub(super) struct Start {
    pub(super) record: RunRecord,
    pub(super) revision: u64,
}
pub(super) struct Dispatch {
    pub(super) run: String,
    pub(super) attempt: PreparedAttempt,
    pub(super) now: u64,
}
pub(super) struct Outcome {
    pub(super) run: String,
    pub(super) attempt: PreparedAttempt,
    pub(super) outcome: DispatchOutcome,
}
pub(super) struct RateLimit {
    pub(super) run: String,
    pub(super) attempt: PreparedAttempt,
    pub(super) evidence: AttemptEvidence,
    pub(super) retry_at: u64,
}
pub(super) struct Confirmation {
    pub(super) run: String,
    pub(super) attempt: PreparedAttempt,
    pub(super) evidence: AttemptEvidence,
    pub(super) proof: NonacceptanceProof,
    pub(super) now: u64,
    pub(super) due: u64,
}
pub(super) enum Settlement {
    Record {
        record: Box<RunRecord>,
        confirmed_nonaccepted: Option<UsdNanos>,
    },
    Run {
        run: String,
        confirmed_nonaccepted: Option<UsdNanos>,
    },
}
