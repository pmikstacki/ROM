use notification_core::{DeliveryId, Intent, Outbox, Outcome, Result, Work};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::Path;

pub struct SqliteOutbox {
    connection: Connection,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    pub state: String,
    pub attempts: u32,
    pub next_at: u64,
    pub receipt: Option<String>,
    pub last_outcome: Option<String>,
}

fn error(err: rusqlite::Error) -> String {
    err.to_string()
}
fn tick(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| "clock overflow".into())
}

impl SqliteOutbox {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path).map_err(error)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS resource (id TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS outbox (
                id TEXT PRIMARY KEY, channel TEXT NOT NULL, schema TEXT NOT NULL,
                payload BLOB NOT NULL, state TEXT NOT NULL DEFAULT 'pending',
                attempts INTEGER NOT NULL DEFAULT 0, next_at INTEGER NOT NULL DEFAULT 0,
                receipt TEXT, last_outcome TEXT
             );",
            )
            .map_err(error)?;
        Ok(Self { connection })
    }

    /// Scratch action transaction, intentionally separate from the persistence probe.
    pub fn transition(&mut self, resource: &str, value: &str, intents: &[Intent]) -> Result<()> {
        if intents.len() > 32
            || value.len() > 65536
            || intents.iter().any(|i| i.payload.len() > 65536)
        {
            return Err("transition too large".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        tx.execute("INSERT INTO resource(id,value) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value", params![resource, value]).map_err(error)?;
        for intent in intents {
            tx.execute(
                "INSERT INTO outbox(id,channel,schema,payload) VALUES (?1,?2,?3,?4)",
                params![intent.id.0, intent.channel, intent.schema, intent.payload],
            )
            .map_err(error)?;
        }
        tx.commit().map_err(error)
    }

    pub fn resource(&self, id: &str) -> Result<Option<String>> {
        self.connection
            .query_row("SELECT value FROM resource WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(error)
    }

    pub fn status(&self, id: &str) -> Result<Option<Status>> {
        self.connection
            .query_row(
                "SELECT state,attempts,next_at,receipt,last_outcome FROM outbox WHERE id=?1",
                [id],
                |row| {
                    let next_at: i64 = row.get(2)?;
                    let next_at = u64::try_from(next_at)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, next_at))?;
                    Ok(Status {
                        state: row.get(0)?,
                        attempts: row.get(1)?,
                        next_at,
                        receipt: row.get(3)?,
                        last_outcome: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(error)
    }
}

impl Outbox for SqliteOutbox {
    fn claim(&mut self, now: u64, lease: u64, max_attempts: u32) -> Result<Option<Work>> {
        let deadline = tick(now.checked_add(lease).ok_or("clock overflow")?)?;
        let now = tick(now)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        // A worker lost on its final attempt cannot be retried indefinitely.
        tx.execute("UPDATE outbox SET state='failed',last_outcome='unknown' WHERE state IN ('pending','leased') AND next_at<=?1 AND attempts>=?2", params![now, max_attempts]).map_err(error)?;
        let work = tx.query_row(
            "SELECT id,channel,schema,payload,attempts FROM outbox WHERE state IN ('pending','leased') AND next_at<=?1 AND attempts<?2 ORDER BY next_at,id LIMIT 1",
            params![now, max_attempts], |row| Ok(Work {
                intent: Intent { id: DeliveryId(row.get(0)?), channel: row.get(1)?, schema: row.get(2)?, payload: row.get(3)? },
                attempt: row.get::<_, u32>(4)? + 1,
            })
        ).optional().map_err(error)?;
        if let Some(work) = &work {
            tx.execute("UPDATE outbox SET state='leased',attempts=?2,next_at=?3,last_outcome='unknown' WHERE id=?1", params![work.intent.id.0, work.attempt, deadline]).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(work)
    }

    fn finish(
        &mut self,
        work: &Work,
        outcome: Outcome,
        next_at: u64,
        max_attempts: u32,
    ) -> Result<()> {
        let (state, receipt, label) = match outcome {
            Outcome::Accepted(receipt) => ("accepted", Some(receipt), "accepted"),
            Outcome::Permanent => ("failed", None, "permanent"),
            Outcome::Retryable => (
                if work.attempt >= max_attempts {
                    "failed"
                } else {
                    "pending"
                },
                None,
                "retryable",
            ),
            Outcome::Unknown => (
                if work.attempt >= max_attempts {
                    "failed"
                } else {
                    "pending"
                },
                None,
                "unknown",
            ),
        };
        let changed = self.connection.execute(
            "UPDATE outbox SET state=?3,receipt=?4,last_outcome=?5,next_at=?6 WHERE id=?1 AND attempts=?2 AND state='leased'",
            params![work.intent.id.0, work.attempt, state, receipt, label, tick(next_at)?]
        ).map_err(error)?;
        if changed != 1 {
            return Err("stale claim acknowledgment".into());
        }
        Ok(())
    }
}
