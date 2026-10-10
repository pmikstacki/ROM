use crate::Sqlite;
use rom::{Error, Result};

impl Sqlite {
    pub(crate) fn make_journal_candidate(mut self) -> Result<Self> {
        {
            let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| Error::Storage)?;
            super::canonical::admit_inventory_for_format(&tx, self.validation_limits, 10)?;
            let metadata = crate::native_work::metadata(&tx)?;
            super::initialize(&tx)?;
            super::replace(&tx, metadata)?;
            tx.pragma_update(None, "user_version", 11)
                .map_err(|_| Error::Storage)?;
            tx.commit().map_err(|_| Error::Unknown)?;
        }
        self.journal_candidate = true;
        Ok(self)
    }

    pub(crate) fn upgrade_journal_candidate(
        source: &std::path::Path,
        destination: &std::path::Path,
        limits: rom_backup::BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        let source_owner =
            rom_backup::NativeOwnership::acquire(source, rom_backup::NativeAccess::Existing)?;
        let destination_owner =
            rom_backup::NativeOwnership::acquire(destination, rom_backup::NativeAccess::Fresh)?;
        let snapshot = crate::snapshot::read_snapshot(source_owner.path(), limits, |c, limits| {
            super::canonical::admit_inventory_for_format(c, limits, 10)?;
            crate::snapshot::collect_native_snapshot(c, limits, false)
        })?;
        Self::restore_snapshot(snapshot, destination_owner, limits, before_publish)
    }
}
