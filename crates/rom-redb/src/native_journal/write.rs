use super::read::Reader;
use crate::{Redb, format::*, native_work::PreparedWork, references};
use redb::{Durability, ReadableTable, Table};
use rom::storage_support::metadata::{JournalDelta, JournalRead, prepare_native_bundle};
use rom::{Bundle, Error, Receipt, Result, Row, WorkResult, WorkUpdate};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Position {
    pub identity: String,
    pub canonical_bytes: u64,
}

pub(crate) struct PreparedJournal {
    header: String,
    appended: Option<(u64, String, String, String)>,
    retired: Vec<(u64, String)>,
}
impl PreparedJournal {
    pub(crate) fn new(delta: &JournalDelta) -> Result<Self> {
        let appended = delta
            .appended()
            .map(|e| {
                Ok((
                    e.event.position,
                    e.event.identity.clone(),
                    serde_json::to_string(&Position {
                        identity: e.event.identity.clone(),
                        canonical_bytes: u64::try_from(e.encoded_bytes)
                            .map_err(|_| Error::TooLarge)?,
                    })
                    .map_err(|_| Error::Storage)?,
                    serde_json::to_string(&e.event.row).map_err(|_| Error::Storage)?,
                ))
            })
            .transpose()?;
        Ok(Self {
            header: serde_json::to_string(&delta.header().parts()).map_err(|_| Error::Storage)?,
            appended,
            retired: delta
                .retired()
                .iter()
                .map(|e| (e.event.position, e.event.identity.clone()))
                .collect(),
        })
    }
    pub(crate) fn publish(
        &self,
        state: &mut Table<'_, &'static str, &'static str>,
        positions: &mut Table<'_, u64, &'static str>,
        events: &mut Table<'_, &'static str, &'static str>,
        checkpoint: &mut impl FnMut() -> Result<()>,
    ) -> Result<()> {
        if let Some((position, id, index, row)) = &self.appended {
            events
                .insert(id.as_str(), row.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
            positions
                .insert(*position, index.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        for (position, id) in &self.retired {
            positions
                .remove(*position)
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
            events
                .remove(id.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        state
            .insert("metadata", self.header.as_str())
            .map_err(|_| Error::NotCommitted)?;
        checkpoint()?;
        Ok(())
    }
}

pub(crate) fn commit_bundle(storage: &Redb, b: &Bundle) -> Result<Receipt> {
    let _gate = storage.commit_gate.lock().map_err(|_| Error::Storage)?;
    storage.available()?;
    let mut tx = storage.db.begin_write().map_err(|_| Error::Storage)?;
    storage.available()?;
    tx.set_durability(Durability::Immediate)
        .map_err(|_| Error::Storage)?;
    {
        let mut reader = Reader::writer(&tx, storage.validation_limits)?;
        let state = JournalRead::header(&reader)?;
        let mut receipts = tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
        let prior: Option<Receipt> = reader.decode(&receipts, &b.receipt.identity)?;
        state.check_retry_epoch(
            b.receipt.retry_epoch,
            prior.is_some(),
            b.completed_work.as_ref().map(|(k, n)| (k, *n)),
            &reader,
        )?;
        if let Some(prior) = prior {
            if prior.identity != b.receipt.identity {
                return Err(Error::Storage);
            }
            state.check_retry_epoch(prior.retry_epoch, true, None, &reader)?;
            return if prior.fingerprint == b.receipt.fingerprint
                && prior.retry_epoch == b.receipt.retry_epoch
            {
                Ok(prior)
            } else {
                Err(Error::IdentityMismatch)
            };
        }
        let mut rows = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
        let key = (
            b.receipt.row.key.kind.as_str(),
            b.receipt.row.key.id.as_str(),
        );
        let existing: Option<Row> = rows
            .get(key)
            .map_err(|_| Error::Storage)?
            .map(|raw| {
                reader.budget.charge(
                    key.0
                        .len()
                        .checked_add(key.1.len())
                        .and_then(|n| n.checked_add(raw.value().len()))
                        .ok_or(Error::TooLarge)?,
                )?;
                serde_json::from_str(raw.value()).map_err(|_| Error::Storage)
            })
            .transpose()?;
        crate::commit::validate_resource(b, existing.as_ref())?;
        let (old, new) = references::prepare_with_budget(
            &tx,
            &rows,
            &b.receipt,
            existing.as_ref(),
            &mut |bytes| reader.budget.charge(bytes),
        )?;
        if b.changed
            && reader
                .decode::<Row>(&reader.events, &b.receipt.identity)?
                .is_some()
        {
            return Err(Error::Storage);
        }
        let delta = prepare_native_bundle(&reader, &reader, b)?;
        delta.validate(&reader, &reader)?;
        let journal = PreparedJournal::new(delta.journal())?;
        let work = PreparedWork::new(delta.work())?;
        let row = serde_json::to_string(&b.receipt.row).map_err(|_| Error::NotCommitted)?;
        let receipt = serde_json::to_string(&b.receipt).map_err(|_| Error::NotCommitted)?;
        let effects = b
            .effects
            .iter()
            .enumerate()
            .map(|(i, e)| {
                Ok((
                    u64::try_from(i).map_err(|_| Error::TooLarge)?,
                    serde_json::to_string(e).map_err(|_| Error::NotCommitted)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut ordinal = 0usize;
        if b.changed {
            storage.replace_references(&tx, &b.receipt.row.key, &old, &new, &mut ordinal)?;
            rows.insert(key, row.as_str())
                .map_err(|_| Error::NotCommitted)?;
            ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
            storage
                .checkpoint(ordinal)
                .map_err(|_| Error::NotCommitted)?;
        }
        receipts
            .insert(b.receipt.identity.as_str(), receipt.as_str())
            .map_err(|_| Error::NotCommitted)?;
        ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
        storage
            .checkpoint(ordinal)
            .map_err(|_| Error::NotCommitted)?;
        let mut checkpoint = || {
            ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
            storage.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
        };
        journal.publish(
            &mut reader.state,
            &mut reader.positions,
            &mut reader.events,
            &mut checkpoint,
        )?;
        work.publish(
            &mut reader.state,
            &mut reader.records,
            &mut reader.roots,
            &mut reader.active,
            &mut checkpoint,
        )?;
        for (i, raw) in effects {
            tx.open_table(EFFECTS)
                .map_err(|_| Error::NotCommitted)?
                .insert((b.receipt.identity.as_str(), i), raw.as_str())
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        reader.invalidate();
    }
    storage.checkpoint(0).map_err(|_| Error::NotCommitted)?;
    if tx.commit().is_err() {
        storage.uncertain.store(true, Ordering::Release);
        return Err(Error::Unknown);
    }
    storage.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
    Ok(b.receipt.clone())
}

pub(crate) fn update_work(storage: &Redb, update: WorkUpdate) -> Result<WorkResult> {
    super::batch::update(storage, vec![update])?
        .into_iter()
        .next()
        .ok_or(Error::Storage)
}
