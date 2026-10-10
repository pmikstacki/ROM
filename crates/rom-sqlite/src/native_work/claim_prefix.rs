//! Prepared-result admission: barriers are inspected before native publication.
#[cfg(feature = "test-support")]
use crate::stage_observation::{ClaimPrefixDisposition, ClaimPrefixGuard, ClaimPrefixReason};
#[cfg(feature = "test-support")]
use crate::stage_observation::{StageOperation, StorageStage, Timer, WorkCommitContext};
use crate::{Sqlite, native_work};
#[cfg(not(feature = "test-support"))]
use rom::Storage;
use rom::storage_support::{
    metadata::JournalRead,
    work::{WorkRead, claim_prefix_accepts_source, prepare_update},
};
use rom::{Error, Result, WorkClaim, WorkResult, WorkUpdate};

pub(crate) fn live(storage: &Sqlite, claim: &rom::ClaimKey, now: u64) -> Result<bool> {
    let mut connection = storage.connection.lock().map_err(|_| Error::Panicked)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .map_err(|_| Error::Storage)?;
    let reader = native_work::Reader::bounded(&tx, storage.validation_limits);
    if JournalRead::header(&reader)?.retry_epochs() != WorkRead::header(&reader)?.retry_epochs {
        return Err(Error::Storage);
    }
    rom::storage_support::work::claim_live(&reader, claim, now)
}

pub(crate) fn claim(storage: &Sqlite, now: u64, max_claims: usize) -> Result<Vec<WorkClaim>> {
    if max_claims == 0 || max_claims > 32 {
        return Err(Error::TooLarge);
    }
    if max_claims == 1 || !storage.journal_candidate {
        #[cfg(feature = "test-support")]
        let result = native_work::batch::claim_singleton(storage, now)?;
        #[cfg(not(feature = "test-support"))]
        let result = storage.reaction_update(WorkUpdate::Claim { now })?;
        return match result {
            WorkResult::Claimed(claim) => Ok(vec![*claim]),
            WorkResult::Changed | WorkResult::Idle => Ok(vec![]),
        };
    }
    #[cfg(feature = "test-support")]
    let stages = storage.stage_observation.get();
    // Created before native guards: early exit releases the transaction first.
    #[cfg(feature = "test-support")]
    let mut prefix = ClaimPrefixGuard::new(stages);
    #[cfg(feature = "test-support")]
    let mut work = stages.map(|_| WorkCommitContext::prefix());
    #[cfg(feature = "test-support")]
    let timer = Timer::new(
        stages,
        StageOperation::WorkUpdate,
        StorageStage::ConnectionLock,
    );
    let mut connection = storage.connection.lock().map_err(|_| Error::Panicked)?;
    #[cfg(feature = "test-support")]
    drop(timer);
    #[cfg(feature = "test-support")]
    let timer = Timer::new(
        stages,
        StageOperation::WorkUpdate,
        StorageStage::TransactionBegin,
    );
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| Error::Storage)?;
    #[cfg(feature = "test-support")]
    drop(timer);
    let mut claims: Vec<WorkClaim> = vec![];
    let mut ordinal = 0usize;
    for _ in 0..max_claims {
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::WorkUpdate,
            StorageStage::SharedPrepare,
        );
        // Fresh per-update budgets see this writer's preceding staged leases.
        let mut reader = native_work::Reader::bounded(&tx, storage.validation_limits);
        #[cfg(feature = "test-support")]
        reader.observe_publications(stages, StageOperation::WorkUpdate);
        if JournalRead::header(&reader)?.retry_epochs() != WorkRead::header(&reader)?.retry_epochs {
            return Err(Error::Storage);
        }
        let delta = prepare_update(&reader, WorkUpdate::Claim { now })?;
        let ordinary = match delta.result() {
            WorkResult::Claimed(claim) => {
                let ordinary = claim_prefix_accepts_source(&claims, claim);
                #[cfg(feature = "test-support")]
                if prefix.enabled() && !ordinary {
                    let reason = if claim.resolution_only {
                        ClaimPrefixReason::ResolutionBarrier
                    } else {
                        match &claim.work.pending.payload {
                            rom::WorkPayload::Notification { .. } => {
                                ClaimPrefixReason::NotificationBarrier
                            }
                            rom::WorkPayload::Action(_) => ClaimPrefixReason::ActionBarrier,
                            rom::WorkPayload::Source(_) => ClaimPrefixReason::DuplicateRoot,
                        }
                    };
                    prefix.termination(
                        reason,
                        if claims.is_empty() {
                            ClaimPrefixDisposition::AdmittedSingleton
                        } else {
                            ClaimPrefixDisposition::Withheld
                        },
                    );
                }
                if !claims.is_empty() && !ordinary {
                    break;
                }
                ordinary
            }
            WorkResult::Changed | WorkResult::Idle => {
                #[cfg(feature = "test-support")]
                if prefix.enabled() {
                    prefix.termination(
                        if matches!(delta.result(), WorkResult::Changed) {
                            ClaimPrefixReason::MaintenanceChanged
                        } else {
                            ClaimPrefixReason::Idle
                        },
                        ClaimPrefixDisposition::Unrestricted,
                    );
                }
                false
            }
        };
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::WorkUpdate,
            StorageStage::NativePublication,
        );
        #[cfg(feature = "test-support")]
        if let Some(context) = &mut work {
            context.include(&delta);
        }
        let result = reader.apply(delta, || {
            ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
            storage.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
        })?;
        #[cfg(feature = "test-support")]
        drop(timer);
        if let WorkResult::Claimed(claim) = result {
            claims.push(*claim);
        }
        if !ordinary {
            break;
        }
    }
    storage.checkpoint(0).map_err(|_| Error::NotCommitted)?;
    #[cfg(feature = "test-support")]
    let timer = {
        if let Some(context) = &mut work {
            context.width = claims.len();
        }
        Timer::work_commit(stages, work)
    };
    let committed = tx.commit();
    #[cfg(feature = "test-support")]
    timer.finish(committed.is_err());
    #[cfg(feature = "test-support")]
    prefix.finish(claims.len(), committed.is_err());
    committed.map_err(|_| Error::Unknown)?;
    storage.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
    Ok(claims)
}
