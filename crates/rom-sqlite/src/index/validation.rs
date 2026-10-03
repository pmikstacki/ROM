//! Full derived-index integrity checks against authoritative logical data.
use super::{derivation, metadata};
use rom::{Error, Result};
use rom_backup::{BackupLimits, Collector};
use rusqlite::{Connection, OptionalExtension, params};

pub(crate) fn validate(
    c: &Connection,
    collector: &mut Collector,
    limits: BackupLimits,
) -> Result<()> {
    let store = metadata::profile(c)?;
    collector.physical(store.len().checked_add(24).ok_or(Error::TooLarge)?)?;
    super::layout::validate(c)?;
    // Charge the actual physical inventory before derived lookups or owned key copies.
    let mut actual = 0usize;
    let mut statement = c
        .prepare("SELECT kind,field,id,encoded FROM query_keys")
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let kind = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let field = row
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let id = row
            .get_ref(2)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let encoded = row
            .get_ref(3)
            .map_err(|_| Error::Storage)?
            .as_blob()
            .map_err(|_| Error::Storage)?;
        collector.physical(derivation::entry_bytes(kind, field, id, encoded)?)?;
        actual = actual.checked_add(1).ok_or(Error::TooLarge)?;
    }
    drop(rows);
    drop(statement);
    let mut statement=c.prepare("SELECT kind,row_count,row_bytes,canonical_row_bytes,live_count,generation FROM query_kinds").map_err(|_|Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    let mut actual_kinds = 0usize;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let kind = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        collector.physical(kind.len().checked_add(40).ok_or(Error::TooLarge)?)?;
        metadata::decode(row, 1)?;
        actual_kinds = actual_kinds.checked_add(1).ok_or(Error::TooLarge)?;
    }
    drop(rows);
    drop(statement);
    let mut expected = 0usize;
    let mut budget = derivation::Budget::new(limits);
    let mut statement = c
        .prepare("SELECT encoded FROM query_keys WHERE kind=? AND field=? AND id=?")
        .map_err(|_| Error::Storage)?;
    let counts = derivation::rows(c, &collector.snapshot, limits, |descriptor, row| {
        derivation::keys(descriptor, row, |field, encoded| {
            budget.charge(derivation::entry_bytes(
                &row.key.kind,
                field,
                &row.key.id,
                &encoded,
            )?)?;
            expected = expected.checked_add(1).ok_or(Error::TooLarge)?;
            let actual: Option<Vec<u8>> = statement
                .query_row(params![row.key.kind, field, row.key.id], |r| r.get(0))
                .optional()
                .map_err(|_| Error::Storage)?;
            if actual.as_ref() != Some(&encoded) {
                return Err(Error::Storage);
            }
            Ok(())
        })
    })?;
    if expected != actual || actual_kinds != counts.len() {
        return Err(Error::Storage);
    }
    for (kind, expected) in counts {
        let actual = metadata::load(c, &kind)?.ok_or(Error::Storage)?;
        if actual.rows != expected.rows
            || actual.bytes != expected.bytes
            || actual.canonical_bytes != expected.canonical_bytes
            || actual.live != expected.live
        {
            return Err(Error::Storage);
        }
    }
    Ok(())
}
