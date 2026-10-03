//! Backend-neutral values for one owned, coherent query read.
use crate::{Descriptor, QuerySpec, Row};

pub const QUERY_SEMANTICS_VERSION: u32 = 1;
pub const QUERY_PROFILE_VERSION: u32 = 1;
pub const QUERY_ENCODING_VERSION: u32 = 1;

/// Native eligibility is established by core, never inferred by an adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    ReferenceOnly,
    UniformReadAndFields,
}
/// Normalized request supplied by core. Constructing this value does not normalize or authorize it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageQuery {
    pub spec: QuerySpec,
    pub descriptor: Descriptor,
    pub semantics: u32,
    pub selection: SelectionMode,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryBounds {
    pub max_rows: usize,
    pub max_bytes: usize,
}
/// Exact whole-kind admission, including tombstones and protected full-Row metadata.
/// Stored-text bytes and canonical serialized Row bytes need not be equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindAdmission {
    pub rows: usize,
    pub persisted_bytes: usize,
    pub canonical_bytes: usize,
}
/// Adapter-owned identity of one coherent read snapshot, not an authority token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuerySnapshot {
    pub store: String,
    pub generation: u64,
    pub profile_version: u32,
    pub encoding_version: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadBinding {
    pub request: StorageQuery,
    pub snapshot: QuerySnapshot,
}
/// Rows must be fully materialized and native guards released before returning.
/// Native candidates must include every possible match under the exact request.
/// Metadata consistency cannot independently prove completeness or snapshot truth:
/// these remain adapter obligations established by integrity and conformance tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryRead {
    Reference {
        rows: Vec<Row>,
    },
    NativeCandidates {
        rows: Vec<Row>,
        admission: KindAdmission,
        binding: Box<ReadBinding>,
    },
}
/// Costs share adapter-calibrated relative units, not asserted measured latency.
/// Both strategies must account for the same whole operation and observation scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryCost {
    pub startup: u64,
    pub rows: u64,
    pub per_row: u64,
    pub bytes: u64,
    pub per_byte: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryEstimates {
    pub binding: ReadBinding,
    pub complete_candidates: bool,
    pub reference: QueryCost,
    pub native: QueryCost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryStrategy {
    Reference,
    NativeCandidates,
}
