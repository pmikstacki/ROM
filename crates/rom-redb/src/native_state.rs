//! Explicit canonical import/export; ordinary writes retain only bounded metadata.
use crate::format::{ACTIVE, ROOTS, STATE, WORK};
use redb::{ReadableTable, Table, WriteTransaction};
use rom::storage_support::{
    metadata::StorageMetadata,
    work::{WorkHeader, WorkHeaderParts, WorkImage, WorkRead},
};
use rom::{Error, Result, StorageState};
use rom_backup::BackupLimits;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn write(tx: &WriteTransaction, state: &StorageState) -> Result<()> {
    write_with_header(tx, state, None)
}
pub(super) fn write_with_header(
    tx: &WriteTransaction,
    state: &StorageState,
    native_header: Option<String>,
) -> Result<()> {
    let (metadata, ledger, operator) = StorageMetadata::split(state.clone());
    let image = WorkImage::from_ledger(ledger, metadata.retry_epochs())?;
    let mut table = tx.open_table(STATE).map_err(|_| Error::Storage)?;
    clear(&mut table)?;
    for (key, raw) in [
        (
            "metadata",
            match native_header {
                Some(raw) => Ok(raw),
                None => serde_json::to_string(&metadata),
            },
        ),
        (
            "work_header",
            serde_json::to_string(&image.header()?.parts()),
        ),
        ("operator", serde_json::to_string(&operator)),
    ] {
        table
            .insert(key, raw.map_err(|_| Error::Storage)?.as_str())
            .map_err(|_| Error::NotCommitted)?;
    }
    let mut records = tx.open_table(WORK).map_err(|_| Error::Storage)?;
    clear(&mut records)?;
    for (id, record) in image.records() {
        records
            .insert(
                id,
                serde_json::to_string(record)
                    .map_err(|_| Error::Storage)?
                    .as_str(),
            )
            .map_err(|_| Error::NotCommitted)?;
    }
    let mut roots = tx.open_table(ROOTS).map_err(|_| Error::Storage)?;
    clear(&mut roots)?;
    for (id, root) in image.roots() {
        roots
            .insert(
                id,
                serde_json::to_string(root)
                    .map_err(|_| Error::Storage)?
                    .as_str(),
            )
            .map_err(|_| Error::NotCommitted)?;
    }
    let mut active = tx.open_table(ACTIVE).map_err(|_| Error::Storage)?;
    let ids = active
        .iter()
        .map_err(|_| Error::Storage)?
        .map(|entry| {
            entry
                .map(|(id, _)| id.value().to_owned())
                .map_err(|_| Error::Storage)
        })
        .collect::<Result<Vec<_>>>()?;
    for id in ids {
        active
            .remove(id.as_str())
            .map_err(|_| Error::NotCommitted)?;
    }
    for id in image.active_ids() {
        active.insert(id, 1).map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}
fn clear(table: &mut Table<'_, &'static str, &'static str>) -> Result<()> {
    let ids = table
        .iter()
        .map_err(|_| Error::Storage)?
        .map(|entry| {
            entry
                .map(|(id, _)| id.value().to_owned())
                .map_err(|_| Error::Storage)
        })
        .collect::<Result<Vec<_>>>()?;
    for id in ids {
        table.remove(id.as_str()).map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}
pub(super) fn metadata(
    table: &impl ReadableTable<&'static str, &'static str>,
) -> Result<StorageMetadata> {
    decode(table, "metadata")
}
pub(super) fn write_metadata(
    table: &mut Table<'_, &'static str, &'static str>,
    metadata: &StorageMetadata,
) -> Result<()> {
    table
        .insert(
            "metadata",
            serde_json::to_string(metadata)
                .map_err(|_| Error::Storage)?
                .as_str(),
        )
        .map_err(|_| Error::NotCommitted)?;
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned>(
    table: &impl ReadableTable<&'static str, &'static str>,
    key: &str,
) -> Result<T> {
    let raw = table
        .get(key)
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    serde_json::from_str(raw.value()).map_err(|_| Error::Storage)
}
/// Charge physical entries before allocating owned keys or decoding values.
pub(super) fn read(
    state: &impl ReadableTable<&'static str, &'static str>,
    records: &impl ReadableTable<&'static str, &'static str>,
    roots: &impl ReadableTable<&'static str, &'static str>,
    active: &impl ReadableTable<&'static str, u8>,
    limits: BackupLimits,
) -> Result<(StorageState, Vec<usize>)> {
    read_with_metadata(state, records, roots, active, limits, None)
}

pub(super) fn read_with_metadata(
    state: &impl ReadableTable<&'static str, &'static str>,
    records: &impl ReadableTable<&'static str, &'static str>,
    roots: &impl ReadableTable<&'static str, &'static str>,
    active: &impl ReadableTable<&'static str, u8>,
    limits: BackupLimits,
    admitted_metadata: Option<StorageMetadata>,
) -> Result<(StorageState, Vec<usize>)> {
    let mut bytes = 0usize;
    let mut count = 0usize;
    let mut charge = |size: usize| -> Result<()> {
        bytes = bytes.checked_add(size).ok_or(Error::TooLarge)?;
        count = count.checked_add(1).ok_or(Error::TooLarge)?;
        if bytes > limits.max_bytes || count > limits.max_records {
            return Err(Error::TooLarge);
        }
        Ok(())
    };
    charge_table(state, &mut charge)?;
    charge_table(records, &mut charge)?;
    charge_table(roots, &mut charge)?;
    for entry in active.iter().map_err(|_| Error::Storage)? {
        let (key, _) = entry.map_err(|_| Error::Storage)?;
        charge(key.value().len().checked_add(1).ok_or(Error::TooLarge)?)?;
    }
    if state.len().map_err(|_| Error::Storage)? != 3 {
        return Err(Error::Storage);
    }
    let metadata = match admitted_metadata {
        Some(metadata) => metadata,
        None => metadata(state)?,
    };
    let parts: WorkHeaderParts = decode(state, "work_header")?;
    let header = WorkHeader::from_parts(parts)?;
    if header.retry_epochs != metadata.retry_epochs() {
        return Err(Error::Storage);
    }
    let operator = decode(state, "operator")?;
    let mut work_map = BTreeMap::new();
    for entry in records.iter().map_err(|_| Error::Storage)? {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        work_map.insert(
            key.value().to_owned(),
            serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?,
        );
    }
    let mut root_map = BTreeMap::new();
    let mut derived = Vec::new();
    for entry in roots.iter().map_err(|_| Error::Storage)? {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        derived.push(
            key.value()
                .len()
                .checked_add(raw.value().len())
                .ok_or(Error::TooLarge)?,
        );
        root_map.insert(
            key.value().to_owned(),
            serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?,
        );
    }
    let mut ids = BTreeSet::new();
    for entry in active.iter().map_err(|_| Error::Storage)? {
        let (id, marker) = entry.map_err(|_| Error::Storage)?;
        if marker.value() != 1 {
            return Err(Error::Storage);
        }
        derived.push(id.value().len().checked_add(1).ok_or(Error::TooLarge)?);
        ids.insert(id.value().to_owned());
    }
    let raw = state
        .get("work_header")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    derived.push(
        "work_header"
            .len()
            .checked_add(raw.value().len())
            .ok_or(Error::TooLarge)?,
    );
    let raw = state
        .get("operator")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    derived.push(
        "operator"
            .len()
            .checked_add(raw.value().len())
            .ok_or(Error::TooLarge)?,
    );
    let image = WorkImage::from_native(header, work_map, root_map, ids)?;
    Ok((metadata.reassemble(image.canonical(), operator), derived))
}

fn charge_table(
    table: &impl ReadableTable<&'static str, &'static str>,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    for entry in table.iter().map_err(|_| Error::Storage)? {
        let (key, raw) = entry.map_err(|_| Error::Storage)?;
        charge(
            key.value()
                .len()
                .checked_add(raw.value().len())
                .ok_or(Error::TooLarge)?,
        )?;
    }
    Ok(())
}

/// Bound caller-visible Work payloads before deserialization. Native duplicate
/// keys and derived projections use the independent configured native budget.
pub(super) fn operator_budget(
    state: &impl ReadableTable<&'static str, &'static str>,
    records: &impl ReadableTable<&'static str, &'static str>,
    max_records: usize,
    max_bytes: usize,
) -> Result<()> {
    if max_records == 0
        || max_bytes == 0
        || records.len().map_err(|_| Error::Storage)?
            > u64::try_from(max_records).map_err(|_| Error::TooLarge)?
    {
        return Err(Error::TooLarge);
    }
    let mut bytes = 0usize;
    for entry in records.iter().map_err(|_| Error::Storage)? {
        let (_, raw) = entry.map_err(|_| Error::Storage)?;
        bytes = bytes
            .checked_add(raw.value().len())
            .ok_or(Error::TooLarge)?;
        if bytes > max_bytes {
            return Err(Error::TooLarge);
        }
    }
    let operator = state
        .get("operator")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    bytes = bytes
        .checked_add(operator.value().len())
        .ok_or(Error::TooLarge)?;
    if bytes > max_bytes {
        return Err(Error::TooLarge);
    }
    Ok(())
}
