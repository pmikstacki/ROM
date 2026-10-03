//! Driver-independent bounded metadata persisted inside each native bundle transaction.
use super::*;
mod maintenance;
mod operator;
pub use operator::StorageWorkSnapshot;
mod retention;
mod work;
pub use retention::RetentionStateReport;
#[cfg(test)]
mod maintenance_tests;
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
    /// Apply only after native identity/revision arbitration. Returned identities left journal retention.
    pub fn bundle(&mut self, b: &Bundle) -> Result<Vec<String>> {
        self.check_retry_epoch(
            b.receipt.retry_epoch,
            false,
            b.completed_work.as_ref().map(|(claim, now)| (claim, *now)),
        )?;
        if b.reactions
            .iter()
            .any(|work| work.cause.retry_epoch != b.receipt.retry_epoch)
        {
            return Err(Error::Conflict);
        }
        let mut next = self.clone();
        if let Some((claim, now)) = &b.completed_work {
            next.work.apply(WorkUpdate::Finish {
                claim: claim.clone(),
                now: *now,
                outcome: WorkOutcome::Done,
            })?;
        }
        next.receipts = next.receipts.checked_add(1).ok_or(Error::TooLarge)?;
        next.effects = next
            .effects
            .checked_add(b.effects.len())
            .ok_or(Error::TooLarge)?;
        if next.receipts > next.limits.receipts || next.effects > next.limits.effects {
            return Err(Error::Overloaded);
        }
        if !b.reactions.is_empty() {
            if !b.changed {
                return Err(Error::NotCommitted);
            }
            next.work.enqueue(
                b.reaction_limits.as_ref().ok_or(Error::NotCommitted)?,
                b.reactions.clone(),
            )?;
        }
        let mut removed = vec![];
        if b.changed {
            next.head = next.head.checked_add(1).ok_or(Error::TooLarge)?;
            let event = JournalEvent {
                position: next.head,
                identity: b.receipt.identity.clone(),
                row: b.receipt.row.clone(),
            };
            if serde_json::to_vec(&event)
                .map_err(|_| Error::Storage)?
                .len()
                > next.limits.journal_bytes
            {
                return Err(Error::TooLarge);
            }
            next.events.push(event);
            while next.events.len() > next.limits.journal_rows
                || next.events.iter().try_fold(0usize, |sum, e| {
                    sum.checked_add(serde_json::to_vec(e).map_err(|_| Error::Storage)?.len())
                        .ok_or(Error::TooLarge)
                })? > next.limits.journal_bytes
            {
                let old = next.events.remove(0);
                next.floor = old.position;
                removed.push(old.identity);
            }
        }
        next.work.validate_retry_epochs(next.retry_epochs)?;
        *self = next;
        Ok(removed)
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
        if max_rows == 0 || max_bytes == 0 {
            return Err(Error::TooLarge);
        }
        let position = after.map_or(0, |c| c.position);
        if position < self.floor
            || position > self.head
            || after.is_some_and(|c| c.kind != kind || c.generation != self.generation)
        {
            return Err(Error::HistoryGap);
        }
        let mut cursor = JournalCursor {
            generation: self.generation.clone(),
            kind: kind.into(),
            position,
        };
        let mut events = vec![];
        let mut bytes = 0usize;
        for e in self.events.iter().filter(|e| e.position > position) {
            if e.row.key.kind == kind {
                let next = bytes
                    .checked_add(serde_json::to_vec(e).map_err(|_| Error::Storage)?.len())
                    .ok_or(Error::TooLarge)?;
                if next > max_bytes {
                    if events.is_empty() {
                        return Err(Error::TooLarge);
                    }
                    break;
                }
                if events.len() == max_rows {
                    break;
                }
                bytes = next;
                events.push(e.clone());
            }
            cursor.position = e.position;
        }
        Ok(JournalPage { events, cursor })
    }
}

#[cfg(test)]
mod operator_tests;
#[cfg(test)]
mod retention_tests;
