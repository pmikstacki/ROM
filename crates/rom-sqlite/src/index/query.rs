//! One coherent native query operation. No application callback runs in this module.
use super::{metadata, planner::NativePlan};
use crate::query_observation::{QueryExecution, QueryObservation, observation};
use rom::{
    Error, KindAdmission, QUERY_ENCODING_VERSION, QUERY_PROFILE_VERSION, QueryBounds, QueryRead,
    QuerySnapshot, QueryStrategy, ReadBinding, Result, Row, StorageQuery, select_query_strategy,
};
use rusqlite::{Connection, params_from_iter};

/// Caller holds one read transaction for metadata, EXPLAIN and row materialization.
/// Atomic index maintenance and startup validation establish candidate completeness;
/// the response metadata alone cannot prove it. There is never a logical SQL LIMIT.
pub(crate) fn query_read(
    c: &Connection,
    request: &StorageQuery,
    bounds: QueryBounds,
) -> Result<QueryRead> {
    Ok(execute::<false>(c, request, bounds, QueryExecution::Automatic)?.read)
}

#[cfg(feature = "test-support")]
pub(crate) fn query_read_observed(
    c: &Connection,
    request: &StorageQuery,
    bounds: QueryBounds,
    execution: QueryExecution,
) -> Result<QueryObservation> {
    execute::<true>(c, request, bounds, execution)
}

fn execute<const OBSERVED: bool>(
    c: &Connection,
    request: &StorageQuery,
    bounds: QueryBounds,
    execution: QueryExecution,
) -> Result<QueryObservation> {
    let kind = &request.descriptor.kind;
    let descriptor = metadata::descriptor(c, kind)?;
    if request.descriptor.canonical().map_err(|_| Error::Storage)? != descriptor {
        return Err(Error::Storage);
    }
    let counters = metadata::load(c, kind)?.ok_or(Error::Storage)?;
    let admission = KindAdmission {
        rows: usize::try_from(counters.rows).map_err(|_| Error::TooLarge)?,
        persisted_bytes: usize::try_from(counters.bytes).map_err(|_| Error::TooLarge)?,
        canonical_bytes: usize::try_from(counters.canonical_bytes).map_err(|_| Error::TooLarge)?,
    };
    if admission.rows > admission.persisted_bytes
        || admission.rows > admission.canonical_bytes
        || (admission.rows == 0
            && (admission.persisted_bytes != 0 || admission.canonical_bytes != 0))
    {
        return Err(Error::Storage);
    }
    if admission.rows > bounds.max_rows
        || admission.persisted_bytes > bounds.max_bytes
        || admission.canonical_bytes > bounds.max_bytes
    {
        return Err(Error::TooLarge);
    }
    let snapshot = QuerySnapshot {
        store: metadata::profile(c)?,
        generation: counters.generation,
        profile_version: QUERY_PROFILE_VERSION,
        encoding_version: QUERY_ENCODING_VERSION,
    };
    let reference = |probes| {
        let (rows, metrics) =
            crate::read_rows::snapshot::<OBSERVED>(c, kind, bounds.max_rows, bounds.max_bytes)?;
        observation(QueryRead::Reference { rows }, metrics, probes)
    };
    #[cfg(feature = "test-support")]
    if execution == QueryExecution::Reference {
        return reference(Default::default());
    }
    #[cfg(not(feature = "test-support"))]
    let _ = execution;
    // Avoid optional index work when the request cannot permit native execution.
    // The shared selector still owns the complete eligibility/binding check.
    if request.selection != rom::SelectionMode::UniformReadAndFields
        || request.semantics != rom::QUERY_SEMANTICS_VERSION
    {
        return reference(Default::default());
    }
    let binding = ReadBinding {
        request: request.clone(),
        snapshot: snapshot.clone(),
    };
    let (selected, probes) = NativePlan::select::<OBSERVED>(
        c,
        request,
        binding.clone(),
        counters.rows,
        counters.canonical_bytes,
    );
    let Some((plan, estimates)) = selected else {
        return reference(probes);
    };
    #[cfg(feature = "test-support")]
    let estimates = {
        let mut estimate = estimates;
        if execution == QueryExecution::Native {
            // The same shared gate still checks semantic capability, completeness
            // and exact binding. Only the relative cost comparison is overridden.
            estimate.reference = rom::QueryCost {
                startup: 1,
                rows: 0,
                per_row: 0,
                bytes: 0,
                per_byte: 0,
            };
            estimate.native = rom::QueryCost {
                startup: 0,
                rows: 0,
                per_row: 0,
                bytes: 0,
                per_byte: 0,
            };
        }
        estimate
    };
    if select_query_strategy(request, &snapshot, Some(&estimates))
        != QueryStrategy::NativeCandidates
    {
        return reference(probes);
    }
    // After selection, execution errors propagate. A second read would hide an
    // integrity error and would no longer describe the selected operation.
    let (rows, metrics) = candidates::<OBSERVED>(c, &plan, kind, admission)?;
    observation(
        QueryRead::NativeCandidates {
            rows,
            admission,
            binding: Box::new(binding),
        },
        metrics,
        probes,
    )
}

fn candidates<const OBSERVED: bool>(
    c: &Connection,
    plan: &NativePlan,
    kind: &str,
    admission: KindAdmission,
) -> Result<(Vec<Row>, crate::read_rows::Metrics)> {
    let mut statement = c.prepare(&plan.sql).map_err(|_| Error::Storage)?;
    let mut cursor = statement
        .query(params_from_iter(&plan.parameters))
        .map_err(|_| Error::Storage)?;
    let mut result = Vec::new();
    let mut stored_bytes = 0usize;
    let mut canonical_bytes = 0usize;
    while let Some(record) = cursor.next().map_err(|_| Error::Storage)? {
        if result.len() >= admission.rows {
            return Err(Error::Storage);
        }
        let text = record
            .get_ref(2)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        stored_bytes = stored_bytes.checked_add(text.len()).ok_or(Error::Storage)?;
        if stored_bytes > admission.persisted_bytes {
            return Err(Error::Storage);
        }
        let row: Row = serde_json::from_str(text).map_err(|_| Error::Storage)?;
        let native_kind = record
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let native_id = record
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        if row.key.kind != kind
            || native_kind != kind
            || row.key.id != native_id
            || row.value.is_none()
        {
            return Err(Error::Storage);
        }
        canonical_bytes = canonical_bytes
            .checked_add(serde_json::to_vec(&row).map_err(|_| Error::Storage)?.len())
            .ok_or(Error::Storage)?;
        if canonical_bytes > admission.canonical_bytes {
            return Err(Error::Storage);
        }
        result.push(row);
    }
    drop(cursor);
    let metrics =
        crate::read_rows::Metrics::read::<OBSERVED>(result.len(), stored_bytes, &statement)?;
    Ok((result, metrics))
}
