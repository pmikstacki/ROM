//! Publication only after the shared combined preconditions have passed.
use crate::native_work::Reader;
use rom::storage_support::metadata::JournalDelta;
use rom::{Error, Result};
use rusqlite::params;

pub(crate) fn replace(
    c: &rusqlite::Connection,
    metadata: rom::storage_support::metadata::StorageMetadata,
) -> Result<()> {
    let image = rom::storage_support::metadata::JournalImage::from_metadata(metadata)?;
    let raw = serde_json::to_string(&image.header().parts()).map_err(|_| Error::Storage)?;
    let entries = image
        .events()
        .iter()
        .map(|event| {
            let bytes = u64::try_from(serde_json::to_vec(event).map_err(|_| Error::Storage)?.len())
                .map_err(|_| Error::TooLarge)?;
            Ok((
                super::key(event.position),
                event.identity.clone(),
                super::key(bytes),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    c.execute("DELETE FROM journal_positions", [])
        .map_err(|_| Error::NotCommitted)?;
    for (position, identity, bytes) in entries {
        c.execute(
            "INSERT INTO journal_positions VALUES(?,?,?)",
            params![position.as_slice(), identity, bytes.as_slice()],
        )
        .map_err(|_| Error::NotCommitted)?;
    }
    c.execute("UPDATE rom_state SET data=? WHERE id=1", [raw])
        .map_err(|_| Error::NotCommitted)?;
    Ok(())
}

impl Reader<'_> {
    pub(crate) fn publish_journal(
        &self,
        delta: &JournalDelta,
        header: &str,
        mut checkpoint: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        if let Some(entry) = delta.appended() {
            let position = super::key(entry.event.position);
            let bytes =
                super::key(u64::try_from(entry.encoded_bytes).map_err(|_| Error::TooLarge)?);
            self.connection
                .execute(
                    "INSERT INTO journal_positions VALUES(?,?,?)",
                    params![position.as_slice(), entry.event.identity, bytes.as_slice()],
                )
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        for entry in delta.retired() {
            let position = super::key(entry.event.position);
            self.connection
                .execute(
                    "DELETE FROM journal_positions WHERE position=?",
                    [position.as_slice()],
                )
                .map_err(|_| Error::NotCommitted)?;
            checkpoint()?;
        }
        self.connection
            .execute("UPDATE rom_state SET data=? WHERE id=1", [header])
            .map_err(|_| Error::NotCommitted)?;
        #[cfg(feature = "test-support")]
        self.record_publication(crate::PublicationCategory::Metadata, header.len());
        checkpoint()
    }
}
