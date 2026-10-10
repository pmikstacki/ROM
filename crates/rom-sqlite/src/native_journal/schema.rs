use rom::{Error, Result};
use rusqlite::Connection;

pub(crate) fn key(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}
pub(crate) fn initialize(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE journal_positions(position BLOB PRIMARY KEY NOT NULL CHECK(typeof(position)='blob' AND length(position)=8),identity TEXT NOT NULL,canonical_bytes BLOB NOT NULL CHECK(typeof(canonical_bytes)='blob' AND length(canonical_bytes)=8)) WITHOUT ROWID; CREATE UNIQUE INDEX journal_identity ON journal_positions(identity);")
        .map_err(|_| Error::Storage)
}

pub(crate) fn validate(c: &Connection) -> Result<()> {
    crate::index::layout::table(
        c,
        "journal_positions",
        true,
        &[
            ("position", "BLOB", true, 1),
            ("identity", "TEXT", true, 0),
            ("canonical_bytes", "BLOB", true, 0),
        ],
    )?;
    let mut statement = c
        .prepare(
            "SELECT name,\"unique\",origin,partial FROM pragma_index_list('journal_positions')",
        )
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    let mut primary = false;
    let mut identity = false;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let name = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let unique: i64 = row.get(1).map_err(|_| Error::Storage)?;
        let origin = row
            .get_ref(2)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let partial: i64 = row.get(3).map_err(|_| Error::Storage)?;
        if unique != 1 || partial != 0 {
            return Err(Error::Storage);
        }
        let expected = match origin {
            "pk" if !primary => {
                primary = true;
                [("position", 1), ("identity", 0), ("canonical_bytes", 0)].as_slice()
            }
            "c" if name == "journal_identity" && !identity => {
                identity = true;
                [("identity", 1), ("position", 0)].as_slice()
            }
            _ => return Err(Error::Storage),
        };
        let mut info = c
            .prepare("SELECT name,\"desc\",coll,\"key\" FROM pragma_index_xinfo(?) ORDER BY seqno")
            .map_err(|_| Error::Storage)?;
        let mut columns = info.query([name]).map_err(|_| Error::Storage)?;
        for (name, key) in expected {
            let column = columns
                .next()
                .map_err(|_| Error::Storage)?
                .ok_or(Error::Storage)?;
            let actual: String = column.get(0).map_err(|_| Error::Storage)?;
            let descending: i64 = column.get(1).map_err(|_| Error::Storage)?;
            let coll: String = column.get(2).map_err(|_| Error::Storage)?;
            let actual_key: i64 = column.get(3).map_err(|_| Error::Storage)?;
            if actual != *name || descending != 0 || coll != "BINARY" || actual_key != *key {
                return Err(Error::Storage);
            }
        }
        if columns.next().map_err(|_| Error::Storage)?.is_some() {
            return Err(Error::Storage);
        }
    }
    if !primary || !identity {
        return Err(Error::Storage);
    }
    Ok(())
}
