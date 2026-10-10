//! The native atomic Resource bundle and authoritative read interface.
#[cfg(feature = "test-support")]
use crate::stage_observation::{StageOperation, StorageStage, Timer};
use crate::{Sqlite, index, native_work, references};
use rom::{
    Bundle, Capabilities, Descriptor, Error, JournalCursor, JournalPage, Key, Receipt, Result, Row,
    Storage, StorageState, WorkRecord, WorkResult, WorkUpdate,
};
use rusqlite::{Connection, OptionalExtension, params};
#[cfg(feature = "test-support")]
use std::sync::atomic::Ordering;

pub(crate) fn row(c: &Connection, key: &Key) -> Result<Option<Row>> {
    Ok(row_with_length(c, key)?.map(|(row, _)| row))
}
fn row_with_length(c: &Connection, key: &Key) -> Result<Option<(Row, usize)>> {
    let text: Option<String> = c
        .prepare_cached("SELECT data FROM resources WHERE kind=? AND id=?")
        .map_err(|_| Error::Storage)?
        .query_row(params![key.kind, key.id], |r| r.get(0))
        .optional()
        .map_err(|_| Error::Storage)?;
    text.map(|t| {
        serde_json::from_str(&t)
            .map(|row| (row, t.len()))
            .map_err(|_| Error::Storage)
    })
    .transpose()
}
pub(crate) fn receipt(c: &Connection, id: &str) -> Result<Option<Receipt>> {
    let text: Option<String> = c
        .query_row("SELECT data FROM receipts WHERE identity=?", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| Error::Storage)?;
    text.map(|t| serde_json::from_str(&t).map_err(|_| Error::Storage))
        .transpose()
}
/// Full canonical read for explicit bounded maintenance, never ordinary mutations.
pub(crate) fn state(c: &Connection, limits: rom_backup::BackupLimits) -> Result<StorageState> {
    Ok(native_work::reconstruct(c, limits)?.state)
}
pub(crate) fn save_state(c: &Connection, state: &StorageState) -> Result<()> {
    native_work::replace_state(c, state)
}
pub(crate) fn snapshot_rows(
    c: &Connection,
    kind: &str,
    max_rows: usize,
    max_bytes: usize,
) -> Result<Vec<Row>> {
    Ok(crate::read_rows::snapshot::<false>(c, kind, max_rows, max_bytes)?.0)
}
impl Storage for Sqlite {
    fn supports_operator(&self) -> bool {
        true
    }
    fn work_snapshot(
        &self,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<rom::StorageWorkSnapshot> {
        self.operator_snapshot(max_records, max_bytes)
    }
    fn control_work(&self, control: &rom::StorageWorkControl) -> Result<rom::WorkControlReceipt> {
        self.operator_control(control)
    }
    fn acquire_owner(&self) -> Result<rom::StorageOwner> {
        self.ownership.acquire()
    }
    fn retry_epochs(&self) -> Result<rom::RetryEpochs> {
        #[cfg(feature = "test-support")]
        let stages = self.stage_observation.get();
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::RetryEpochs,
            StorageStage::ConnectionLock,
        );
        let connection = self.connection.lock().map_err(|_| Error::Panicked)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::RetryEpochs,
            StorageStage::MetadataRead,
        );
        let epochs = if self.journal_candidate {
            rom::storage_support::metadata::JournalRead::header(&native_work::Reader::bounded(
                &connection,
                self.validation_limits,
            ))?
            .retry_epochs()
        } else {
            native_work::metadata(&connection)?.retry_epochs()
        };
        #[cfg(feature = "test-support")]
        drop(timer);
        Ok(epochs)
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        references::register(&tx, descriptors)?;
        index::register(&tx, descriptors)?;
        tx.commit().map_err(|_| Error::Unknown)
    }
    fn supports_reactions(&self) -> bool {
        true
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        Ok(state(
            &*self.connection.lock().map_err(|_| Error::Panicked)?,
            self.validation_limits,
        )?
        .work
        .records())
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        native_work::batch::update(self, vec![update])?
            .into_iter()
            .next()
            .ok_or(Error::Storage)
    }
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        native_work::batch::update(self, updates)
    }
    fn reaction_claim_prefix(&self, now: u64, max_claims: usize) -> Result<Vec<rom::WorkClaim>> {
        native_work::claim_prefix::claim(self, now, max_claims)
    }
    fn reaction_claim_live(&self, claim: &rom::ClaimKey, now: u64) -> Result<Option<bool>> {
        if !self.journal_candidate {
            return Ok(None);
        }
        native_work::claim_prefix::live(self, claim, now).map(Some)
    }
    fn supports_journal(&self) -> bool {
        true
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        if self.journal_candidate {
            let c = self.connection.lock().map_err(|_| Error::Panicked)?;
            let h = rom::storage_support::metadata::JournalRead::header(
                &native_work::Reader::bounded(&c, self.validation_limits),
            )?
            .parts();
            return Ok(JournalCursor {
                generation: h.generation,
                kind: kind.into(),
                position: h.head,
            });
        }
        Ok(
            native_work::metadata(&*self.connection.lock().map_err(|_| Error::Panicked)?)?
                .journal_head(kind),
        )
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        if self.journal_candidate {
            let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
            let tx = c.transaction().map_err(|_| Error::Storage)?;
            let reader = native_work::Reader::bounded(&tx, self.validation_limits);
            return rom::storage_support::metadata::journal_page(
                &reader, kind, after, max_rows, max_bytes,
            );
        }
        native_work::metadata(&*self.connection.lock().map_err(|_| Error::Panicked)?)?
            .journal(kind, after, max_rows, max_bytes)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        let c = self.connection.lock().unwrap();
        if self.journal_candidate {
            return Ok(native_work::Reader::bounded(&c, self.validation_limits)
                .resource(key)?
                .map(|(row, _)| row));
        }
        row(&c, key)
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        let c = self.connection.lock().map_err(|_| Error::Panicked)?;
        snapshot_rows(&c, kind, max_rows, max_bytes)
    }
    fn query_read(
        &self,
        request: &rom::StorageQuery,
        bounds: rom::QueryBounds,
    ) -> Result<rom::QueryRead> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c.transaction().map_err(|_| Error::Storage)?;
        index::query_read(&tx, request, bounds)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        let c = self.connection.lock().unwrap();
        if self.journal_candidate {
            return native_work::Reader::bounded(&c, self.validation_limits).receipt(id);
        }
        receipt(&c, id)
    }
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        #[cfg(feature = "test-support")]
        let stages = self.stage_observation.get();
        #[cfg(feature = "test-support")]
        let timer = Timer::new(stages, StageOperation::Commit, StorageStage::ConnectionLock);
        let mut c = self.connection.lock().unwrap();
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::Commit,
            StorageStage::TransactionBegin,
        );
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(stages, StageOperation::Commit, StorageStage::MetadataRead);
        let mut reader = if self.journal_candidate {
            native_work::Reader::bounded(&tx, self.validation_limits)
        } else {
            native_work::Reader::new(&tx)
        };
        let metadata = crate::native_journal::Metadata::load(&reader, self.journal_candidate)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(stages, StageOperation::Commit, StorageStage::SharedPrepare);
        #[cfg(feature = "test-support")]
        reader.observe_publications(stages, StageOperation::Commit);
        let prior = if self.journal_candidate {
            reader.receipt(&b.receipt.identity)?
        } else {
            receipt(&tx, &b.receipt.identity)?
        };
        metadata.check_retry_epoch(
            b.receipt.retry_epoch,
            prior.is_some(),
            b.completed_work.as_ref().map(|(key, now)| (key, *now)),
            &reader,
        )?;
        if let Some(prior) = prior {
            metadata.check_retry_epoch(prior.retry_epoch, true, None, &reader)?;
            if prior.fingerprint != b.receipt.fingerprint
                || prior.retry_epoch != b.receipt.retry_epoch
            {
                return Err(Error::IdentityMismatch);
            }
            return Ok(prior);
        }
        let stored = if self.journal_candidate {
            reader.resource(&b.receipt.row.key)?
        } else {
            row_with_length(&tx, &b.receipt.row.key)?
        };
        let old_raw_len = stored.as_ref().map(|(_, len)| *len);
        let existing = stored.map(|(row, _)| row);
        if existing.as_ref().map(|r| r.revision) != b.expected {
            return Err(Error::Conflict);
        }
        let expected_revision = b
            .expected
            .unwrap_or(0)
            .checked_add(u64::from(b.changed))
            .ok_or(Error::TooLarge)?;
        if b.receipt.row.revision != expected_revision
            || (!b.changed && (!b.effects.is_empty() || existing.as_ref() != Some(&b.receipt.row)))
        {
            return Err(Error::NotCommitted);
        }
        let admitted_descriptor = if self.journal_candidate {
            Some(reader.descriptor(&b.receipt.row.key.kind)?)
        } else {
            None
        };
        let targets = if let Some(definition) = &admitted_descriptor {
            references::prepare_with(&tx, &b.receipt, definition, |target| {
                Ok(reader.resource(target)?.map(|(row, _)| row))
            })?
        } else {
            references::prepare(&tx, &b.receipt)?
        };
        let crate::native_journal::Prepared {
            metadata,
            work: work_delta,
            journal,
            retired,
            encoded_header,
        } = metadata.prepare(&reader, b)?;
        let prepared_index = if b.changed {
            admitted_descriptor
                .as_ref()
                .map(|descriptor| {
                    index::prepare(
                        &tx,
                        existing.as_ref(),
                        &b.receipt.row,
                        old_raw_len,
                        descriptor,
                        Some(&reader),
                    )
                })
                .transpose()?
        } else {
            None
        };
        let previous_edges = if self.journal_candidate && b.changed {
            Some(references::previous(
                &tx,
                &b.receipt.row.key,
                Some(&reader),
            )?)
        } else {
            None
        };
        let prepared_work = native_work::PreparedWork::encode(work_delta)?;
        let encoded = crate::native_journal::EncodedBundle::new(b)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(
            stages,
            StageOperation::Commit,
            StorageStage::NativePublication,
        );
        #[cfg(feature = "test-support")]
        let f = self.fault.swap(0, Ordering::SeqCst);
        #[cfg(not(feature = "test-support"))]
        let f = 0;
        let mut ordinal = 0;
        if b.changed {
            let r = &b.receipt.row;
            tx.execute("INSERT INTO resources(kind,id,revision,data) VALUES (?,?,?,?) ON CONFLICT(kind,id) DO UPDATE SET revision=excluded.revision,data=excluded.data",params![r.key.kind,r.key.id,i64::try_from(r.revision).map_err(|_|Error::TooLarge)?,&encoded.row]).map_err(|_|Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            if let Some(previous) = &previous_edges {
                references::publish(self, &tx, &r.key, &targets, previous, &mut ordinal)?;
            } else {
                references::replace(self, &tx, &r.key, &targets, &mut ordinal)?;
            }
            let mut checkpoint = || {
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
            };
            if let Some(prepared) = &prepared_index {
                index::publish(&tx, r, prepared, &mut checkpoint)?;
            } else {
                index::replace(&tx, existing.as_ref(), r, old_raw_len, &mut checkpoint)?;
            }
        }
        if f == 1 {
            return Err(Error::NotCommitted);
        }
        if b.changed {
            let raw = &encoded.row;
            tx.execute(
                "INSERT INTO events(identity,data) VALUES (?,?)",
                params![b.receipt.identity, &raw],
            )
            .map_err(|_| Error::NotCommitted)?;
            #[cfg(feature = "test-support")]
            if let Some(handle) = stages {
                handle.record_publication(
                    StageOperation::Commit,
                    crate::PublicationCategory::EventPayload,
                    raw.len(),
                );
            }
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        if f == 2 {
            return Err(Error::NotCommitted);
        }
        tx.execute(
            "INSERT INTO receipts(identity,data) VALUES (?,?)",
            params![b.receipt.identity, &encoded.receipt],
        )
        .map_err(|_| Error::NotCommitted)?;
        ordinal += 1;
        self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        if f == 3 {
            return Err(Error::NotCommitted);
        }
        for (i, raw) in encoded.effects.iter().enumerate() {
            tx.execute(
                "INSERT INTO effects(identity,ordinal,data) VALUES (?,?,?)",
                params![b.receipt.identity, i as i64, raw],
            )
            .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        if f == 4 {
            return Err(Error::NotCommitted);
        }
        for id in retired {
            tx.execute("DELETE FROM events WHERE identity=?", [id])
                .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        reader.publish_work(prepared_work, || {
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
        })?;
        if let Some(journal) = journal {
            reader.publish_journal(
                &journal,
                encoded_header.as_deref().ok_or(Error::Storage)?,
                || {
                    ordinal += 1;
                    self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
                },
            )?;
        }
        reader.fence = rom::storage_support::work::ReadFence::new();
        if let Some(metadata) = metadata {
            #[cfg(feature = "test-support")]
            native_work::save_metadata_observed(&tx, &metadata, stages)?;
            #[cfg(not(feature = "test-support"))]
            native_work::save_metadata(&tx, &metadata)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        #[cfg(feature = "test-support")]
        let timer = Timer::new(stages, StageOperation::Commit, StorageStage::NativeCommit);
        tx.commit().map_err(|_| Error::Unknown)?;
        #[cfg(feature = "test-support")]
        drop(timer);
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        if f == 5 {
            Err(Error::Unknown)
        } else {
            Ok(b.receipt.clone())
        }
    }
}
