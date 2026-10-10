use super::read::Reader;
use super::write::Position;
use crate::format::POSITIONS;
use redb::{ReadableTable, WriteTransaction};
use rom::storage_support::metadata::{JournalImage, JournalRead};
use rom::{Error, Result, StorageState};
use rom_backup::BackupLimits;

/// Called only after complete physical admission of the native inventory.
pub(crate) fn collect<S, P, E, W, R, A>(
    reader: &Reader<S, P, E, W, R, A>,
    limits: BackupLimits,
) -> Result<(StorageState, Vec<usize>)>
where
    S: ReadableTable<&'static str, &'static str>,
    P: ReadableTable<u64, &'static str>,
    E: ReadableTable<&'static str, &'static str>,
    W: ReadableTable<&'static str, &'static str>,
    R: ReadableTable<&'static str, &'static str>,
    A: ReadableTable<&'static str, u8>,
{
    if reader.positions.len().map_err(|_| Error::Storage)?
        != reader.events.len().map_err(|_| Error::Storage)?
    {
        return Err(Error::Storage);
    }
    let header = JournalRead::header(reader)?;
    let mut events = Vec::new();
    let mut physical = Vec::new();
    for entry in reader.positions.iter().map_err(|_| Error::Storage)? {
        let (position, raw) = entry.map_err(|_| Error::Storage)?;
        physical.push(
            8usize
                .checked_add(raw.value().len())
                .ok_or(Error::TooLarge)?,
        );
        events.push(reader.entry(position.value())?.ok_or(Error::Storage)?.event);
    }
    let metadata = JournalImage::from_native(header, events)?.into_metadata();
    let (state, derived) = crate::native_state::read_with_metadata(
        &reader.state,
        &reader.records,
        &reader.roots,
        &reader.active,
        limits,
        Some(metadata),
    )?;
    physical.extend(derived);
    Ok((state, physical))
}

/// Maintenance writes derive scalar and index facts from one canonical image.
pub(crate) fn import(tx: &WriteTransaction, state: &StorageState) -> Result<()> {
    let image = JournalImage::from_metadata(
        rom::storage_support::metadata::StorageMetadata::from_state(state),
    )?;
    crate::native_state::write_with_header(
        tx,
        state,
        Some(serde_json::to_string(&image.header().parts()).map_err(|_| Error::Storage)?),
    )?;
    let mut positions = tx.open_table(POSITIONS).map_err(|_| Error::Storage)?;
    let keys = positions
        .iter()
        .map_err(|_| Error::Storage)?
        .map(|r| r.map(|(p, _)| p.value()).map_err(|_| Error::Storage))
        .collect::<Result<Vec<_>>>()?;
    for p in keys {
        positions.remove(p).map_err(|_| Error::NotCommitted)?;
    }
    for event in image.events() {
        let bytes = serde_json::to_vec(event).map_err(|_| Error::Storage)?.len();
        let index = serde_json::to_string(&Position {
            identity: event.identity.clone(),
            canonical_bytes: u64::try_from(bytes).map_err(|_| Error::TooLarge)?,
        })
        .map_err(|_| Error::Storage)?;
        positions
            .insert(event.position, index.as_str())
            .map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}
