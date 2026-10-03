//! Explicit offline retirement of journal prefixes and completed causal roots.
use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetentionStateReport {
    pub journal_identities: Vec<String>,
    pub work_records: usize,
    pub work_roots: usize,
}

impl StorageState {
    pub fn retry_epochs(&self) -> RetryEpochs {
        self.retry_epochs
    }

    pub fn journal_floor(&self) -> u64 {
        self.floor
    }

    /// Check inside the same transaction as receipt arbitration and work completion.
    /// A replay does not require its original claim to remain active.
    pub fn check_retry_epoch(
        &self,
        epoch: u64,
        replay_exists: bool,
        completed_work: Option<(&ClaimKey, u64)>,
    ) -> Result<()> {
        self.retry_epochs.check(epoch, true, false)?;
        if replay_exists {
            return Ok(());
        }
        let causal = if let Some((claim, now)) = completed_work {
            if self.work.claim_retry_epoch(claim, now)? != epoch {
                return Err(Error::Conflict);
            }
            true
        } else {
            false
        };
        self.retry_epochs.check(epoch, false, causal)
    }

    /// Maintenance only. Counts describe the retained native receipt/effect tables.
    /// The caller must preserve their dependency closure before publishing a snapshot.
    /// Errors leave this state unchanged; repeated cutoffs are valid.
    pub fn apply_retention(
        &mut self,
        epochs: RetryEpochs,
        journal_through: u64,
        receipt_count: usize,
        effect_count: usize,
    ) -> Result<RetentionStateReport> {
        epochs.validate()?;
        if epochs.current < self.retry_epochs.current
            || epochs.admission_floor < self.retry_epochs.admission_floor
            || epochs.replay_floor < self.retry_epochs.replay_floor
            || journal_through < self.floor
            || journal_through > self.head
            || receipt_count > self.receipts
            || effect_count > self.effects
        {
            return Err(Error::Conflict);
        }
        let events: Vec<_> = self
            .events
            .iter()
            .map(|event| (event.identity.clone(), event.row.clone()))
            .collect();
        self.validate_archive(self.receipts, self.effects, &events)?;
        let mut next = self.clone();
        let (work_records, work_roots) = next.work.retire_completed_roots(epochs.replay_floor)?;
        let count = next
            .events
            .partition_point(|event| event.position <= journal_through);
        let journal_identities = next
            .events
            .drain(..count)
            .map(|event| event.identity)
            .collect();
        next.floor = journal_through;
        next.retry_epochs = epochs;
        next.receipts = receipt_count;
        next.effects = effect_count;
        next.work.validate_retry_epochs(epochs)?;
        *self = next;
        Ok(RetentionStateReport {
            journal_identities,
            work_records,
            work_roots,
        })
    }
}
