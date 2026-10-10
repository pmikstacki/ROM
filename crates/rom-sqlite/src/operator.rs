//! Atomic operator controls share the native metadata transaction.
use crate::{
    Sqlite,
    persistence::{save_state, state},
};
use rom::{Error, Result, StorageWorkControl, StorageWorkSnapshot, WorkControlReceipt};

impl Sqlite {
    pub(crate) fn operator_snapshot(
        &self,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<StorageWorkSnapshot> {
        let connection = self.connection.lock().map_err(|_| Error::Panicked)?;
        crate::native_work::admit_operator(&connection, max_records, max_bytes)?;
        state(&connection, self.validation_limits)?.work_snapshot(max_records, max_bytes)
    }

    pub(crate) fn operator_control(
        &self,
        control: &StorageWorkControl,
    ) -> Result<WorkControlReceipt> {
        let mut connection = self.connection.lock().map_err(|_| Error::Panicked)?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        let mut metadata = state(&transaction, self.validation_limits)?;
        let receipt = metadata.control_work(control)?;
        if receipt.result.replayed {
            return Ok(receipt);
        }
        self.checkpoint(usize::MAX - 1)
            .map_err(|_| Error::NotCommitted)?;
        save_state(&transaction, &metadata)?;
        self.checkpoint(1).map_err(|_| Error::NotCommitted)?;
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        transaction.commit().map_err(|_| Error::Unknown)?;
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        Ok(receipt)
    }
}
