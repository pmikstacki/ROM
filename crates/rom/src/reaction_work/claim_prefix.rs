//! Compatible singleton admission for adapters without grouped claim support.
use super::*;

/// Check one persisted claim through the adapter's bounded transaction reader.
/// Missing, expired, ineligible and replaced generations cannot dispatch.
pub fn claim_live<R: incremental::WorkRead>(reader: &R, key: &ClaimKey, now: u64) -> Result<bool> {
    let header = reader.header()?;
    let Some(record) = reader.record(&key.id)? else {
        return Ok(false);
    };
    if record.pending.id != key.id {
        return Err(Error::Storage);
    }
    if now < record.pending.eligibility_floor()
        || record.generation != key.generation
        || !matches!(record.state, WorkState::Leased { until, generation, .. } if until > now && generation == key.generation)
    {
        return Ok(false);
    }
    match header
        .retry_epochs
        .check(record.pending.cause.retry_epoch, false, true)
    {
        Ok(()) => Ok(true),
        Err(Error::IdentityExpired) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Whether a prepared claim can extend an ordinary Source prefix.
/// False means a singleton when the prefix is empty, or a barrier otherwise.
/// Call before applying the candidate delta so withheld work keeps its budget.
pub fn claim_prefix_accepts_source(previous: &[WorkClaim], claim: &WorkClaim) -> bool {
    !claim.resolution_only
        && matches!(claim.work.pending.payload, WorkPayload::Source(_))
        && !previous
            .iter()
            .any(|old| old.work.pending.cause.root == claim.work.pending.cause.root)
}

pub(crate) fn default_claim_prefix<S: Storage + ?Sized>(
    storage: &S,
    now: u64,
    max_claims: usize,
) -> Result<Vec<WorkClaim>> {
    if max_claims == 0 || max_claims > 32 {
        return Err(Error::TooLarge);
    }
    match storage.reaction_update(WorkUpdate::Claim { now })? {
        WorkResult::Claimed(claim) => Ok(vec![*claim]),
        WorkResult::Changed | WorkResult::Idle => Ok(vec![]),
    }
}
