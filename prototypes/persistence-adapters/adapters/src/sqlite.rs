use crate::common::*;
use rom_persistence_core::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
    #[cfg(feature = "fault-injection")]
    probe: crate::probe::Probe,
}
fn unavailable(_: rusqlite::Error) -> Error {
    Error::Unavailable
}

impl SqliteStore {
    #[cfg(feature = "fault-injection")]
    pub fn with_probe(mut self, probe: crate::probe::Probe) -> Self {
        self.probe = probe;
        self
    }
    pub fn open(path: &Path) -> Result<Self, Error> {
        let mut connection = Connection::open(path).map_err(unavailable)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(unavailable)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(unavailable)?;
        let mode: String = connection
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(unavailable)?;
        let sync: i64 = connection
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .map_err(unavailable)?;
        if mode != "wal" || sync != 2 {
            return Err(Error::Unsupported("durable WAL configuration"));
        }
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS resources (key BLOB PRIMARY KEY, data BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS receipts (action TEXT PRIMARY KEY, data BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS events (sequence INTEGER PRIMARY KEY, data BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS metadata (id INTEGER PRIMARY KEY CHECK(id=1), data BLOB NOT NULL);").map_err(unavailable)?;
        if tx
            .query_row("SELECT data FROM metadata WHERE id=1", [], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .optional()
            .map_err(unavailable)?
            .is_none()
        {
            tx.execute(
                "INSERT INTO metadata VALUES (1, ?1)",
                [encode(&Meta::new()?)?],
            )
            .map_err(unavailable)?;
        }
        tx.commit().map_err(unavailable)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            #[cfg(feature = "fault-injection")]
            probe: crate::probe::Probe::default(),
        })
    }
    pub fn engine_version() -> String {
        rusqlite::version().into()
    }
}

impl Storage for SqliteStore {
    fn capabilities(&self) -> Capabilities {
        CAPABILITIES
    }
    fn load(&self, key: &ResourceKey) -> Result<Option<Resource>, Error> {
        let connection = self.connection.lock().map_err(|_| Error::Unavailable)?;
        let bytes = connection
            .query_row(
                "SELECT data FROM resources WHERE key=?1",
                [encode(key)?],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(unavailable)?;
        bytes.map(|b| decode(&b)).transpose()
    }
    fn receipt(&self, action: &str) -> Result<ReceiptStatus, Error> {
        let connection = self.connection.lock().map_err(|_| Error::Unavailable)?;
        let bytes = connection
            .query_row("SELECT data FROM receipts WHERE action=?1", [action], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .optional()
            .map_err(unavailable)?;
        Ok(match bytes {
            Some(b) => ReceiptStatus::Found(decode::<StoredReceipt>(&b)?.receipt),
            None => ReceiptStatus::AbsentNow,
        })
    }
    fn commit(&self, transition: &Transition) -> Result<Receipt, Error> {
        transition.validate(self.capabilities())?;
        let mut connection = self.connection.lock().map_err(|_| Error::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(unavailable)?;
        if let Some(bytes) = tx
            .query_row(
                "SELECT data FROM receipts WHERE action=?1",
                [&transition.action],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(unavailable)?
        {
            return decode::<StoredReceipt>(&bytes)?.resolve(transition);
        }
        let key = encode(&transition.key)?;
        let current = tx
            .query_row("SELECT data FROM resources WHERE key=?1", [&key], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .optional()
            .map_err(unavailable)?
            .map(|b| decode::<Resource>(&b))
            .transpose()?;
        let (resource, receipt, events) = transition.materialize(current.map(|r| r.revision))?;
        let mut meta: Meta = decode(
            &tx.query_row("SELECT data FROM metadata WHERE id=1", [], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .map_err(unavailable)?,
        )?;
        tx.execute("INSERT INTO resources VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![key, encode(&resource)?]).map_err(unavailable)?;
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::AfterResource)?;
        tx.execute(
            "INSERT INTO receipts VALUES (?1, ?2)",
            params![
                transition.action,
                encode(&StoredReceipt {
                    transition: transition.clone(),
                    receipt: receipt.clone()
                })?
            ],
        )
        .map_err(unavailable)?;
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::AfterReceipt)?;
        for event in events {
            tx.execute(
                "INSERT INTO events VALUES (?1, ?2)",
                params![
                    i64::try_from(meta.allocate()?)
                        .map_err(|_| Error::Rejected("journal capacity"))?,
                    encode(&event)?
                ],
            )
            .map_err(unavailable)?;
            #[cfg(feature = "fault-injection")]
            self.probe.hit(crate::probe::Checkpoint::AfterEvent)?;
        }
        tx.execute("UPDATE metadata SET data=?1 WHERE id=1", [encode(&meta)?])
            .map_err(unavailable)?;
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::BeforeCommit)?;
        // Conservatively classify any commit error: never promise rollback here.
        tx.commit().map_err(|_| Error::Unknown {
            action: transition.action.clone(),
        })?;
        #[cfg(feature = "fault-injection")]
        self.probe.hit(crate::probe::Checkpoint::AfterCommit)?;
        Ok(receipt)
    }
    fn journal(&self, after: Option<&Cursor>, limit: usize) -> Result<Page, Error> {
        let mut connection = self.connection.lock().map_err(|_| Error::Unavailable)?;
        let tx = connection.transaction().map_err(unavailable)?;
        let meta: Meta = decode(
            &tx.query_row("SELECT data FROM metadata WHERE id=1", [], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .map_err(unavailable)?,
        )?;
        let mut position = meta.position(b'S', after, limit)?;
        let mut statement = tx
            .prepare(
                "SELECT sequence, data FROM events WHERE sequence>?1 ORDER BY sequence LIMIT ?2",
            )
            .map_err(unavailable)?;
        let rows = statement
            .query_map(
                params![
                    i64::try_from(position).map_err(|_| Error::InvalidCursor)?,
                    i64::try_from(limit).map_err(|_| Error::Rejected("page limit"))?
                ],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .map_err(unavailable)?;
        let mut events = vec![];
        for row in rows {
            let (sequence, bytes) = row.map_err(unavailable)?;
            events.push(decode(&bytes)?);
            position = u64::try_from(sequence).map_err(|_| Error::Unavailable)?;
        }
        Ok(Page {
            events,
            cursor: meta.cursor(b'S', position),
        })
    }
}
