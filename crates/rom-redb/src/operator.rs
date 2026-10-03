//! Native atomic operator controls; domain transitions stay in shared core state.
use crate::{Redb, format::STATE, state};
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
        state::read(&table)?.work_snapshot(max_records, max_bytes)
    }

    pub(crate) fn operator_control(
        &self,
        control: &StorageWorkControl,
    ) -> Result<WorkControlReceipt> {
        let _writer = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let mut transaction = self.db.begin_write().map_err(|_| Error::Storage)?;
        transaction
            .set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        let receipt = {
            let mut table = transaction.open_table(STATE).map_err(|_| Error::Storage)?;
            let mut metadata = state::read(&table)?;
            let receipt = metadata.control_work(control)?;
            if receipt.result.replayed {
                return Ok(receipt);
            }
            self.checkpoint(usize::MAX - 1)
                .map_err(|_| Error::NotCommitted)?;
            state::write(&mut table, &metadata)?;
            self.checkpoint(1).map_err(|_| Error::NotCommitted)?;
            receipt
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
