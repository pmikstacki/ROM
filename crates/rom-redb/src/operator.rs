//! Native atomic operator controls; domain transitions stay in shared core state.
use crate::{
    Redb,
    format::{ACTIVE, ROOTS, STATE, WORK},
};
use redb::{Durability, ReadableDatabase};
use rom::{Error, Result, StorageWorkControl, StorageWorkSnapshot, WorkControlReceipt};
use std::sync::atomic::Ordering;

impl Redb {
    pub(crate) fn operator_snapshot(
        &self,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<StorageWorkSnapshot> {
        self.available()?;
        let transaction = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = transaction.open_table(STATE).map_err(|_| Error::Storage)?;
        let records = transaction.open_table(WORK).map_err(|_| Error::Storage)?;
        let roots = transaction.open_table(ROOTS).map_err(|_| Error::Storage)?;
        let active = transaction.open_table(ACTIVE).map_err(|_| Error::Storage)?;
        crate::native_state::operator_budget(&table, &records, max_records, max_bytes)?;
        crate::admission::check(&transaction, self.native_format, self.validation_limits)?;
        if self.native_format == crate::format::JOURNAL_FORMAT {
            return crate::maintenance::snapshot_in_format(
                &transaction,
                self.validation_limits,
                crate::maintenance::NativeFormat::Exact(self.native_format),
            )?
            .state
            .work_snapshot(max_records, max_bytes);
        }
        let (state, _) =
            crate::native_state::read(&table, &records, &roots, &active, self.validation_limits)?;
        state.work_snapshot(max_records, max_bytes)
    }

    pub(crate) fn operator_control(
        &self,
        control: &StorageWorkControl,
    ) -> Result<WorkControlReceipt> {
        let _writer = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let read = self.db.begin_read().map_err(|_| Error::Storage)?;
        crate::admission::check(&read, self.native_format, self.validation_limits)?;
        drop(read);
        let mut transaction = self.db.begin_write().map_err(|_| Error::Storage)?;
        transaction
            .set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        let receipt = {
            if self.native_format == crate::format::JOURNAL_FORMAT {
                let reader =
                    crate::native_journal::Reader::writer(&transaction, self.validation_limits)?;
                let (mut metadata, _) =
                    crate::native_journal::collect(&reader, self.validation_limits)?;
                let receipt = metadata.control_work(control)?;
                if receipt.result.replayed {
                    return Ok(receipt);
                }
                reader.invalidate();
                drop(reader);
                self.checkpoint(usize::MAX - 1)
                    .map_err(|_| Error::NotCommitted)?;
                crate::native_journal::import(&transaction, &metadata)?;
                self.checkpoint(1).map_err(|_| Error::NotCommitted)?;
                receipt
            } else {
                let table = transaction.open_table(STATE).map_err(|_| Error::Storage)?;
                let records = transaction.open_table(WORK).map_err(|_| Error::Storage)?;
                let roots = transaction.open_table(ROOTS).map_err(|_| Error::Storage)?;
                let active = transaction.open_table(ACTIVE).map_err(|_| Error::Storage)?;
                let (mut metadata, _) = crate::native_state::read(
                    &table,
                    &records,
                    &roots,
                    &active,
                    self.validation_limits,
                )?;
                drop((table, records, roots, active));
                let receipt = metadata.control_work(control)?;
                if receipt.result.replayed {
                    return Ok(receipt);
                }
                self.checkpoint(usize::MAX - 1)
                    .map_err(|_| Error::NotCommitted)?;
                crate::native_state::write(&transaction, &metadata)?;
                self.checkpoint(1).map_err(|_| Error::NotCommitted)?;
                receipt
            }
        };
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        if transaction.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        Ok(receipt)
    }
}
