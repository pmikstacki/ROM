use super::*;
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
    /// Trusted host inspection, bounded by the persisted ledger policy.
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        Err(Error::Unsupported("durable reactions".into()))
    }
    fn capabilities(&self) -> Capabilities;
    fn load(&self, key: &Key) -> Result<Option<Row>>;
    /// Reject overflow without truncation. Charge serialized full Row bytes,
    /// using checked arithmetic before decoding/appending each row.
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>>;
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
