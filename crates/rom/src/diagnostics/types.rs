//! Fixed redacted records and bounded outcome categories.
use serde::Serialize;

/// An opaque diagnostic link. It is neither an identity nor an authorization grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DiagnosticToken(pub(crate) [u8; 32]);

/// A diagnostic stage; records are lossy and cannot substitute for durable receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DiagnosticStage {
    Admission,
    Authorization,
    Resolve,
    StorageRead,
    Validation,
    Execution,
    Commit,
    Receipt,
    Event,
    Reaction,
    Materialize,
    DeliveryStart,
    ExternalAttempt,
    DeliveryFinish,
    Work,
    Shutdown,
}

/// Category only: no application error text, payload or principal is included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DiagnosticOutcome {
    Started,
    Succeeded,
    Replay,
    Denied,
    Invalid,
    Conflict,
    NotCommitted,
    Unknown,
    Panicked,
    TimedOut,
    Retryable,
    Permanent,
    Cancelled,
    Stopped,
}

/// Fixed-size, payload-free observation of owned execution or durable work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DiagnosticEvent {
    pub version: u16,
    pub session: [u8; 16],
    /// Allocation sequence within this host stream. Concurrent receive order can differ.
    /// Loss creates gaps. This does not establish business journal order.
    pub sequence: u64,
    pub operation: DiagnosticToken,
    pub root: DiagnosticToken,
    pub parent: Option<DiagnosticToken>,
    pub work: Option<DiagnosticToken>,
    /// Generation-sensitive claim link; the work link remains stable across leases.
    pub claim: Option<DiagnosticToken>,
    pub stage: DiagnosticStage,
    pub outcome: DiagnosticOutcome,
    pub attempt: u32,
    pub depth: u32,
    /// Checked local elapsed time. It cannot be compared across process restarts.
    pub elapsed_ns: Option<u64>,
    /// Checked supervised stage interval, when measured. Reaction includes pool queue and return transit.
    pub stage_elapsed_ns: Option<u64>,
    pub event_count: u32,
}

/// Independent saturating counters. Sampling is advisory under concurrent publication.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DiagnosticStats {
    pub enqueued: u64,
    pub dropped_full: u64,
    pub dropped_closed: u64,
    pub dropped_collector: u64,
    pub dropped_sequence: u64,
}
