//! Coherent keyed reads inside the original SQLite transaction.
use rom::storage_support::{
    metadata::StorageMetadata,
    work::{ReadFence, RootAccount, WorkHeader, WorkHeaderParts, WorkRead},
};
use rom::{Error, Result, WorkRecord, WorkState};
use rusqlite::Connection;
use std::cell::Cell;

pub(crate) fn metadata(c: &Connection) -> Result<StorageMetadata> {
    let text: String = c
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    serde_json::from_str(&text).map_err(|_| Error::Storage)
}
pub(super) fn header(c: &Connection) -> Result<WorkHeader> {
    let text: String = c
        .query_row("SELECT data FROM work_header WHERE id=1", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    let parts: WorkHeaderParts = serde_json::from_str(&text).map_err(|_| Error::Storage)?;
    WorkHeader::from_parts(parts)
}
pub(crate) struct Reader<'tx> {
    #[cfg(feature = "test-support")]
    pub(super) publication: Option<(&'tx crate::StageObservation, crate::StageOperation)>,
    pub(crate) connection: &'tx Connection,
    pub(crate) fence: ReadFence,
    pub(crate) examined: Cell<usize>,
    pub(crate) decoded: Cell<usize>,
    pub(crate) budget: Option<rom_backup::BackupLimits>,
    physical_bytes: Cell<usize>,
    physical_records: Cell<usize>,
}
impl<'tx> Reader<'tx> {
    pub(crate) fn new(connection: &'tx Connection) -> Self {
        Self {
            #[cfg(feature = "test-support")]
            publication: None,
            connection,
            fence: ReadFence::new(),
            examined: Cell::new(0),
            decoded: Cell::new(0),
            budget: None,
            physical_bytes: Cell::new(0),
            physical_records: Cell::new(0),
        }
    }
    pub(crate) fn bounded(connection: &'tx Connection, limits: rom_backup::BackupLimits) -> Self {
        let mut reader = Self::new(connection);
        reader.budget = Some(limits);
        reader
    }
    pub(crate) fn charge(&self, bytes: usize, records: usize) -> Result<()> {
        let Some(limits) = self.budget else {
            return Ok(());
        };
        let next_bytes = self
            .physical_bytes
            .get()
            .checked_add(bytes)
            .ok_or(Error::TooLarge)?;
        let next_records = self
            .physical_records
            .get()
            .checked_add(records)
            .ok_or(Error::TooLarge)?;
        if next_bytes > limits.max_bytes || next_records > limits.max_records {
            return Err(Error::TooLarge);
        }
        self.physical_bytes.set(next_bytes);
        self.physical_records.set(next_records);
        Ok(())
    }
    fn text(&self, sql: &str, key: Option<&str>) -> Result<Option<String>> {
        let mut statement = self.connection.prepare(sql).map_err(|_| Error::Storage)?;
        let mut rows = if let Some(key) = key {
            statement.query([key])
        } else {
            statement.query([])
        }
        .map_err(|_| Error::Storage)?;
        let Some(row) = rows.next().map_err(|_| Error::Storage)? else {
            return Ok(None);
        };
        let text = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        self.charge(
            text.len()
                .checked_add(key.map_or(1, str::len))
                .ok_or(Error::TooLarge)?,
            1,
        )?;
        Ok(Some(text.to_owned()))
    }
    fn decode(&self, id: &str, text: &str) -> Result<WorkRecord> {
        let record: WorkRecord = serde_json::from_str(text).map_err(|_| Error::Storage)?;
        if record.pending.id != id {
            return Err(Error::Storage);
        }
        self.decoded
            .set(self.decoded.get().checked_add(1).ok_or(Error::TooLarge)?);
        Ok(record)
    }
}
impl WorkRead for Reader<'_> {
    fn coherence(&self) -> ReadFence {
        self.fence.clone()
    }
    fn header(&self) -> Result<WorkHeader> {
        if self.budget.is_none() {
            return header(self.connection);
        }
        let raw = self
            .text("SELECT data FROM work_header WHERE id=1", None)?
            .ok_or(Error::Storage)?;
        WorkHeader::from_parts(serde_json::from_str(&raw).map_err(|_| Error::Storage)?)
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        let text = self.text("SELECT data FROM work_records WHERE id=?", Some(id))?;
        text.map(|text| self.decode(id, &text)).transpose()
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        let text = self.text("SELECT data FROM work_roots WHERE id=?", Some(id))?;
        text.map(|text| {
            let root: RootAccount = serde_json::from_str(&text).map_err(|_| Error::Storage)?;
            root.validate()?;
            Ok(root)
        })
        .transpose()
    }
    fn next_candidate(&self, now: u64, after: Option<&str>) -> Result<Option<WorkRecord>> {
        let Some(limits) = self.header()?.limits else {
            return Ok(None);
        };
        let sql = if after.is_some() {
            "SELECT a.id,w.data FROM work_active a LEFT JOIN work_records w ON w.id=a.id WHERE a.id>? ORDER BY a.id"
        } else {
            "SELECT a.id,w.data FROM work_active a LEFT JOIN work_records w ON w.id=a.id ORDER BY a.id"
        };
        let mut statement = self.connection.prepare(sql).map_err(|_| Error::Storage)?;
        let mut rows = if let Some(after) = after {
            statement.query([after])
        } else {
            statement.query([])
        }
        .map_err(|_| Error::Storage)?;
        while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            self.examined
                .set(self.examined.get().checked_add(1).ok_or(Error::TooLarge)?);
            let id = row
                .get_ref(0)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?;
            let text = row
                .get_ref(1)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?;
            self.charge(
                id.len()
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(text.len()))
                    .ok_or(Error::TooLarge)?,
                2,
            )?;
            let record = self.decode(id, text)?;
            let eligible = match record.state {
                WorkState::Pending => {
                    record.due <= now
                        || now.saturating_sub(record.pending.cause.started_at)
                            >= limits.max_age_seconds
                }
                WorkState::Leased { until, .. } => until <= now,
                _ => return Err(Error::Storage),
            };
            if eligible {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }
}
