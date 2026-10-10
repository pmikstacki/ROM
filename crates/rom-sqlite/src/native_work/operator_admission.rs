//! Logical operator payload admission before Work decoding, separate from native budgets.
use rom::{Error, Result};
use rusqlite::Connection;

pub(crate) fn admit(c: &Connection, max_records: usize, max_bytes: usize) -> Result<()> {
    if max_records == 0 || max_bytes == 0 {
        return Err(Error::TooLarge);
    }
    let records: i64 = c
        .query_row("SELECT COUNT(*) FROM work_records", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if usize::try_from(records).map_err(|_| Error::Storage)? > max_records {
        return Err(Error::TooLarge);
    }
    let mut bytes = 0usize;
    // Only raw logical payload values enter this lower bound. Native ID keys,
    // accounting, root summaries, indexes and journal metadata do not enter it.
    for sql in [
        "SELECT data FROM work_records",
        "SELECT data FROM operator_state WHERE id=1",
    ] {
        let mut statement = c.prepare(sql).map_err(|_| Error::Storage)?;
        let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
        while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            let text = row
                .get_ref(0)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?;
            bytes = bytes.checked_add(text.len()).ok_or(Error::TooLarge)?;
            if bytes > max_bytes {
                return Err(Error::TooLarge);
            }
        }
    }
    Ok(())
}
