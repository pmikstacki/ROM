//! Ordered Work transitions sharing one acknowledged native transaction.
#[cfg(feature = "test-support")]
use crate::stage_observation::{StageOperation, StorageStage, Timer, WorkCommitContext};
use crate::{Sqlite, native_work};
use rom::storage_support::work::{WorkRead, prepare_update, validate_work_update_batch};
use rom::{Error, Result, WorkResult, WorkUpdate};

pub(crate) fn update(storage: &Sqlite, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
    update_inner(
        storage,
        updates,
        #[cfg(feature = "test-support")]
        false,
    )
}

#[cfg(feature = "test-support")]
pub(super) fn claim_singleton(storage: &Sqlite, now: u64) -> Result<WorkResult> {
    update_inner(storage, vec![WorkUpdate::Claim { now }], true)?
        .into_iter()
        .next()
        .ok_or(Error::Storage)
}

fn update_inner(
    storage: &Sqlite,
    updates: Vec<WorkUpdate>,
    #[cfg(feature = "test-support")] prefix: bool,
) -> Result<Vec<WorkResult>> {
    if updates.len() > 32 {
        return Err(Error::TooLarge);
    }
    if updates.is_empty() {
        return Ok(vec![]);
    }
    let grouped = updates.len() > 1;
    if grouped && !storage.journal_candidate {
        return Err(Error::Unsupported(
            "atomic Work updates on predecessor format".into(),
        ));
    }
    #[cfg(feature = "test-support")]
    let stages = storage.stage_observation.get();
    #[cfg(feature = "test-support")]
    let mut work = stages.map(|_| {
        if prefix {
            WorkCommitContext::prefix()
        } else {
            WorkCommitContext::updates(&updates)
        }
    });
    #[cfg(feature = "test-support")]
    let timer = Timer::new(
        stages,
        StageOperation::WorkUpdate,
        StorageStage::ConnectionLock,
    );
    let mut c = storage.connection.lock().map_err(|_| Error::Panicked)?;
    #[cfg(feature = "test-support")]
    drop(timer);
    #[cfg(feature = "test-support")]
    let timer = Timer::new(
        stages,
        StageOperation::WorkUpdate,
        StorageStage::TransactionBegin,
    );
    let tx = c
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_| Error::Storage)?;
    #[cfg(feature = "test-support")]
    drop(timer);
    let count = updates.len();
    let mut results = Vec::with_capacity(count);
    let mut updates = updates.into_iter();
    let mut ordinal = 0usize;
    for _ in 0..count {
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::WorkUpdate,
            StorageStage::SharedPrepare,
        );
        // Each reader sees preceding writes in this same transaction. Its native
        // read budget is per update, so aggregate reads are bounded by 32 times it.
        let mut reader = if storage.journal_candidate {
            native_work::Reader::bounded(&tx, storage.validation_limits)
        } else {
            native_work::Reader::new(&tx)
        };
        #[cfg(feature = "test-support")]
        reader.observe_publications(stages, StageOperation::WorkUpdate);
        if grouped {
            let header = reader.header()?;
            if rom::storage_support::metadata::JournalRead::header(&reader)?.retry_epochs()
                != header.retry_epochs
            {
                return Err(Error::Storage);
            }
            if results.is_empty() {
                let max_bytes = header
                    .limits
                    .as_ref()
                    .map_or(storage.validation_limits.max_bytes, |limits| {
                        limits.max_bytes
                    });
                validate_work_update_batch(updates.as_slice(), max_bytes)?;
            }
        }
        let delta = prepare_update(&reader, updates.next().ok_or(Error::Storage)?)?;
        #[cfg(feature = "test-support")]
        if let Some(context) = &mut work {
            context.include(&delta);
        }
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::WorkUpdate,
            StorageStage::NativePublication,
        );
        results.push(reader.apply(delta, || {
            ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
            storage.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
        })?);
        if results.len() == count {
            storage.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        }
        #[cfg(feature = "test-support")]
        drop(timer);
    }
    #[cfg(feature = "test-support")]
    let timer = {
        if prefix && let Some(context) = &mut work {
            context.width = results
                .iter()
                .filter(|result| matches!(result, WorkResult::Claimed(_)))
                .count();
        }
        Timer::work_commit(stages, work)
    };
    let committed = tx.commit();
    #[cfg(feature = "test-support")]
    timer.finish(committed.is_err());
    committed.map_err(|_| Error::Unknown)?;
    storage.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
    Ok(results)
}
