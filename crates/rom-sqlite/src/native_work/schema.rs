//! Keyed Work tables shared by the current and predecessor journal layouts.
use rom::{Error, Result};
use rusqlite::Connection;
pub(crate) const FORMAT: u32 = rom_backup::STORAGE_FORMAT;
pub(crate) const PREDECESSOR_FORMAT: u32 = 10;

pub(crate) fn initialize(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE work_header(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL);
CREATE TABLE operator_state(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL);
CREATE TABLE work_records(id TEXT PRIMARY KEY NOT NULL,data TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE work_roots(id TEXT PRIMARY KEY NOT NULL,data TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE work_active(id TEXT PRIMARY KEY NOT NULL) WITHOUT ROWID;",
    )
    .map_err(|_| Error::Storage)
}

pub(crate) fn validate_layout(c: &Connection) -> Result<()> {
    for table in ["rom_state", "work_header", "operator_state"] {
        crate::index::layout::table(
            c,
            table,
            false,
            &[("id", "INTEGER", false, 1), ("data", "TEXT", true, 0)],
        )?;
        let (count, id): (i64, Option<i64>) = c
            .query_row(&format!("SELECT COUNT(*),MIN(id) FROM {table}"), [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|_| Error::Storage)?;
        if count != 1 || id != Some(1) {
            return Err(Error::Storage);
        }
    }
    for table in ["work_records", "work_roots"] {
        crate::index::layout::table(
            c,
            table,
            true,
            &[("id", "TEXT", true, 1), ("data", "TEXT", true, 0)],
        )?;
        crate::index::layout::indexes(c, table, Some((&["id"], &["data"])), false)?;
    }
    crate::index::layout::table(c, "work_active", true, &[("id", "TEXT", true, 1)])?;
    crate::index::layout::indexes(c, "work_active", Some((&["id"], &[])), false)
}
