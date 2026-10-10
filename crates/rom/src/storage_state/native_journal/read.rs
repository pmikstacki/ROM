use super::*;
use crate::storage_support::work::ReadFence;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub event: JournalEvent,
    pub encoded_bytes: usize,
}
/// All values come from one transaction. Charge cumulative physical read bytes
/// before allocation; logical page bounds do not replace that physical budget.
pub trait JournalRead {
    fn coherence(&self) -> ReadFence;
    fn header(&self) -> Result<MetadataHeader>;
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>>;
}

/// Reuse canonical admission for a touched event without reconstructing history.
pub(super) fn validate_entry(
    header: &MetadataHeader,
    position: u64,
    entry: &JournalEntry,
) -> Result<()> {
    if entry.event.position != position || position == 0 {
        return Err(Error::Storage);
    }
    let actual = serde_json::to_vec(&entry.event)
        .map_err(|_| Error::Storage)?
        .len();
    if actual != entry.encoded_bytes {
        return Err(Error::Storage);
    }
    let mut parts = header.parts();
    parts.head = position;
    parts.floor = position - 1;
    parts.journal_records = 1;
    parts.journal_bytes = actual;
    JournalImage::from_native(
        MetadataHeader::from_parts(parts)?,
        vec![entry.event.clone()],
    )?;
    Ok(())
}
impl JournalRead for JournalImage {
    fn coherence(&self) -> ReadFence {
        self.coherence.clone()
    }
    fn header(&self) -> Result<MetadataHeader> {
        Ok(self.header.clone())
    }
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>> {
        let parts = self.header.parts();
        if position <= parts.floor || position > parts.head {
            return Ok(None);
        }
        let index = usize::try_from(position - parts.floor - 1).map_err(|_| Error::TooLarge)?;
        self.metadata
            .events
            .get(index)
            .map(|e| {
                Ok(JournalEntry {
                    event: e.clone(),
                    encoded_bytes: serde_json::to_vec(e).map_err(|_| Error::Storage)?.len(),
                })
            })
            .transpose()
    }
}
impl JournalImage {
    pub fn apply(&mut self, delta: JournalDelta) -> Result<()> {
        delta.validate(self)?;
        let mut events = self.events().to_vec();
        events.drain(..delta.retired.len());
        if let Some(e) = delta.appended {
            events.push(e.event);
        }
        let mut next = Self::from_native(delta.after, events)?;
        next.coherence = ReadFence::new();
        *self = next;
        Ok(())
    }
}
