use super::*;
mod maintenance;
mod operator;
mod receipt;
pub use operator::{StorageWorkControl, WorkControlDecision};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Key {
    pub kind: String,
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub key: Key,
    pub revision: u64,
    pub value: Option<Value>,
    /// Persistence-only policy context. Runtime public outcomes remove this data.
    #[serde(default, skip_serializing_if = "ProtectedMetadata::is_empty")]
    pub protected: ProtectedMetadata,
}
/// Trusted persistence metadata, never projected as Resource fields or public history.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedMetadata {
    /// Last live canonical value, retained only to authorize deletion facts.
    /// Older stored tombstones without this context cannot authorize public
    /// history or receipt disclosure and therefore fail closed. Deletion is
    /// logical removal, not physical erasure of this protected policy context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_authorization: Option<Value>,
    /// Accepted source attribution, committed with the target value and receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_provenance: Option<SourceProvenance>,
}
impl ProtectedMetadata {
    pub fn is_empty(&self) -> bool {
        self.deletion_authorization.is_none() && self.source_provenance.is_none()
    }
}
impl Row {
    pub(crate) fn authorization_value(&self) -> Option<&Value> {
        self.value
            .as_ref()
            .or(self.protected.deletion_authorization.as_ref())
    }
    /// Strip all persistence-only context after current authorization succeeds.
    pub fn public_outcome(mut self) -> Self {
        self.protected = ProtectedMetadata::default();
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    #[serde(default)]
    pub retry_epoch: u64,
    /// Schema version whose codec defines this committed request fingerprint.
    /// Legacy data without this marker uses its unmigrated source catalog version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_version: Option<u32>,
    pub identity: String,
    pub fingerprint: String,
    pub row: Row,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub expected: Option<u64>,
    pub receipt: Receipt,
    pub changed: bool,
    pub effects: Vec<Intent>,
    pub reactions: Vec<PendingWork>,
    pub reaction_limits: Option<ReactionLimits>,
    pub completed_work: Option<(ClaimKey, u64)>,
}
#[derive(Clone, Copy)]
pub struct Capabilities {
    pub atomic_bundle: bool,
    pub snapshots: bool,
    pub effects: bool,
}
/// Single ROM owner per adapter. load/snapshot are authoritative; commit MUST conditionally
/// arbitrate expected revision and identity and atomically persist row/event/receipt/effects.
/// Implementations must not claim rollback for uncertain acknowledgment.
pub trait Storage: Send + Sync + 'static {
    /// Claim this backing store for one Runtime until its final handle and work drop.
    /// Transparent wrappers must forward this method to their underlying store.
    fn acquire_owner(&self) -> Result<StorageOwner> {
        Err(Error::Unsupported("Runtime storage ownership".into()))
    }
    /// Persisted retry admission and replay boundaries. Legacy adapters use epoch zero.
    fn retry_epochs(&self) -> Result<RetryEpochs> {
        Ok(RetryEpochs::default())
    }
    /// Bind the persisted schema before intake. Existing kinds must match exactly.
    /// Omitted kinds remain in the catalog and retain their integrity obligations.
    /// New commits require registration; matching receipts remain replayable.
    fn register(&self, _descriptors: &[Descriptor]) -> Result<()> {
        Err(Error::Unsupported("persisted Resource schema".into()))
    }
    fn supports_reactions(&self) -> bool {
        false
    }
    fn reaction_update(&self, _update: WorkUpdate) -> Result<WorkResult> {
        Err(Error::Unsupported("durable reactions".into()))
    }
    /// Apply at most 32 ordered updates in one durable transaction.
    /// Each update observes preceding transaction-local changes, including revisions,
    /// root accounting and candidate selection. Return results only after commit acknowledgement.
    /// Unsupported and confirmed precommit failures publish no part of the batch.
    /// Unknown means the whole transaction may have committed; never assume rollback.
    /// For multiple updates, native overrides must bound encoded input with
    /// validate_work_update_batch and retain the persisted Work policy,
    /// live-claim fences and per-update validation. Empty and singleton inputs
    /// preserve the default behavior, including singleton limits and errors.
    /// The default returns empty for empty input, delegates a singleton unchanged,
    /// and rejects multiple updates before mutation. It does not emulate atomicity
    /// through separately committed updates. Transparent wrappers should forward this method.
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        crate::reaction_work::batch::default_atomic_updates(self, updates)
    }
    /// Acknowledge an ordered prefix of at most 32 claims before callbacks run.
    /// Ordinary Source claims may share a transaction only across distinct roots.
    /// A first Action, Notification or resolution-only claim remains a singleton.
    /// Later barriers remain unclaimed. Changed and Idle return no claims after
    /// acknowledging lifecycle effects. Unknown returns no dispatchable claims.
    /// The default preserves existing adapters through exactly one Claim update.
    fn reaction_claim_prefix(&self, now: u64, max_claims: usize) -> Result<Vec<WorkClaim>> {
        crate::reaction_work::claim_prefix::default_claim_prefix(self, now, max_claims)
    }
    /// Sample the persisted claim generation, lease deadline, eligibility and
    /// retry epoch through a bounded keyed read without changing durable state.
    /// None means this optional read is unavailable. Runtime compatibility then
    /// permits only a freshly acknowledged singleton, never a retained group.
    /// A successful read does not prevent a concurrent reclaim after the sample.
    /// Transparent wrappers must forward this method together with claim prefixes.
    fn reaction_claim_live(&self, _claim: &ClaimKey, _now: u64) -> Result<Option<bool>> {
        Ok(None)
    }
    /// Trusted host inspection, bounded by the persisted ledger policy.
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        Err(Error::Unsupported("durable reactions".into()))
    }
    /// True only when bounded operator snapshots and atomic controls are implemented.
    fn supports_operator(&self) -> bool {
        false
    }
    /// Coherent trusted ledger data, bounded before projection.
    fn work_snapshot(&self, _max_records: usize, _max_bytes: usize) -> Result<StorageWorkSnapshot> {
        Err(Error::Unsupported("operator work snapshots".into()))
    }
    /// Atomically arbitrate CAS and idempotency, publishing work and receipt together.
    fn control_work(&self, _control: &StorageWorkControl) -> Result<WorkControlReceipt> {
        Err(Error::Unsupported("atomic operator work control".into()))
    }
    fn capabilities(&self) -> Capabilities;
    fn load(&self, key: &Key) -> Result<Option<Row>>;
    /// Reject overflow without truncation. Charge serialized full Row bytes,
    /// using checked arithmetic before decoding/appending each row.
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>>;
    /// One coherent owned query read. Release native guards before returning Rows.
    /// Native adapters must preserve whole-kind admission and complete candidate coverage.
    /// Never invoke application policies or codecs inside native locks/transactions.
    /// The default preserves existing adapters through exactly one bounded snapshot.
    fn query_read(&self, request: &StorageQuery, bounds: QueryBounds) -> Result<QueryRead> {
        self.snapshot(&request.descriptor.kind, bounds.max_rows, bounds.max_bytes)
            .map(|rows| QueryRead::Reference { rows })
    }
    fn receipt(&self, identity: &str) -> Result<Option<Receipt>>;
    fn commit(&self, bundle: &Bundle) -> Result<Receipt>;
    /// Capability is availability, never authority to disclose historical facts.
    fn supports_journal(&self) -> bool {
        false
    }
    /// Explicit checkpoint at the current coherent head; never implies facts were processed.
    fn journal_head(&self, _kind: &str) -> Result<JournalCursor> {
        Err(Error::Unsupported("journal".into()))
    }
    /// Read a coherent page ordered by the persisted global commit sequence.
    /// None starts at position zero. Wrong kind/generation, future positions and
    /// positions below the retained floor return HistoryGap. Empty bounds fail.
    /// Charge serialized whole JournalEvent bytes before decoding/appending.
    /// Return at most max_rows matching facts; paginate at whole-event boundaries.
    /// If the next matching event alone exceeds max_bytes, return TooLarge.
    /// Cursor advances across inspected nonmatching facts to the page boundary
    /// or coherent head. Adapters must never silently reset a missing cursor.
    fn journal(
        &self,
        _kind: &str,
        _after: Option<&JournalCursor>,
        _max_rows: usize,
        _max_bytes: usize,
    ) -> Result<JournalPage> {
        Err(Error::Unsupported("journal".into()))
    }
}
