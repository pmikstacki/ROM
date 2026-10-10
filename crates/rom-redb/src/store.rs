//! Database lifecycle, ownership and trusted administrative inspection.
use crate::{
    format::*,
    maintenance,
    references::{INCOMING, OUTGOING, SCHEMAS},
};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata};
#[cfg(feature = "test-support")]
use rom::Row;
use rom::{Error, Intent, Result, StorageLimits, StorageState};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "test-support")]
type Observer = std::sync::Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;

/// One database handle; clones should be shared using `Arc`, not reopened concurrently.
pub struct Redb {
    pub(super) db: Database,
    pub(super) ownership: rom::StorageOwnership,
    pub(super) validation_limits: rom_backup::BackupLimits,
    pub(super) native_format: u64,
    // The engine field must drop before its native ownership guard.
    _native_owner: rom_backup::NativeOwnership,
    pub(super) uncertain: AtomicBool,
    pub(super) commit_gate: std::sync::Mutex<()>,
    #[cfg(feature = "test-support")]
    observer: std::sync::Mutex<Option<Observer>>,
}
impl Redb {
    /// Open format four, or initialize a new empty database. Never upgrade implicitly.
    /// An unclean close can require a private recovery probe with temporary disk space
    /// approximately equal to the source file size. Unsupported sources remain unchanged.
    /// The native guard excludes other ROM owners of this local database path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_limits(path, StorageLimits::default())
    }
    pub fn open_with_limits(path: impl AsRef<Path>, limits: StorageLimits) -> Result<Self> {
        Self::open_with_validation_limits(path, limits, rom_backup::BackupLimits::default())
    }
    /// Validate the complete persisted graph within explicit startup byte/record budgets.
    pub fn open_with_validation_limits(
        path: impl AsRef<Path>,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
    ) -> Result<Self> {
        Self::open_owned(
            rom_backup::NativeOwnership::acquire(
                path.as_ref(),
                rom_backup::NativeAccess::OpenOrCreate,
            )?,
            limits,
            validation_limits,
        )
    }
    pub(super) fn open_owned(
        owner: rom_backup::NativeOwnership,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
    ) -> Result<Self> {
        Self::open_owned_in_format(owner, limits, validation_limits, FORMAT)
    }
    pub(super) fn open_owned_in_format(
        owner: rom_backup::NativeOwnership,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
        native_format: u64,
    ) -> Result<Self> {
        let path = owner.path();
        crate::preflight::check(path, native_format, &limits, validation_limits)?;
        let db = Database::create(path).map_err(|_| Error::Storage)?;
        let read = db.begin_read().map_err(|_| Error::Storage)?;
        let empty =
            crate::preflight::validate_existing(&read, native_format, &limits, validation_limits)?;
        drop(read);
        if empty {
            let mut tx = db.begin_write().map_err(|_| Error::Storage)?;
            tx.set_durability(Durability::Immediate)
                .map_err(|_| Error::Storage)?;
            let state = StorageState::new(limits.clone())?;
            if native_format == JOURNAL_FORMAT {
                crate::native_journal::import(&tx, &state)?;
            } else {
                crate::native_state::write(&tx, &state)?;
            }
            tx.open_table(ROWS).map_err(|_| Error::Storage)?;
            tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
            tx.open_table(EVENTS).map_err(|_| Error::Storage)?;
            tx.open_table(EFFECTS).map_err(|_| Error::Storage)?;
            tx.open_table(SCHEMAS).map_err(|_| Error::Storage)?;
            tx.open_table(OUTGOING).map_err(|_| Error::Storage)?;
            tx.open_table(INCOMING).map_err(|_| Error::Storage)?;
            tx.open_table(META)
                .map_err(|_| Error::Storage)?
                .insert("format", native_format)
                .map_err(|_| Error::Storage)?;
            tx.commit().map_err(|_| Error::Unknown)?;
            maintenance::snapshot_in_format(
                &db.begin_read().map_err(|_| Error::Storage)?,
                validation_limits,
                maintenance::NativeFormat::Exact(native_format),
            )?
            .validate()?;
        }
        Ok(Self {
            db,
            ownership: rom::StorageOwnership::default(),
            validation_limits,
            native_format,
            _native_owner: owner,
            uncertain: AtomicBool::new(false),
            commit_gate: std::sync::Mutex::new(()),
            #[cfg(feature = "test-support")]
            observer: std::sync::Mutex::new(None),
        })
    }
    pub(super) fn available(&self) -> Result<()> {
        if self.uncertain.load(Ordering::Acquire) {
            Err(Error::Unknown)
        } else {
            Ok(())
        }
    }
    /// Test-only transaction observer: 1-based write ordinal; 0 before commit;
    /// usize::MAX after commit; usize::MAX - 1 before an operator metadata write.
    /// Returning an error before commit rolls back.
    #[cfg(feature = "test-support")]
    pub fn on_commit(&self, observer: Option<Observer>) {
        *self.observer.lock().unwrap() = observer;
    }
    pub(super) fn checkpoint(&self, point: usize) -> Result<()> {
        #[cfg(feature = "test-support")]
        {
            let observer = self.observer.lock().map_err(|_| Error::Storage)?.clone();
            if let Some(observer) = observer {
                observer(point)?;
            }
        }
        let _ = point;
        Ok(())
    }
    /// Administrative counts in Resource, event, receipt, effect order.
    pub fn counts(&self) -> Result<[u64; 4]> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        Ok([
            tx.open_table(ROWS)
                .map_err(|_| Error::Storage)?
                .len()
                .map_err(|_| Error::Storage)?,
            tx.open_table(EVENTS)
                .map_err(|_| Error::Storage)?
                .len()
                .map_err(|_| Error::Storage)?,
            tx.open_table(RECEIPTS)
                .map_err(|_| Error::Storage)?
                .len()
                .map_err(|_| Error::Storage)?,
            tx.open_table(EFFECTS)
                .map_err(|_| Error::Storage)?
                .len()
                .map_err(|_| Error::Storage)?,
        ])
    }
    /// Administrative effect inspection; hosts must limit access to this full scan.
    pub fn intentions(&self) -> Result<Vec<(String, Intent)>> {
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(EFFECTS).map_err(|_| Error::Storage)?;
        table
            .iter()
            .map_err(|_| Error::Storage)?
            .map(|entry| {
                let (key, value) = entry.map_err(|_| Error::Storage)?;
                Ok((
                    key.value().0.to_owned(),
                    serde_json::from_str(value.value()).map_err(|_| Error::Storage)?,
                ))
            })
            .collect()
    }
    /// Inspect committed event payloads without promising cursor ordering.
    #[cfg(feature = "test-support")]
    pub fn event_rows(&self) -> Result<Vec<Row>> {
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let table = tx.open_table(EVENTS).map_err(|_| Error::Storage)?;
        table
            .iter()
            .map_err(|_| Error::Storage)?
            .map(|entry| {
                let (_, value) = entry.map_err(|_| Error::Storage)?;
                serde_json::from_str(value.value()).map_err(|_| Error::Storage)
            })
            .collect()
    }
}
