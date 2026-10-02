//! redb persistence adapter with atomic Resource/event/receipt/effect bundles.
//!
//! The host must run these synchronous methods on its bounded storage executor.
//! Format version one stores JSON ROM values, using tuple keys for kind/id isolation.
//! A commit error is uncertain; discard the adapter and reopen before recovery.
use redb::{
    Database, Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
};
use rom::{Bundle, Capabilities, Error, Intent, Key, Receipt, Result, Row, Storage};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const ROWS: TableDefinition<(&str, &str), &str> = TableDefinition::new("resources");
const RECEIPTS: TableDefinition<&str, &str> = TableDefinition::new("receipts");
const EVENTS: TableDefinition<&str, &str> = TableDefinition::new("events");
const EFFECTS: TableDefinition<(&str, u64), &str> = TableDefinition::new("effects");
const META: TableDefinition<&str, u64> = TableDefinition::new("rom_metadata");
const FORMAT: u64 = 1;
#[cfg(feature = "test-support")]
type Observer = std::sync::Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;

/// One database handle; clones should be shared using `Arc`, not reopened concurrently.
pub struct Redb {
    db: Database,
    uncertain: AtomicBool,
    commit_gate: std::sync::Mutex<()>,
    #[cfg(feature = "test-support")]
    observer: std::sync::Mutex<Option<Observer>>,
}
impl Redb {
    /// Open format one, or initialize a new empty database. Never upgrade implicitly.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path).map_err(|_| Error::Storage)?;
        let read = db.begin_read().map_err(|_| Error::Storage)?;
        let empty = read
            .list_tables()
            .map_err(|_| Error::Storage)?
            .next()
            .is_none();
        if !empty {
            let meta = read
                .open_table(META)
                .map_err(|_| Error::Unsupported("redb format marker missing".into()))?;
            if meta
                .get("format")
                .map_err(|_| Error::Storage)?
                .map(|x| x.value())
                != Some(FORMAT)
            {
                return Err(Error::Unsupported("redb storage format".into()));
            }
            read.open_table(ROWS).map_err(|_| Error::Storage)?;
            read.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
            read.open_table(EVENTS).map_err(|_| Error::Storage)?;
            read.open_table(EFFECTS).map_err(|_| Error::Storage)?;
        }
        drop(read);
        if empty {
            let mut tx = db.begin_write().map_err(|_| Error::Storage)?;
            tx.set_durability(Durability::Immediate)
                .map_err(|_| Error::Storage)?;
            tx.open_table(ROWS).map_err(|_| Error::Storage)?;
            tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
            tx.open_table(EVENTS).map_err(|_| Error::Storage)?;
            tx.open_table(EFFECTS).map_err(|_| Error::Storage)?;
            tx.open_table(META)
                .map_err(|_| Error::Storage)?
                .insert("format", FORMAT)
                .map_err(|_| Error::Storage)?;
            tx.commit().map_err(|_| Error::Unknown)?;
        }
        Ok(Self {
            db,
            uncertain: AtomicBool::new(false),
            commit_gate: std::sync::Mutex::new(()),
            #[cfg(feature = "test-support")]
            observer: std::sync::Mutex::new(None),
        })
    }
    fn available(&self) -> Result<()> {
        if self.uncertain.load(Ordering::Acquire) {
            Err(Error::Unknown)
        } else {
            Ok(())
        }
    }
    /// Test-only transaction observer: 1-based write ordinal; 0 before commit;
    /// usize::MAX after commit. Returning an error before commit rolls back.
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
impl Storage for Redb {
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
    fn commit(&self, b: &Bundle) -> Result<Receipt> {
        // Keep the uncertain-state update serialized with other writers, including
        // the interval after redb releases its internal transaction lock.
        let _writer = self.commit_gate.lock().map_err(|_| Error::Storage)?;
        self.available()?;
        let mut tx = self.db.begin_write().map_err(|_| Error::Storage)?;
        // Another writer may have made an uncertain commit while this one waited.
        self.available()?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        {
            let mut receipts = tx.open_table(RECEIPTS).map_err(|_| Error::Storage)?;
            if let Some(prior) = receipts
                .get(b.receipt.identity.as_str())
                .map_err(|_| Error::Storage)?
            {
                let prior: Receipt =
                    serde_json::from_str(prior.value()).map_err(|_| Error::Storage)?;
                return if prior.fingerprint == b.receipt.fingerprint {
                    Ok(prior)
                } else {
                    Err(Error::IdentityMismatch)
                };
            }
            let mut rows = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
            let key = (
                b.receipt.row.key.kind.as_str(),
                b.receipt.row.key.id.as_str(),
            );
            let existing: Option<Row> = rows
                .get(key)
                .map_err(|_| Error::Storage)?
                .map(|v| serde_json::from_str(v.value()).map_err(|_| Error::Storage))
                .transpose()?;
            if existing.as_ref().map(|r| r.revision) != b.expected {
                return Err(Error::Conflict);
            }
            let revision = b
                .expected
                .unwrap_or(0)
                .checked_add(u64::from(b.changed))
                .ok_or(Error::TooLarge)?;
            if b.receipt.row.revision != revision
                || (!b.changed
                    && (!b.effects.is_empty() || existing.as_ref() != Some(&b.receipt.row)))
            {
                return Err(Error::NotCommitted);
            }
            // Common profile matches SQLite's representable revision range.
            i64::try_from(revision).map_err(|_| Error::TooLarge)?;
            let row = serde_json::to_string(&b.receipt.row).map_err(|_| Error::NotCommitted)?;
            let receipt = serde_json::to_string(&b.receipt).map_err(|_| Error::NotCommitted)?;
            let mut ordinal = 0;
            if b.changed {
                rows.insert(key, row.as_str())
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
                tx.open_table(EVENTS)
                    .map_err(|_| Error::NotCommitted)?
                    .insert(b.receipt.identity.as_str(), row.as_str())
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            }
            receipts
                .insert(b.receipt.identity.as_str(), receipt.as_str())
                .map_err(|_| Error::NotCommitted)?;
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            let mut effects = tx.open_table(EFFECTS).map_err(|_| Error::NotCommitted)?;
            for (i, intent) in b.effects.iter().enumerate() {
                let payload = serde_json::to_string(intent).map_err(|_| Error::NotCommitted)?;
                effects
                    .insert(
                        (
                            b.receipt.identity.as_str(),
                            u64::try_from(i).map_err(|_| Error::TooLarge)?,
                        ),
                        payload.as_str(),
                    )
                    .map_err(|_| Error::NotCommitted)?;
                ordinal += 1;
                self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
            }
        }
        self.checkpoint(0).map_err(|_| Error::NotCommitted)?;
        if tx.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        self.checkpoint(usize::MAX).map_err(|_| Error::Unknown)?;
        Ok(b.receipt.clone())
    }
}
