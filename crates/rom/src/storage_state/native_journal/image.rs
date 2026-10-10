//! Bounded canonical journal reconstruction for explicit import and maintenance.
use super::*;

/// Canonical journal structure and scalar accounting, separate from native row agreement.
/// Resource row contents retain exactly the canonical archive validation contract.
pub struct JournalImage {
    pub(super) coherence: crate::storage_support::work::ReadFence,
    pub(super) header: MetadataHeader,
    pub(super) metadata: StorageMetadata,
}
impl JournalImage {
    /// Validate canonical ordering, identity uniqueness, limits and retry boundaries.
    /// The identity-row projection comes from these events; it is not an independent
    /// native table and therefore cannot prove native event table agreement.
    pub fn from_metadata(metadata: StorageMetadata) -> Result<Self> {
        let rows = metadata
            .events
            .iter()
            .map(|e| (e.identity.clone(), e.row.clone()))
            .collect::<Vec<_>>();
        metadata
            .clone()
            .reassemble(WorkLedger::default(), OperatorLedger::default())
            .validate_archive(metadata.receipts, metadata.effects, &rows)?;
        let bytes = metadata.events.iter().try_fold(0usize, |sum, event| {
            sum.checked_add(serde_json::to_vec(event).map_err(|_| Error::Storage)?.len())
                .ok_or(Error::TooLarge)
        })?;
        let header = MetadataHeader::from_parts(MetadataHeaderParts {
            retry_epochs: metadata.retry_epochs,
            limits: metadata.limits.clone(),
            generation: metadata.generation.clone(),
            head: metadata.head,
            floor: metadata.floor,
            receipts: metadata.receipts,
            effects: metadata.effects,
            journal_records: metadata.events.len(),
            journal_bytes: bytes,
        })?;
        Ok(Self {
            coherence: crate::storage_support::work::ReadFence::new(),
            header,
            metadata,
        })
    }
    /// Reject scalar facts that differ from the supplied canonical event projection.
    pub fn from_native(header: MetadataHeader, events: Vec<JournalEvent>) -> Result<Self> {
        let p = header.parts();
        let metadata = StorageMetadata {
            retry_epochs: p.retry_epochs,
            limits: p.limits,
            generation: p.generation,
            head: p.head,
            floor: p.floor,
            receipts: p.receipts,
            effects: p.effects,
            events,
        };
        let imported = Self::from_metadata(metadata)?;
        if imported.header != header {
            return Err(Error::Storage);
        }
        Ok(imported)
    }
    pub fn header(&self) -> &MetadataHeader {
        &self.header
    }
    pub fn events(&self) -> &[JournalEvent] {
        &self.metadata.events
    }
    pub fn into_metadata(self) -> StorageMetadata {
        self.metadata
    }
}
