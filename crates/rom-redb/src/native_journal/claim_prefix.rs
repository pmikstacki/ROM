//! Bounded ordered claim preparation under the existing durable writer fence.
use super::Reader;
use crate::{Redb, native_work::PreparedWork};
use redb::{Durability, ReadableDatabase};
use rom::storage_support::{
    metadata::JournalRead,
    work::{WorkRead, claim_prefix_accepts_source, prepare_update},
};
use rom::{Error, Result, WorkClaim, WorkResult, WorkUpdate};
use std::sync::atomic::Ordering;

pub(crate) fn live(storage: &Redb, claim: &rom::ClaimKey, now: u64) -> Result<bool> {
    storage.available()?;
    let tx = storage.db.begin_read().map_err(|_| Error::Storage)?;
    let reader = Reader::read_transaction(&tx, storage.validation_limits)?;
    if JournalRead::header(&reader)?.retry_epochs() != WorkRead::header(&reader)?.retry_epochs {
        return Err(Error::Storage);
    }
    rom::storage_support::work::claim_live(&reader, claim, now)
}

pub(crate) fn claim(storage: &Redb, now: u64, max_claims: usize) -> Result<Vec<WorkClaim>> {
    if max_claims == 0 || max_claims > 32 {
        return Err(Error::TooLarge);
    }
    let _gate = storage.commit_gate.lock().map_err(|_| Error::Panicked)?;
    storage.available()?;
    let mut tx = storage.db.begin_write().map_err(|_| Error::Storage)?;
    storage.available()?;
    tx.set_durability(Durability::Immediate)
        .map_err(|_| Error::Storage)?;
    let mut claims: Vec<WorkClaim> = vec![];
    let mut ordinal = 0usize;
    for _ in 0..max_claims {
        // At most 32 per-update budgets, each observing this writer's mutations.
        let mut reader = Reader::writer(&tx, storage.validation_limits)?;
        if JournalRead::header(&reader)?.retry_epochs() != WorkRead::header(&reader)?.retry_epochs {
            return Err(Error::Storage);
        }
        let delta = prepare_update(&reader, WorkUpdate::Claim { now })?;
        let ordinary = match delta.result() {
            WorkResult::Claimed(claim) => {
                let ordinary = claim_prefix_accepts_source(&claims, claim);
                if !claims.is_empty() && !ordinary {
                    break;
                }
                ordinary
            }
            WorkResult::Changed | WorkResult::Idle => false,
        };
        delta.validate(&reader)?;
        PreparedWork::new(&delta)?.publish(
            &mut reader.state,
            &mut reader.records,
            &mut reader.roots,
            &mut reader.active,
            &mut || {
                ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
                storage.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
            },
        )?;
        reader.invalidate();
        if let WorkResult::Claimed(claim) = delta.into_result() {
            claims.push(*claim);
        }
        if !ordinary {
            break;
        }
    }
    storage.checkpoint(0).map_err(|_| Error::NotCommitted)?;
    if tx.commit().is_err() {
        storage.uncertain.store(true, Ordering::Release);
        return Err(Error::Unknown);
    }
    storage.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
    Ok(claims)
}
