//! Synchronous Storage operations over coherent native transactions.
use crate::{Redb, format::*};
use redb::{Durability, ReadableDatabase};
use rom::{
    Bundle, Capabilities, Descriptor, Error, JournalCursor, JournalPage, Key, Receipt, Result, Row,
    Storage, WorkRecord, WorkResult, WorkUpdate,
};
use std::sync::atomic::Ordering;

impl Storage for Redb {
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
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        if self.native_format == JOURNAL_FORMAT {
            let reader =
                crate::native_journal::Reader::read_transaction(&tx, self.validation_limits)?;
            return Ok(
                rom::storage_support::metadata::JournalRead::header(&reader)?.retry_epochs(),
            );
        }
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let state = crate::native_state::metadata(&table)?;
        Ok(state.retry_epochs())
    }
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        self.register_descriptors(descriptors)
    }
    fn supports_reactions(&self) -> bool {
        true
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        crate::admission::check(&tx, self.native_format, self.validation_limits)?;
        if self.native_format == JOURNAL_FORMAT {
            return Ok(crate::maintenance::snapshot_in_format(
                &tx,
                self.validation_limits,
                crate::maintenance::NativeFormat::Exact(self.native_format),
            )?
            .state
            .work
            .records());
        }
        let state = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let records = tx.open_table(WORK).map_err(|_| Error::Storage)?;
        let roots = tx.open_table(ROOTS).map_err(|_| Error::Storage)?;
        let active = tx.open_table(ACTIVE).map_err(|_| Error::Storage)?;
        let (canonical, _) =
            crate::native_state::read(&state, &records, &roots, &active, self.validation_limits)?;
        Ok(canonical.work.records())
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        if self.native_format == JOURNAL_FORMAT {
            return crate::native_journal::update_work(self, update);
        }
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let mut tx = self.db.begin_write().map_err(|_| Error::Storage)?;
        self.available()?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        let mut ordinal = 0usize;
        let result = {
            let mut table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
            let mut work = crate::native_work::NativeWork::new(&tx, &table)?;
            let delta = rom::storage_support::work::prepare_update(&work, update)?;
            work.apply(delta, &mut table, &mut || {
                ordinal = ordinal.checked_add(1).ok_or(Error::TooLarge)?;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)
            })?
        };
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        if tx.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        Ok(result)
    }
    fn reaction_updates_atomic(&self, updates: Vec<WorkUpdate>) -> Result<Vec<WorkResult>> {
        if updates.len() > 32 {
            return Err(Error::TooLarge);
        }
        if updates.is_empty() {
            return Ok(vec![]);
        }
        if updates.len() == 1 {
            return self
                .reaction_update(updates.into_iter().next().ok_or(Error::Storage)?)
                .map(|result| vec![result]);
        }
        if self.native_format != JOURNAL_FORMAT {
            return Err(Error::Unsupported(
                "atomic Work updates on predecessor format".into(),
            ));
        }
        crate::native_journal::batch::update(self, updates)
    }
    fn reaction_claim_prefix(&self, now: u64, max_claims: usize) -> Result<Vec<rom::WorkClaim>> {
        if max_claims == 0 || max_claims > 32 {
            return Err(Error::TooLarge);
        }
        if self.native_format != JOURNAL_FORMAT {
            return match self.reaction_update(WorkUpdate::Claim { now })? {
                WorkResult::Claimed(claim) => Ok(vec![*claim]),
                WorkResult::Changed | WorkResult::Idle => Ok(vec![]),
            };
        }
        crate::native_journal::claim_prefix::claim(self, now, max_claims)
    }
    fn reaction_claim_live(&self, claim: &rom::ClaimKey, now: u64) -> Result<Option<bool>> {
        if self.native_format != JOURNAL_FORMAT {
            return Ok(None);
        }
        crate::native_journal::claim_prefix::live(self, claim, now).map(Some)
    }
    fn supports_journal(&self) -> bool {
        true
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        if self.native_format == JOURNAL_FORMAT {
            let reader =
                crate::native_journal::Reader::read_transaction(&tx, self.validation_limits)?;
            let parts = rom::storage_support::metadata::JournalRead::header(&reader)?.parts();
            return Ok(JournalCursor {
                generation: parts.generation,
                kind: kind.into(),
                position: parts.head,
            });
        }
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let s = crate::native_state::metadata(&table)?;
        Ok(s.journal_head(kind))
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        if self.native_format == JOURNAL_FORMAT {
            let reader =
                crate::native_journal::Reader::read_transaction(&tx, self.validation_limits)?;
            return rom::storage_support::metadata::journal_page(
                &reader, kind, after, max_rows, max_bytes,
            );
        }
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let s = crate::native_state::metadata(&table)?;
        s.journal(kind, after, max_rows, max_bytes)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_bundle: true,
            snapshots: true,
            effects: true,
        }
    }
    fn load(&self, key: &Key) -> Result<Option<Row>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
        let budget = crate::native_journal::Budget::new(self.validation_limits);
        table
            .get((key.kind.as_str(), key.id.as_str()))
            .map_err(|_| Error::Storage)?
            .map(|v| {
                crate::native_records::row(
                    (key.kind.as_str(), key.id.as_str()),
                    v.value(),
                    (self.native_format == JOURNAL_FORMAT).then_some(&budget),
                )
            })
            .transpose()
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
        let mut rows = Vec::new();
        let mut bytes = 0_usize;
        let budget = crate::native_journal::Budget::new(self.validation_limits);
        for entry in table.range((kind, "")..).map_err(|_| Error::Storage)? {
            let (key, value) = entry.map_err(|_| Error::Storage)?;
            if key.value().0 != kind {
                break;
            }
            bytes = bytes
                .checked_add(value.value().len())
                .ok_or(Error::TooLarge)?;
            if rows.len() >= max_rows || bytes > max_bytes {
                return Err(Error::TooLarge);
            }
            rows.push(crate::native_records::row(
                key.value(),
                value.value(),
                (self.native_format == JOURNAL_FORMAT).then_some(&budget),
            )?);
        }
        Ok(rows)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
        let budget = crate::native_journal::Budget::new(self.validation_limits);
        table
            .get(id)
            .map_err(|_| Error::Storage)?
            .map(|v| {
                crate::native_records::receipt(
                    id,
                    v.value(),
                    (self.native_format == JOURNAL_FORMAT).then_some(&budget),
                )
            })
            .transpose()
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.commit_bundle(bundle)
    }
}
