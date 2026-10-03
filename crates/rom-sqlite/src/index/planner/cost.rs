//! Checked relative costs. Weights are conservative heuristics, not nanoseconds.
use crate::query_observation::ProbeMetrics;
use rom::{QueryCost, QueryEstimates, ReadBinding};

pub(super) fn estimates(
    binding: ReadBinding,
    rows: u64,
    bytes: u64,
    candidates: u64,
    probes: &ProbeMetrics,
) -> Option<QueryEstimates> {
    let candidate_bytes = if rows == 0 {
        0
    } else {
        u64::try_from((u128::from(bytes) * u128::from(candidates)).div_ceil(u128::from(rows)))
            .ok()?
    };
    // Completed probes are shared sunk work, charged equally to both choices.
    let probe_cost = u64::try_from(probes.statements)
        .ok()?
        .checked_mul(64)?
        .checked_add(u64::try_from(probes.rows).ok()?)?;
    Some(QueryEstimates {
        binding,
        complete_candidates: true,
        reference: QueryCost {
            startup: 16u64.checked_add(probe_cost)?,
            rows,
            per_row: 100,
            bytes,
            per_byte: 1,
        },
        native: QueryCost {
            startup: 128u64.checked_add(probe_cost)?,
            rows: candidates,
            per_row: 200,
            bytes: candidate_bytes,
            per_byte: 2,
        },
    })
}
