//! Bounded Row materialization shared by normal and observed reads.
use rom::{Error, Result, Row};
use rusqlite::{Connection, Statement, params};

pub(crate) struct Metrics {
    #[cfg(feature = "test-support")]
    pub decoded_rows: usize,
    #[cfg(feature = "test-support")]
    pub decoded_bytes: usize,
    #[cfg(feature = "test-support")]
    pub vm_steps: u64,
}
impl Metrics {
    pub(crate) fn read<const OBSERVED: bool>(
        rows: usize,
        bytes: usize,
        statement: &Statement<'_>,
    ) -> Result<Self> {
        #[cfg(not(feature = "test-support"))]
        let _ = (rows, bytes, statement);
        Ok(Self {
            #[cfg(feature = "test-support")]
            decoded_rows: rows,
            #[cfg(feature = "test-support")]
            decoded_bytes: bytes,
            #[cfg(feature = "test-support")]
            vm_steps: if OBSERVED {
                u64::try_from(statement.get_status(rusqlite::StatementStatus::VmStep))
                    .map_err(|_| Error::TooLarge)?
            } else {
                0
            },
        })
    }
}

pub(crate) fn snapshot<const OBSERVED: bool>(
    c: &Connection,
    kind: &str,
    max_rows: usize,
    max_bytes: usize,
) -> Result<(Vec<Row>, Metrics)> {
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
    drop(cursor);
    let metrics = Metrics::read::<OBSERVED>(result.len(), bytes, &statement)?;
    Ok((result, metrics))
}
