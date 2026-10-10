//! Ordered format-11 Work updates under the existing durable writer fence.
use super::Reader;
use crate::{Redb, native_work::PreparedWork};
use redb::Durability;
use rom::storage_support::{
    metadata::JournalRead,
    work::{WorkRead, prepare_update, validate_work_update_batch},
};
use rom::{Error, Result, WorkResult, WorkUpdate};
use std::sync::atomic::Ordering;

pub(crate) fn update(storage: &Redb, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
    if updates.len() > 32 {
        return Err(Error::TooLarge);
    }
    if updates.is_empty() {
        return Ok(vec![]);
    }
    let grouped = updates.len() > 1;
    let _gate = storage.commit_gate.lock().map_err(|_| Error::Panicked)?;
    storage.available()?;
    let mut tx = storage.db.begin_write().map_err(|_| Error::Storage)?;
    storage.available()?;
    tx.set_durability(Durability::Immediate)
        .map_err(|_| Error::Storage)?;
    let count = updates.len();
    let mut results = Vec::with_capacity(count);
    let mut updates = updates.into_iter();
    let mut ordinal = 0usize;
    for _ in 0..count {
        // A fresh fence/budget reads prior staged writes. The 32-operation cap
        // bounds aggregate native read admission to 32 per-update budgets.
        let mut reader = Reader::writer(&tx, storage.validation_limits)?;
        let metadata = JournalRead::header(&reader)?;
        let header = WorkRead::header(&reader)?;
        if metadata.retry_epochs() != header.retry_epochs {
            return Err(Error::Storage);
        }
        if grouped && results.is_empty() {
            let max_bytes = header
                .limits
                .as_ref()
                .map_or(storage.validation_limits.max_bytes, |limits| {
                    limits.max_bytes
                });
            validate_work_update_batch(updates.as_slice(), max_bytes)?;
        }
        let delta = prepare_update(&reader, updates.next().ok_or(Error::Storage)?)?;
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
        results.push(delta.into_result());
    }
    storage.checkpoint(0).map_err(|_| Error::NotCommitted)?;
    if tx.commit().is_err() {
        storage.uncertain.store(true, Ordering::Release);
        return Err(Error::Unknown);
    }
    storage.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
    Ok(results)
}
