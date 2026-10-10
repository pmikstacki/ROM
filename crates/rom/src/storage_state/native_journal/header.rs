//! Scalar admission rules for a bounded canonical journal projection.
use super::*;

/// Persisted scalar DTO; deserialize then admit with `MetadataHeader::from_parts`.
/// Construction does not establish agreement with native rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataHeaderParts {
    pub retry_epochs: RetryEpochs,
    pub limits: StorageLimits,
    pub generation: String,
    pub head: u64,
    pub floor: u64,
    pub receipts: usize,
    pub effects: usize,
    pub journal_records: usize,
    pub journal_bytes: usize,
}

/// Validated scalar relationships; journal payload agreement requires `JournalImage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataHeader(MetadataHeaderParts);
impl MetadataHeader {
    pub fn from_parts(parts: MetadataHeaderParts) -> Result<Self> {
        // Reuse canonical generation, capacity and epoch validation. An empty
        // retained projection can validate scalars without fabricating events.
        let projection = StorageMetadata {
            retry_epochs: parts.retry_epochs,
            limits: parts.limits.clone(),
            generation: parts.generation.clone(),
            head: parts.head,
            floor: parts.head,
            receipts: parts.receipts,
            effects: parts.effects,
            events: vec![],
        };
        projection
            .reassemble(WorkLedger::default(), OperatorLedger::default())
            .validate_archive(parts.receipts, parts.effects, &[])?;
        let retained = parts.head.checked_sub(parts.floor).ok_or(Error::Storage)?;
        let records = u64::try_from(parts.journal_records).map_err(|_| Error::TooLarge)?;
        if retained != records
            || parts.journal_records > parts.limits.journal_rows
            || parts.journal_bytes > parts.limits.journal_bytes
            || (parts.journal_records == 0) != (parts.journal_bytes == 0)
        {
            return Err(Error::Storage);
        }
        Ok(Self(parts))
    }
    pub fn parts(&self) -> MetadataHeaderParts {
        self.0.clone()
    }
    pub fn retry_epochs(&self) -> RetryEpochs {
        self.0.retry_epochs
    }
}

impl MetadataHeader {
    pub fn check_retry_epoch(
        &self,
        epoch: u64,
        replay_exists: bool,
        completed_work: Option<(&ClaimKey, u64)>,
        work: &impl crate::storage_support::work::WorkRead,
    ) -> Result<()> {
        self.retry_epochs().check(epoch, true, false)?;
        if replay_exists {
            return Ok(());
        }
        let mut edit = prepare::new_edit(self.retry_epochs(), work)?;
        prepare::check_new_epoch(self.retry_epochs(), epoch, completed_work, &mut edit)
    }
}
