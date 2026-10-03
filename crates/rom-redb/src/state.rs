//! Shared native codec for the durable metadata record.
use redb::{ReadableTable, Table};
use rom::{Error, Result, StorageState};

pub(super) fn read(table: &impl ReadableTable<&'static str, &'static str>) -> Result<StorageState> {
    let value = table
        .get("state")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    serde_json::from_str(value.value()).map_err(|_| Error::Storage)
}

pub(super) fn write(
    table: &mut Table<'_, &'static str, &'static str>,
    state: &StorageState,
) -> Result<()> {
    table
        .insert(
            "state",
            serde_json::to_string(state)
                .map_err(|_| Error::Storage)?
                .as_str(),
        )
        .map_err(|_| Error::NotCommitted)?;
    Ok(())
}
