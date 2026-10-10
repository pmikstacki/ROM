//! Exact read-set validation and bounded native publication in one transaction.
use super::Reader;
use rom::storage_support::{
    metadata::StorageMetadata,
    work::{ReadFence, WorkDelta, WorkImage, WorkRead},
};
use rom::{Error, Result, StorageState, WorkResult};
use rusqlite::{Connection, params};

pub(crate) fn save_metadata(c: &Connection, metadata: &StorageMetadata) -> Result<()> {
    let raw = serde_json::to_string(metadata).map_err(|_| Error::Storage)?;
    save_metadata_raw(c, &raw)
}
pub(super) fn save_metadata_raw(c: &Connection, raw: &str) -> Result<()> {
    c.execute("UPDATE rom_state SET data=? WHERE id=1", [raw])
        .map_err(|_| Error::NotCommitted)?;
    Ok(())
}
/// Full import is reserved for fresh restore and explicit operator maintenance.
pub(crate) fn replace_state(c: &Connection, state: &StorageState) -> Result<()> {
    let format: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    match format {
        11 => replace_state_for_layout(c, state, true),
        super::PREDECESSOR_FORMAT => replace_state_for_layout(c, state, false),
        _ => Err(Error::Unsupported("SQLite storage format".into())),
    }
}
pub(crate) fn replace_state_for_layout(
    c: &Connection,
    state: &StorageState,
    keyed_journal: bool,
) -> Result<()> {
    let (metadata, work, operator) = StorageMetadata::split(state.clone());
    let image = WorkImage::from_ledger(work, metadata.retry_epochs())?;
    c.execute_batch("DELETE FROM work_records; DELETE FROM work_roots; DELETE FROM work_active;")
        .map_err(|_| Error::NotCommitted)?;
    if keyed_journal {
        crate::native_journal::replace(c, metadata)?;
    } else {
        save_metadata(c, &metadata)?;
    }
    c.execute("INSERT INTO operator_state(id,data) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", [serde_json::to_string(&operator).map_err(|_| Error::Storage)?]).map_err(|_| Error::NotCommitted)?;
    c.execute("INSERT INTO work_header(id,data) VALUES(1,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", [serde_json::to_string(&image.header()?.parts()).map_err(|_| Error::Storage)?]).map_err(|_| Error::NotCommitted)?;
    for (id, record) in image.records() {
        c.execute(
            "INSERT INTO work_records VALUES(?,?)",
            params![
                id,
                serde_json::to_string(record).map_err(|_| Error::Storage)?
            ],
        )
        .map_err(|_| Error::NotCommitted)?;
    }
    for (id, root) in image.roots() {
        c.execute(
            "INSERT INTO work_roots VALUES(?,?)",
            params![id, serde_json::to_string(root).map_err(|_| Error::Storage)?],
        )
        .map_err(|_| Error::NotCommitted)?;
    }
    for id in image.active_ids() {
        c.execute("INSERT INTO work_active VALUES(?)", [id])
            .map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}
impl Reader<'_> {
    pub(crate) fn apply(
        &mut self,
        delta: WorkDelta,
        mut checkpoint: impl FnMut() -> Result<()>,
    ) -> Result<WorkResult> {
        delta.validate(self)?;
        let result = self.publish_work(super::PreparedWork::encode(delta)?, &mut checkpoint)?;
        self.fence = ReadFence::new();
        Ok(result)
    }
    pub(crate) fn publish_work(
        &self,
        prepared: super::PreparedWork,
        mut checkpoint: impl FnMut() -> Result<()>,
    ) -> Result<WorkResult> {
        for (id, raw, active) in &prepared.records {
            self.connection.execute("INSERT INTO work_records VALUES(?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", params![id, &raw]).map_err(|_| Error::NotCommitted)?;
            #[cfg(feature = "test-support")]
            self.record_publication(crate::PublicationCategory::WorkRecord, raw.len());
            checkpoint()?;
            if *active {
                self.connection.execute(
                    "INSERT INTO work_active VALUES(?) ON CONFLICT(id) DO NOTHING",
                    [id],
                )
            } else {
                self.connection
                    .execute("DELETE FROM work_active WHERE id=?", [id])
            }
            .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        for (id, raw) in &prepared.roots {
            self.connection.execute("INSERT INTO work_roots VALUES(?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data", params![id, &raw]).map_err(|_| Error::NotCommitted)?;
            #[cfg(feature = "test-support")]
            self.record_publication(crate::PublicationCategory::WorkRoot, raw.len());
            checkpoint()?;
        }
        let raw = prepared.header;
        self.connection
            .execute("UPDATE work_header SET data=? WHERE id=1", [&raw])
            .map_err(|_| Error::NotCommitted)?;
        #[cfg(feature = "test-support")]
        self.record_publication(crate::PublicationCategory::WorkHeader, raw.len());
        checkpoint()?;
        Ok(prepared.delta.into_result())
    }
}
