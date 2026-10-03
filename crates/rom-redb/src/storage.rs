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
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let state = crate::state::read(&table)?;
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
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let s = crate::state::read(&table)?;
        Ok(s.work.records())
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let mut tx = self.db.begin_write().map_err(|_| Error::Storage)?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        let result = {
            let mut table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
            let mut s = crate::state::read(&table)?;
            let result = s.update_work(update)?;
            crate::state::write(&mut table, &s)?;
            result
        };
        if tx.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        Ok(result)
    }
    fn supports_journal(&self) -> bool {
        true
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let s = crate::state::read(&table)?;
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
        let table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
        let s = crate::state::read(&table)?;
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
        table
            .get((key.kind.as_str(), key.id.as_str()))
            .map_err(|_| Error::Storage)?
            .map(|v| serde_json::from_str(v.value()).map_err(|_| Error::Storage))
            .transpose()
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
        let mut rows = Vec::new();
        let mut bytes = 0_usize;
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
            rows.push(serde_json::from_str(value.value()).map_err(|_| Error::Storage)?);
        }
        Ok(rows)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
        table
            .get(id)
            .map_err(|_| Error::Storage)?
            .map(|v| serde_json::from_str(v.value()).map_err(|_| Error::Storage))
            .transpose()
    }
    fn commit(&self, bundle: &Bundle) -> Result<Receipt> {
        self.commit_bundle(bundle)
    }
}
