//! SQLite reference persistence capability. The deployment database remains host-configured.
use rom::{
    Bundle, Capabilities, Error, JournalCursor, JournalPage, Key, Receipt, Result, Row, Storage,
    StorageLimits, StorageState, WorkRecord, WorkResult, WorkUpdate,
};
use rusqlite::{Connection, OptionalExtension, params};
#[cfg(feature = "test-support")]
use std::sync::atomic::{AtomicU8, Ordering};
use std::{path::Path, sync::Mutex};

pub struct Sqlite {
    connection: Mutex<Connection>,
    #[cfg(feature = "test-support")]
    fault: AtomicU8,
    #[cfg(feature = "test-support")]
    observer: Mutex<Option<Observer>>,
}
#[cfg(feature = "test-support")]
type Observer = std::sync::Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
impl Sqlite {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_limits(path, StorageLimits::default())
    }
    pub fn open_with_limits(path: impl AsRef<Path>, limits: StorageLimits) -> Result<Self> {
        let c = Connection::open(path).map_err(|_| Error::Storage)?;
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
        if version != 2 && (version != 0 || objects != 0) {
            return Err(Error::Unsupported("SQLite storage format".into()));
        }
        if version == 2 {
            for name in ["resources", "receipts", "events", "effects", "rom_state"] {
                c.prepare(&format!("SELECT * FROM {name} LIMIT 0"))
                    .map_err(|_| Error::Storage)?;
            }
        }
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));
            CREATE TABLE IF NOT EXISTS receipts(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS events(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS effects(identity TEXT NOT NULL,ordinal INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(identity,ordinal));
            CREATE TABLE IF NOT EXISTS rom_state(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL); PRAGMA user_version=2;").map_err(|_|Error::Storage)?;
        if version == 0 {
            c.execute(
                "INSERT INTO rom_state VALUES (1,?)",
                [serde_json::to_string(&StorageState::new(limits.clone())?)
                    .map_err(|_| Error::Storage)?],
            )
            .map_err(|_| Error::Storage)?;
        }
        state(&c)?.check_limits(&limits)?;
        c.execute_batch("COMMIT").map_err(|_| Error::Unknown)?;
        Ok(Self {
            connection: Mutex::new(c),
            #[cfg(feature = "test-support")]
            fault: AtomicU8::new(0),
            #[cfg(feature = "test-support")]
            observer: Mutex::new(None),
        })
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
    fn checkpoint(&self, point: usize) -> Result<()> {
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
fn row(c: &Connection, key: &Key) -> Result<Option<Row>> {
    let text: Option<String> = c
        .query_row(
            "SELECT data FROM resources WHERE kind=? AND id=?",
            params![key.kind, key.id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Error::Storage)?;
    text.map(|t| serde_json::from_str(&t).map_err(|_| Error::Storage))
        .transpose()
}
fn receipt(c: &Connection, id: &str) -> Result<Option<Receipt>> {
    let text: Option<String> = c
        .query_row("SELECT data FROM receipts WHERE identity=?", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| Error::Storage)?;
    text.map(|t| serde_json::from_str(&t).map_err(|_| Error::Storage))
        .transpose()
}
fn state(c: &Connection) -> Result<StorageState> {
    let text: String = c
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    serde_json::from_str(&text).map_err(|_| Error::Storage)
}
fn save_state(c: &Connection, state: &StorageState) -> Result<()> {
    c.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(state).map_err(|_| Error::Storage)?],
    )
    .map_err(|_| Error::NotCommitted)?;
    Ok(())
}
impl Storage for Sqlite {
    fn supports_reactions(&self) -> bool {
        true
    }
    fn reaction_records(&self) -> Result<Vec<WorkRecord>> {
        Ok(
            state(&*self.connection.lock().map_err(|_| Error::Panicked)?)?
                .work
                .records(),
        )
    }
    fn reaction_update(&self, update: WorkUpdate) -> Result<WorkResult> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        let mut s = state(&tx)?;
        let result = s.work.apply(update)?;
        save_state(&tx, &s)?;
        tx.commit().map_err(|_| Error::Unknown)?;
        Ok(result)
    }
    fn supports_journal(&self) -> bool {
        true
    }
    fn journal_head(&self, kind: &str) -> Result<JournalCursor> {
        Ok(state(&*self.connection.lock().map_err(|_| Error::Panicked)?)?.journal_head(kind))
    }
    fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        state(&*self.connection.lock().map_err(|_| Error::Panicked)?)?
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
        row(&self.connection.lock().unwrap(), key)
    }
    fn snapshot(&self, kind: &str, max_rows: usize, max_bytes: usize) -> Result<Vec<Row>> {
        let c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let mut statement = c
            .prepare("SELECT data FROM resources WHERE kind=? ORDER BY id LIMIT ?")
            .map_err(|_| Error::Storage)?;
        let limit = i64::try_from(max_rows.saturating_add(1)).unwrap_or(i64::MAX);
        let mut cursor = statement
            .query(params![kind, limit])
            .map_err(|_| Error::Storage)?;
        let mut result = Vec::new();
        let mut bytes = 0usize;
        while let Some(row) = cursor.next().map_err(|_| Error::Storage)? {
            if result.len() == max_rows {
                return Err(Error::TooLarge);
            }
            let text = row
                .get_ref(0)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?;
            bytes = bytes.checked_add(text.len()).ok_or(Error::TooLarge)?;
            if bytes > max_bytes {
                return Err(Error::TooLarge);
            }
            result.push(serde_json::from_str(text).map_err(|_| Error::Storage)?);
        }
        Ok(result)
    }
    fn receipt(&self, id: &str) -> Result<Option<Receipt>> {
        receipt(&self.connection.lock().unwrap(), id)
    }
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        let mut c = self.connection.lock().unwrap();
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        if let Some(prior) = receipt(&tx, &b.receipt.identity)? {
            if prior.fingerprint != b.receipt.fingerprint {
                return Err(Error::IdentityMismatch);
            }
            return Ok(prior);
        }
        let existing = row(&tx, &b.receipt.row.key)?;
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
        let mut metadata = state(&tx)?;
        let retired = metadata.bundle(b)?;
        #[cfg(feature = "test-support")]
        let f = self.fault.swap(0, Ordering::SeqCst);
        #[cfg(not(feature = "test-support"))]
        let f = 0;
        let mut ordinal = 0;
        if b.changed {
            let r = &b.receipt.row;
            tx.execute("INSERT INTO resources(kind,id,revision,data) VALUES (?,?,?,?) ON CONFLICT(kind,id) DO UPDATE SET revision=excluded.revision,data=excluded.data",params![r.key.kind,r.key.id,i64::try_from(r.revision).map_err(|_|Error::TooLarge)?,serde_json::to_string(r).unwrap()]).map_err(|_|Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        if f == 1 {
            return Err(Error::NotCommitted);
        }
        if b.changed {
            tx.execute(
                "INSERT INTO events(identity,data) VALUES (?,?)",
                params![
                    b.receipt.identity,
                    serde_json::to_string(&b.receipt.row).unwrap()
                ],
            )
            .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        if f == 2 {
            return Err(Error::NotCommitted);
        }
        tx.execute(
            "INSERT INTO receipts(identity,data) VALUES (?,?)",
            params![
                b.receipt.identity,
                serde_json::to_string(&b.receipt).unwrap()
            ],
        )
        .map_err(|_| Error::NotCommitted)?;
        ordinal += 1;
        self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        if f == 3 {
            return Err(Error::NotCommitted);
        }
        for (i, intent) in b.effects.iter().enumerate() {
            tx.execute(
                "INSERT INTO effects(identity,ordinal,data) VALUES (?,?,?)",
                params![
                    b.receipt.identity,
                    i as i64,
                    serde_json::to_string(intent).unwrap()
                ],
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
        }
        save_state(&tx, &metadata)?;
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        tx.commit().map_err(|_| Error::Unknown)?;
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        if f == 5 {
            Err(Error::Unknown)
        } else {
            Ok(b.receipt.clone())
        }
    }
}
