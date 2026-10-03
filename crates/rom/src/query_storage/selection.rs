//! Pure capability, freshness and checked-cost selection. No application callbacks.
use super::protocol::*;
use crate::Shape;

pub(super) fn valid_snapshot(snapshot: &QuerySnapshot) -> bool {
    !snapshot.store.is_empty()
        && snapshot.store.len() <= 256
        && snapshot.profile_version == QUERY_PROFILE_VERSION
        && snapshot.encoding_version == QUERY_ENCODING_VERSION
}
pub(crate) fn scalar_shape(shape: &Shape) -> bool {
    match shape {
        Shape::Optional(inner) | Shape::Nullable(inner) => scalar_shape(inner),
        Shape::List(_) | Shape::Map(_) => false,
        _ => true,
    }
}
pub(super) fn native_eligible(request: &StorageQuery) -> bool {
    request.selection == SelectionMode::UniformReadAndFields
        && request.semantics == QUERY_SEMANTICS_VERSION
        && !request.descriptor.kind.is_empty()
        && request.descriptor.version != 0
        && request
            .spec
            .filters
            .iter()
            .map(|f| &f.field)
            .chain(request.spec.comparisons.iter().map(|c| &c.field))
            .chain(request.spec.order.iter().map(|o| &o.field))
            .all(|name| {
                request
                    .descriptor
                    .fields
                    .iter()
                    .find(|f| &f.name == name)
                    .is_some_and(|field| scalar_shape(&field.shape))
            })
}
fn score(cost: QueryCost) -> Option<u64> {
    cost.startup
        .checked_add(cost.rows.checked_mul(cost.per_row)?)?
        .checked_add(cost.bytes.checked_mul(cost.per_byte)?)
}
/// Optional unsupported, stale or overflowing estimates choose the reference path.
/// This function never performs admission, authority checks or physical execution.
/// The caller supplies the identity from its actual active read transaction.
pub fn select_query_strategy(
    request: &StorageQuery,
    snapshot: &QuerySnapshot,
    estimates: Option<&QueryEstimates>,
) -> QueryStrategy {
    let Some(estimates) = estimates else {
        return QueryStrategy::Reference;
    };
    if !native_eligible(request)
        || !valid_snapshot(snapshot)
        || estimates.binding.request != *request
        || estimates.binding.snapshot != *snapshot
        || !estimates.complete_candidates
    {
        return QueryStrategy::Reference;
    }
    match (score(estimates.reference), score(estimates.native)) {
        (Some(reference), Some(native)) if native < reference => QueryStrategy::NativeCandidates,
        _ => QueryStrategy::Reference,
    }
}
