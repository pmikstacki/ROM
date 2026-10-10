//! Driver-independent bounded metadata persisted inside each native bundle transaction.
use super::*;
mod bundle;
mod maintenance;
pub(crate) mod metadata;
pub(crate) mod native_journal;
mod operator;
pub use operator::StorageWorkSnapshot;
mod retention;
mod work;
pub use retention::RetentionStateReport;
#[cfg(test)]
mod maintenance_tests;
#[cfg(test)]
mod metadata_tests;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLimits {
    pub receipts: usize,
    pub effects: usize,
    pub journal_rows: usize,
    pub journal_bytes: usize,
}
impl Default for StorageLimits {
    fn default() -> Self {
        Self {
            receipts: 100_000,
            effects: 100_000,
            journal_rows: 1024,
            journal_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageState {
    pub work: WorkLedger,
    pub operator: OperatorLedger,
    #[serde(default)]
    retry_epochs: RetryEpochs,
    limits: StorageLimits,
    generation: String,
    head: u64,
    floor: u64,
    receipts: usize,
    effects: usize,
    events: Vec<JournalEvent>,
}
impl StorageState {
    pub fn new(limits: StorageLimits) -> Result<Self> {
        if [
            limits.receipts,
            limits.effects,
            limits.journal_rows,
            limits.journal_bytes,
        ]
        .contains(&0)
        {
            return Err(Error::TooLarge);
        }
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let generation = format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Storage)?
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        Ok(Self {
            work: WorkLedger::default(),
            operator: OperatorLedger::default(),
            retry_epochs: RetryEpochs::default(),
            limits,
            generation,
            head: 0,
            floor: 0,
            receipts: 0,
            effects: 0,
            events: vec![],
        })
    }
    /// Maintenance only: validate full logical table consistency before archive/restore.
    pub fn validate_archive(
        &self,
        receipts: usize,
        effects: usize,
        events: &[(String, Row)],
    ) -> Result<()> {
        Self::new(self.limits.clone())?;
        crate::operator::WorkVersion {
            generation: self.generation.clone(),
            revision: 0,
        }
        .validate()?;
        if self.generation.is_empty()
            || self.floor > self.head
            || self.receipts != receipts
            || self.effects != effects
            || receipts > self.limits.receipts
            || effects > self.limits.effects
            || self.events.len() > self.limits.journal_rows
            || self.head - self.floor != self.events.len() as u64
            || events.len() != self.events.len()
        {
            return Err(Error::Storage);
        }
        let native: BTreeMap<_, _> = events.iter().map(|(id, row)| (id, row)).collect();
        let mut bytes = 0usize;
        let mut identities = BTreeSet::new();
        for (i, event) in self.events.iter().enumerate() {
            if !identities.insert(&event.identity)
                || event.position != self.floor + i as u64 + 1
                || native.get(&event.identity) != Some(&&event.row)
            {
                return Err(Error::Storage);
            }
            bytes = bytes
                .checked_add(serde_json::to_vec(event).map_err(|_| Error::Storage)?.len())
                .ok_or(Error::TooLarge)?;
        }
        if bytes > self.limits.journal_bytes {
            return Err(Error::Storage);
        }
        self.operator.validate_archive()?;
        if self
            .operator
            .receipts
            .values()
            .any(|receipt| receipt.request.retry_epoch > self.retry_epochs.current)
        {
            return Err(Error::Storage);
        }
        self.work.validate_archive()?;
        self.work.validate_retry_epochs(self.retry_epochs)
    }
    /// Persisted maintenance limits; restore does not silently replace them with defaults.
    pub fn storage_limits(&self) -> StorageLimits {
        self.limits.clone()
    }
    /// Fence pre-restore cursors and claims, retaining identities, attempts and outcomes.
    pub fn prepare_restore(&mut self) -> Result<()> {
        let mut restored = self.clone();
        restored.work.prepare_restore()?;
        restored.generation = Self::new(self.limits.clone())?.generation;
        *self = restored;
        Ok(())
    }
    pub fn check_limits(&self, limits: &StorageLimits) -> Result<()> {
        if &self.limits != limits {
            Err(Error::Unsupported("persisted storage limits differ".into()))
        } else {
            self.operator.validate_archive()?;
            self.work.check_compatible_capacity()
        }
    }
    pub fn journal_head(&self, kind: &str) -> JournalCursor {
        JournalCursor {
            generation: self.generation.clone(),
            kind: kind.into(),
            position: self.head,
        }
    }
    pub fn journal(
        &self,
        kind: &str,
        after: Option<&JournalCursor>,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<JournalPage> {
        metadata::StorageMetadata::from_state(self).journal(kind, after, max_rows, max_bytes)
    }
}

#[cfg(test)]
mod operator_tests;
#[cfg(test)]
mod retention_tests;
