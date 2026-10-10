//! Native connection setup, validation and host inspection.
use crate::snapshot;
#[cfg(feature = "test-support")]
use rom::Row;
use rom::{Error, Result, StorageLimits, StorageState};
use rusqlite::Connection;
#[cfg(feature = "test-support")]
use std::sync::atomic::{AtomicU8, Ordering};
use std::{path::Path, sync::Mutex};

pub struct Sqlite {
    pub(crate) connection: Mutex<Connection>,
    pub(crate) ownership: rom::StorageOwnership,
    pub(crate) validation_limits: rom_backup::BackupLimits,
    pub(crate) journal_candidate: bool,
    // Declared after the engine so the native connection closes before exclusion ends.
    _native_owner: Option<rom_backup::NativeOwnership>,
    #[cfg(feature = "test-support")]
    pub(crate) fault: AtomicU8,
    #[cfg(feature = "test-support")]
    observer: Mutex<Option<Observer>>,
    #[cfg(feature = "test-support")]
    pub(crate) stage_observation: std::sync::OnceLock<crate::stage_observation::StageObservation>,
}
#[cfg(feature = "test-support")]
type Observer = std::sync::Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
impl Sqlite {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_limits(path, StorageLimits::default())
    }
    pub fn open_with_limits(path: impl AsRef<Path>, limits: StorageLimits) -> Result<Self> {
        Self::open_with_validation_limits(path, limits, rom_backup::BackupLimits::default())
    }
    /// Validate all persisted schemas and references before accepting an existing store.
    /// The default open methods use the bounded maintenance limits; larger stores
    /// need an explicit host validation budget. No format upgrade or repair occurs.
    pub fn open_with_validation_limits(
        path: impl AsRef<Path>,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
    ) -> Result<Self> {
        let path = path.as_ref();
        if path.as_os_str().as_encoded_bytes().starts_with(b"file:") {
            return Err(Error::Unsupported(
                "SQLite URI paths are unsupported; use a filesystem path".into(),
            ));
        }
        // Preserve SQLite's per-connection ephemeral path sentinels before canonicalization.
        if path.as_os_str().is_empty() || path == Path::new(":memory:") {
            return Self::open_connection(path, limits, validation_limits, None);
        }
        Self::open_owned(
            rom_backup::NativeOwnership::acquire(path, rom_backup::NativeAccess::OpenOrCreate)?,
            limits,
            validation_limits,
        )
    }
    pub(crate) fn open_owned(
        owner: rom_backup::NativeOwnership,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
    ) -> Result<Self> {
        let path = owner.path().to_owned();
        Self::open_connection(&path, limits, validation_limits, Some(owner))
    }
    fn open_connection(
        path: &Path,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
        owner: Option<rom_backup::NativeOwnership>,
    ) -> Result<Self> {
        Self::open_connection_for_layout(path, limits, validation_limits, owner, true)
    }
    pub(crate) fn open_connection_for_layout(
        path: &Path,
        limits: StorageLimits,
        validation_limits: rom_backup::BackupLimits,
        owner: Option<rom_backup::NativeOwnership>,
        journal_candidate: bool,
    ) -> Result<Self> {
        let mut c = Connection::open(path).map_err(|_| Error::Storage)?;
        let version: u32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| Error::Storage)?;
        let objects: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name NOT GLOB 'sqlite_*'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| Error::Storage)?;
        let expected_format = if journal_candidate {
            crate::native_work::FORMAT
        } else {
            crate::native_work::PREDECESSOR_FORMAT
        };
        if version != expected_format && (version != 0 || objects != 0) {
            return Err(Error::Unsupported("SQLite storage format".into()));
        }
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| Error::Storage)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        if version == 0 {
            tx.execute_batch("CREATE TABLE resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));
                CREATE TABLE receipts(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
                CREATE TABLE events(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
                CREATE TABLE effects(identity TEXT NOT NULL,ordinal INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(identity,ordinal));
                CREATE TABLE rom_state(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL);
                CREATE TABLE schemas(kind TEXT PRIMARY KEY NOT NULL,data TEXT NOT NULL);
                CREATE TABLE reference_edges(source_kind TEXT NOT NULL,source_id TEXT NOT NULL,target_kind TEXT NOT NULL,target_id TEXT NOT NULL,PRIMARY KEY(source_kind,source_id,target_kind,target_id));
                CREATE INDEX reference_edges_target ON reference_edges(target_kind,target_id,source_kind,source_id);
").map_err(|_|Error::Storage)?;
            crate::index::initialize(&tx)?;
            crate::native_work::initialize(&tx)?;
            if journal_candidate {
                crate::native_journal::initialize(&tx)?;
            }
            tx.execute("INSERT INTO rom_state VALUES (1,'')", [])
                .map_err(|_| Error::Storage)?;
            crate::native_work::replace_state_for_layout(
                &tx,
                &StorageState::new(limits.clone())?,
                journal_candidate,
            )?;
            tx.pragma_update(None, "user_version", expected_format)
                .map_err(|_| Error::Storage)?;
        }
        let initialized =
            snapshot::collect_native_snapshot(&tx, validation_limits, journal_candidate)?;
        initialized.validate()?;
        initialized.state.check_limits(&limits)?;
        tx.commit().map_err(|_| Error::Unknown)?;
        Ok(Self {
            connection: Mutex::new(c),
            ownership: rom::StorageOwnership::default(),
            validation_limits,
            journal_candidate,
            _native_owner: owner,
            #[cfg(feature = "test-support")]
            fault: AtomicU8::new(0),
            #[cfg(feature = "test-support")]
            observer: Mutex::new(None),
            #[cfg(feature = "test-support")]
            stage_observation: std::sync::OnceLock::new(),
        })
    }
    /// Close the engine before releasing its native ownership reservation.
    pub(crate) fn close(self) -> Result<()> {
        let Self {
            connection,
            _native_owner,
            ..
        } = self;
        let result = match connection.into_inner() {
            Ok(connection) => connection.close().map_err(|(connection, _)| {
                drop(connection);
                Error::Storage
            }),
            Err(poisoned) => {
                drop(poisoned.into_inner());
                Err(Error::Panicked)
            }
        };
        drop(_native_owner);
        result
    }
    /// 1: after state; 2: after event; 3: after receipt; 4: after effects;
    /// 5: actual commit succeeds but acknowledgment is lost. One next commit only.
    #[cfg(feature = "test-support")]
    pub fn inject_fault(&self, point: u8) {
        self.fault.store(point, Ordering::SeqCst);
    }
    /// Test-only observer: 1-based write ordinal, 0 before commit, usize::MAX after commit.
    #[cfg(feature = "test-support")]
    pub fn on_commit(&self, observer: Option<Observer>) {
        *self.observer.lock().unwrap() = observer;
    }
    pub(crate) fn checkpoint(&self, point: usize) -> Result<()> {
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
    /// Test inspection, not an ordered journal cursor API.
    #[cfg(feature = "test-support")]
    pub fn event_rows(&self) -> Result<Vec<Row>> {
        let c = self.connection.lock().map_err(|_| Error::Storage)?;
        let mut s = c
            .prepare("SELECT data FROM events ORDER BY identity")
            .map_err(|_| Error::Storage)?;
        let rows = s
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| Error::Storage)?;
        rows.map(|r| {
            serde_json::from_str(&r.map_err(|_| Error::Storage)?).map_err(|_| Error::Storage)
        })
        .collect()
    }
    pub fn counts(&self) -> Result<[u64; 4]> {
        let c = self.connection.lock().unwrap();
        let mut result = [0; 4];
        for (i, table) in ["resources", "events", "receipts", "effects"]
            .iter()
            .enumerate()
        {
            result[i] = c
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| {
                    r.get::<_, i64>(0)
                })
                .map_err(|_| Error::Storage)? as u64;
        }
        Ok(result)
    }
    pub fn intentions(&self) -> Result<Vec<(String, rom::Intent)>> {
        let c = self.connection.lock().unwrap();
        let mut s = c
            .prepare("SELECT identity,data FROM effects ORDER BY identity,ordinal")
            .map_err(|_| Error::Storage)?;
        let rows = s
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|_| Error::Storage)?;
        rows.map(|r| {
            let (id, data) = r.map_err(|_| Error::Storage)?;
            Ok((id, serde_json::from_str(&data).map_err(|_| Error::Storage)?))
        })
        .collect()
    }
}
