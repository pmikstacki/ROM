use crate::native_work::Reader;
use rom::storage_support::metadata::{JournalEntry, JournalRead, MetadataHeader};
use rom::storage_support::work::ReadFence;
use rom::{Error, JournalEvent, Result};
impl JournalRead for Reader<'_> {
    fn coherence(&self) -> ReadFence {
        self.fence.clone()
    }
    fn header(&self) -> Result<MetadataHeader> {
        let mut s = self
            .connection
            .prepare("SELECT data FROM rom_state WHERE id=1")
            .map_err(|_| Error::Storage)?;
        let mut rows = s.query([]).map_err(|_| Error::Storage)?;
        let r = rows
            .next()
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        let raw = r
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        self.charge(raw.len().checked_add(1).ok_or(Error::TooLarge)?, 1)?;
        MetadataHeader::from_parts(serde_json::from_str(raw).map_err(|_| Error::Storage)?)
    }
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>> {
        let mut s = self.connection.prepare("SELECT j.identity,j.canonical_bytes,e.data FROM journal_positions j LEFT JOIN events e ON e.identity=j.identity WHERE j.position=?").map_err(|_| Error::Storage)?;
        let k = super::key(position);
        let mut rows = s.query([k.as_slice()]).map_err(|_| Error::Storage)?;
        let Some(r) = rows.next().map_err(|_| Error::Storage)? else {
            return Ok(None);
        };
        let id = r
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let bytes = r
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_blob()
            .map_err(|_| Error::Storage)?;
        let raw = r
            .get_ref(2)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        self.charge(
            id.len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(raw.len()))
                .and_then(|n| n.checked_add(16))
                .ok_or(Error::TooLarge)?,
            2,
        )?;
        let encoded_bytes = usize::try_from(u64::from_be_bytes(
            bytes.try_into().map_err(|_| Error::Storage)?,
        ))
        .map_err(|_| Error::TooLarge)?;
        Ok(Some(JournalEntry {
            event: JournalEvent {
                position,
                identity: id.into(),
                row: serde_json::from_str(raw).map_err(|_| Error::Storage)?,
            },
            encoded_bytes,
        }))
    }
}

impl Reader<'_> {
    pub(crate) fn require_absent_event(&self, identity: &str) -> Result<()> {
        let mut statement = self
            .connection
            .prepare("SELECT data FROM events WHERE identity=?")
            .map_err(|_| Error::Storage)?;
        let mut rows = statement.query([identity]).map_err(|_| Error::Storage)?;
        if let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            let raw = row
                .get_ref(0)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?;
            self.charge(
                identity
                    .len()
                    .checked_add(raw.len())
                    .ok_or(Error::TooLarge)?,
                1,
            )?;
            return Err(Error::Storage);
        }
        Ok(())
    }
}
