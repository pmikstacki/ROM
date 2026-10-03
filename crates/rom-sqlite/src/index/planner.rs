//! Bounded same-snapshot predicate selection; no application callbacks or Row decoding.
mod cost;
mod predicate;
mod probe;

use crate::query_observation::ProbeMetrics;
use predicate::Predicate;
use rom::{QueryEstimates, ReadBinding, StorageQuery};
use rusqlite::{Connection, types::Value};

#[cfg(test)]
mod tests;

const MAX_PREDICATES: usize = 4;
const INITIAL_CAP: u64 = 16;
const EXPANDED_CAP: u64 = 4096;
const MAX_PROBE_KEYS: usize = MAX_PREDICATES * (INITIAL_CAP as usize + EXPANDED_CAP as usize + 2);

pub(super) struct NativePlan {
    pub sql: String,
    pub parameters: Vec<Value>,
}

impl NativePlan {
    /// Probes and materialization share the caller's admitted read transaction.
    /// A saturated probe changes cost only; forced native may still use the plan.
    pub(super) fn select<const OBSERVED: bool>(
        c: &Connection,
        request: &StorageQuery,
        binding: ReadBinding,
        rows: u64,
        bytes: u64,
    ) -> (Option<(Self, QueryEstimates)>, ProbeMetrics) {
        let mut metrics = ProbeMetrics::default();
        let selected = select::<OBSERVED>(c, request, binding, rows, bytes, &mut metrics);
        (selected, metrics)
    }
}

fn select<const OBSERVED: bool>(
    c: &Connection,
    request: &StorageQuery,
    binding: ReadBinding,
    rows: u64,
    bytes: u64,
    metrics: &mut ProbeMetrics,
) -> Option<(NativePlan, QueryEstimates)> {
    let initial_cap = INITIAL_CAP.min(rows / 2);
    let mut candidates = Vec::new();
    for predicate in Predicate::for_request(request) {
        if let Some(count) = probe::count::<OBSERVED>(c, &predicate, initial_cap, metrics) {
            candidates.push((predicate, count));
            if matches!(count, probe::Count::Exact(0)) {
                break;
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let mut best = smallest_exact(&candidates);
    if best.is_none() {
        for (index, candidate) in candidates.iter_mut().enumerate() {
            let cap = best.map_or(EXPANDED_CAP.min(rows / 2), |(_, count): (usize, u64)| {
                count.saturating_sub(1).min(EXPANDED_CAP).min(rows / 2)
            });
            // Repeating an identical cap cannot refine a saturated count.
            if cap <= initial_cap {
                continue;
            }
            if let Some(count) = probe::count::<OBSERVED>(c, &candidate.0, cap, metrics) {
                candidate.1 = count;
                if let probe::Count::Exact(count) = count
                    && best.is_none_or(|(_, prior)| count < prior)
                {
                    best = Some((index, count));
                }
            }
        }
    }
    let (index, count) = best.unwrap_or((0, rows));
    let plan = candidates.swap_remove(index).0.materialization();
    if !probe::recognized(c, &plan.sql, &plan.parameters, true)? {
        return None;
    }
    let estimates = cost::estimates(binding, rows, bytes, count, metrics)?;
    Some((plan, estimates))
}

fn smallest_exact(candidates: &[(Predicate, probe::Count)]) -> Option<(usize, u64)> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, (_, count))| match count {
            probe::Count::Exact(count) => Some((index, *count)),
            probe::Count::Saturated => None,
        })
        .min_by_key(|(_, count)| *count)
}
