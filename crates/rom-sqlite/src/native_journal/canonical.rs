//! Explicit full reconstruction; raw inventory admission precedes any decoding.
use crate::native_work::Reader;
use rom::storage_support::metadata::{JournalImage, JournalRead, StorageMetadata};
use rom::{Error, Result};
use rom_backup::BackupLimits;
use rusqlite::{Connection, types::ValueRef};

pub(crate) fn admit_inventory(c: &Connection, limits: BackupLimits) -> Result<()> {
    admit_inventory_for_format(c, limits, 11)
}
pub(crate) fn admit_inventory_for_format(
    c: &Connection,
    limits: BackupLimits,
    format: u32,
) -> Result<()> {
    crate::snapshot::validate_inventory(c, format)?;
    if format == 11 {
        super::schema::validate(c)?;
    }
    let mut names=c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT GLOB 'sqlite_*' ORDER BY name").map_err(|_|Error::Storage)?;
    let mut tables = names.query([]).map_err(|_| Error::Storage)?;
    let mut bytes = 0usize;
    let mut records = 0usize;
    while let Some(table) = tables.next().map_err(|_| Error::Storage)? {
        let name = table
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        // Exact inventory admission above establishes trusted SQL identifier names.
        let mut statement = c
            .prepare(&format!("SELECT * FROM {name}"))
            .map_err(|_| Error::Storage)?;
        let columns = statement.column_count();
        let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
        while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            records = records.checked_add(1).ok_or(Error::TooLarge)?;
            for column in 0..columns {
                let length = match row.get_ref(column).map_err(|_| Error::Storage)? {
                    ValueRef::Null => 0,
                    ValueRef::Integer(_) | ValueRef::Real(_) => 8,
                    ValueRef::Text(x) | ValueRef::Blob(x) => x.len(),
                };
                bytes = bytes.checked_add(length).ok_or(Error::TooLarge)?;
                if bytes > limits.max_bytes || records > limits.max_records {
                    return Err(Error::TooLarge);
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn reconstruct_metadata(
    c: &Connection,
    limits: BackupLimits,
) -> Result<(StorageMetadata, Vec<usize>)> {
    let reader = Reader::bounded(c, limits);
    let header = JournalRead::header(&reader)?;
    let parts = header.parts();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM journal_positions", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    let payloads: i64 = c
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if usize::try_from(count).map_err(|_| Error::Storage)? != parts.journal_records
        || count != payloads
    {
        return Err(Error::Storage);
    }
    let mut events = Vec::new();
    let mut derived = Vec::new();
    let mut position = parts.floor;
    while position < parts.head {
        position = position.checked_add(1).ok_or(Error::TooLarge)?;
        let entry = reader.entry(position)?.ok_or(Error::Storage)?;
        let actual = serde_json::to_vec(&entry.event)
            .map_err(|_| Error::Storage)?
            .len();
        if actual != entry.encoded_bytes {
            return Err(Error::Storage);
        }
        derived.push(
            entry
                .event
                .identity
                .len()
                .checked_add(16)
                .ok_or(Error::TooLarge)?,
        );
        events.push(entry.event);
    }
    Ok((
        JournalImage::from_native(header, events)?.into_metadata(),
        derived,
    ))
}
