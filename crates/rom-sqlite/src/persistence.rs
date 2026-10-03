//! The native atomic Resource bundle and authoritative read interface.
use crate::{Sqlite, references};
use rom::{
    Bundle, Capabilities, Descriptor, Error, JournalCursor, JournalPage, Key, Receipt, Result, Row,
    Storage, StorageState, WorkRecord, WorkResult, WorkUpdate,
};
use rusqlite::{Connection, OptionalExtension, params};
#[cfg(feature = "test-support")]
use std::sync::atomic::Ordering;

pub(crate) fn row(c: &Connection, key: &Key) -> Result<Option<Row>> {
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
pub(crate) fn receipt(c: &Connection, id: &str) -> Result<Option<Receipt>> {
    let text: Option<String> = c
        .query_row("SELECT data FROM receipts WHERE identity=?", [id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| Error::Storage)?;
    text.map(|t| serde_json::from_str(&t).map_err(|_| Error::Storage))
        .transpose()
}
pub(crate) fn state(c: &Connection) -> Result<StorageState> {
    let text: String = c
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    serde_json::from_str(&text).map_err(|_| Error::Storage)
}
pub(crate) fn save_state(c: &Connection, state: &StorageState) -> Result<()> {
    c.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(state).map_err(|_| Error::Storage)?],
    )
    .map_err(|_| Error::NotCommitted)?;
    Ok(())
}
impl Storage for Sqlite {
    fn register(&self, descriptors: &[Descriptor]) -> Result<()> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| Error::Storage)?;
        references::register(&tx, descriptors)?;
        tx.commit().map_err(|_| Error::Unknown)
    }
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
        let targets = references::prepare(&tx, &b.receipt)?;
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
            references::replace(self, &tx, &r.key, &targets, &mut ordinal)?;
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
            ordinal += 1;
            self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
        }
        save_state(&tx, &metadata)?;
        ordinal += 1;
        self.checkpoint(ordinal).map_err(|_| Error::NotCommitted)?;
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
