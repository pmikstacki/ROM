//! Complete borrowed native inventory admission before any owned JSON decode.
use crate::{
    format::*,
    references::{INCOMING, OUTGOING, SCHEMAS},
};
use redb::{ReadTransaction, ReadableTable};
use rom::{Error, Result};
use rom_backup::BackupLimits;

pub(super) fn check(tx: &ReadTransaction, version: u64, limits: BackupLimits) -> Result<()> {
    let mut bytes = 0usize;
    let mut records = 0usize;
    let mut charge = |size: usize| -> Result<()> {
        bytes = bytes.checked_add(size).ok_or(Error::TooLarge)?;
        records = records.checked_add(1).ok_or(Error::TooLarge)?;
        if bytes > limits.max_bytes || records > limits.max_records {
            return Err(Error::TooLarge);
        }
        Ok(())
    };
    strings(
        &tx.open_table(STATE).map_err(|_| Error::Storage)?,
        &mut charge,
    )?;
    for table in [RECEIPTS, EVENTS] {
        strings(
            &tx.open_table(table).map_err(|_| Error::Storage)?,
            &mut charge,
        )?;
    }
    for entry in tx
        .open_table(ROWS)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        let key = key.value();
        charge(sum(&[key.0.len(), key.1.len(), raw.value().len()])?)?;
    }
    for entry in tx
        .open_table(EFFECTS)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        charge(sum(&[key.value().0.len(), 8, raw.value().len()])?)?;
    }
    for entry in tx
        .open_table(META)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, _) = entry.map_err(|_| Error::Storage)?;
        charge(sum(&[key.value().len(), 8])?)?;
    }
    if version != 3 {
        strings(
            &tx.open_table(SCHEMAS).map_err(|_| Error::Storage)?,
            &mut charge,
        )?;
        for table in [OUTGOING, INCOMING] {
            for entry in tx
                .open_table(table)
                .map_err(|_| Error::Storage)?
                .iter()
                .map_err(|_| Error::Storage)?
            {
                let (key, _) = entry.map_err(|_| Error::Storage)?;
                let k = key.value();
                charge(sum(&[k.0.len(), k.1.len(), k.2.len(), k.3.len(), 1])?)?;
            }
        }
    }
    if version >= 10 {
        for table in [WORK, ROOTS] {
            strings(
                &tx.open_table(table).map_err(|_| Error::Storage)?,
                &mut charge,
            )?;
        }
        for entry in tx
            .open_table(ACTIVE)
            .map_err(|_| Error::Storage)?
            .iter()
            .map_err(|_| Error::Storage)?
        {
            let (key, _) = entry.map_err(|_| Error::Storage)?;
            charge(sum(&[key.value().len(), 1])?)?;
        }
    }
    if version == JOURNAL_FORMAT {
        for entry in tx
            .open_table(POSITIONS)
            .map_err(|_| Error::Storage)?
            .iter()
            .map_err(|_| Error::Storage)?
        {
            let (_, raw) = entry.map_err(|_| Error::Storage)?;
            charge(sum(&[8, raw.value().len()])?)?;
        }
    }
    Ok(())
}
fn sum(parts: &[usize]) -> Result<usize> {
    parts
        .iter()
        .try_fold(0usize, |n, p| n.checked_add(*p).ok_or(Error::TooLarge))
}
fn strings(
    table: &impl ReadableTable<&'static str, &'static str>,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    for entry in table.iter().map_err(|_| Error::Storage)? {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        charge(sum(&[key.value().len(), raw.value().len()])?)?;
    }
    Ok(())
}
