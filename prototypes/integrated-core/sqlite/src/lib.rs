//! Scratch SQLite capability. Not ROM's selected production database.
use rom_probe::{Bundle, Capabilities, Error, Key, Receipt, Result, Row, Storage};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

pub struct Sqlite {
    connection: Mutex<Connection>,
    fault: AtomicU8,
}
impl Sqlite {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let c = Connection::open(path).map_err(|_| Error::Storage)?;
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));
            CREATE TABLE IF NOT EXISTS receipts(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS events(identity TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS effects(identity TEXT NOT NULL,ordinal INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(identity,ordinal));").map_err(|_|Error::Storage)?;
        Ok(Self {
            connection: Mutex::new(c),
            fault: AtomicU8::new(0),
        })
    }
    /// 1: after state; 2: after event; 3: after receipt; 4: after effects;
    /// 5: actual commit succeeds but acknowledgment is lost. One next commit only.
    pub fn inject_fault(&self, point: u8) {
        self.fault.store(point, Ordering::SeqCst);
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
    pub fn intentions(&self) -> Result<Vec<(String, rom_probe::Intent)>> {
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
impl Storage for Sqlite {
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
    fn snapshot(&self, kind: &str) -> Result<Vec<Row>> {
        let c = self.connection.lock().unwrap();
        let mut s = c
            .prepare("SELECT data FROM resources WHERE kind=? ORDER BY id")
            .map_err(|_| Error::Storage)?;
        let rows = s
            .query_map([kind], |r| r.get::<_, String>(0))
            .map_err(|_| Error::Storage)?;
        rows.map(|r| {
            serde_json::from_str(&r.map_err(|_| Error::Storage)?).map_err(|_| Error::Storage)
        })
        .collect()
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
        let f = self.fault.swap(0, Ordering::SeqCst);
        if b.changed {
            let r = &b.receipt.row;
            tx.execute("INSERT INTO resources(kind,id,revision,data) VALUES (?,?,?,?) ON CONFLICT(kind,id) DO UPDATE SET revision=excluded.revision,data=excluded.data",params![r.key.kind,r.key.id,i64::try_from(r.revision).map_err(|_|Error::TooLarge)?,serde_json::to_string(r).unwrap()]).map_err(|_|Error::NotCommitted)?;
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
        }
        if f == 4 {
            return Err(Error::NotCommitted);
        }
        tx.commit().map_err(|_| Error::Unknown)?;
        if f == 5 {
            Err(Error::Unknown)
        } else {
            Ok(b.receipt.clone())
        }
    }
}
