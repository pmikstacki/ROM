//! Test-support execution controls and scoped native read observations.
#[cfg(feature = "test-support")]
use rom::{Error, QueryBounds, QueryStrategy, StorageQuery};
use rom::{QueryRead, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryExecution {
    Automatic,
    #[cfg(feature = "test-support")]
    Reference,
    #[cfg(feature = "test-support")]
    Native,
}

#[cfg(feature = "test-support")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryMetrics {
    pub strategy: QueryStrategy,
    /// Full stored Row JSON values decoded by the materialization statement.
    pub decoded_rows: usize,
    /// UTF-8 bytes submitted to the native Row JSON decoder, not heap allocation.
    pub decoded_bytes: usize,
    /// SQLite VM steps for materialization only, excluding metadata and EXPLAIN.
    pub vm_steps: u64,
    /// Covering-index probe entries visited; repeated passes count separately.
    pub probe_rows: usize,
    pub probe_statements: usize,
    /// Probe execution only; excludes EXPLAIN and materialization statements.
    pub probe_vm_steps: u64,
    /// Probe execution only; excludes preparation, EXPLAIN and materialization.
    pub probe_elapsed_ns: u128,
}

#[derive(Default)]
pub(crate) struct ProbeMetrics {
    pub rows: usize,
    pub statements: usize,
    #[cfg(feature = "test-support")]
    pub vm_steps: u64,
    #[cfg(feature = "test-support")]
    pub elapsed_ns: u128,
}

#[derive(Debug)]
pub struct QueryObservation {
    pub read: QueryRead,
    #[cfg(feature = "test-support")]
    pub metrics: QueryMetrics,
}

#[cfg(feature = "test-support")]
impl crate::Sqlite {
    /// Measurement control only. Native bypasses cost comparison, never admission,
    /// semantic eligibility, snapshot binding or physical plan recognition. If no
    /// permitted native plan exists, metrics report the actual reference strategy.
    pub fn query_read_observed(
        &self,
        request: &StorageQuery,
        bounds: QueryBounds,
        execution: QueryExecution,
    ) -> Result<QueryObservation> {
        let mut connection = self.connection.lock().map_err(|_| Error::Panicked)?;
        let transaction = connection.transaction().map_err(|_| Error::Storage)?;
        crate::index::query_read_observed(&transaction, request, bounds, execution)
    }
}

pub(crate) fn observation(
    read: QueryRead,
    materialization: crate::read_rows::Metrics,
    probes: ProbeMetrics,
) -> Result<QueryObservation> {
    #[cfg(feature = "test-support")]
    let metrics = QueryMetrics {
        strategy: match &read {
            QueryRead::Reference { .. } => QueryStrategy::Reference,
            QueryRead::NativeCandidates { .. } => QueryStrategy::NativeCandidates,
        },
        decoded_rows: materialization.decoded_rows,
        decoded_bytes: materialization.decoded_bytes,
        vm_steps: materialization.vm_steps,
        probe_rows: probes.rows,
        probe_statements: probes.statements,
        probe_vm_steps: probes.vm_steps,
        probe_elapsed_ns: probes.elapsed_ns,
    };
    #[cfg(not(feature = "test-support"))]
    let _ = (materialization, probes);
    Ok(QueryObservation {
        read,
        #[cfg(feature = "test-support")]
        metrics,
    })
}
