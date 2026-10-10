//! Shared transaction-local work transitions; readers are trusted native support.
//! Unrelated records rely on validated import and maintained native invariants.
use super::accounting::{LedgerBytes, entry_for_root, entry_for_work};
use super::*;
mod admission;
mod engine;
mod update;
use engine::Engine;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkHeader {
    pub limits: Option<ReactionLimits>,
    pub retry_epochs: RetryEpochs,
    pub accounting: LedgerBytes,
}
/// Native persisted header DTO. Decode then reconstruct; it is not authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkHeaderParts {
    pub limits: Option<ReactionLimits>,
    pub retry_epochs: RetryEpochs,
    pub accounting: super::accounting::WorkAccounting,
}
impl WorkHeader {
    pub fn from_parts(parts: WorkHeaderParts) -> Result<Self> {
        let accounting = LedgerBytes::from_parts(parts.limits.as_ref(), parts.accounting)?;
        let header = Self {
            limits: parts.limits,
            retry_epochs: parts.retry_epochs,
            accounting,
        };
        header.validate()?;
        Ok(header)
    }
    pub fn parts(&self) -> WorkHeaderParts {
        WorkHeaderParts {
            limits: self.limits.clone(),
            retry_epochs: self.retry_epochs,
            accounting: self.accounting.parts(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.retry_epochs.validate()?;
        self.accounting.validate_policy(self.limits.as_ref())?;
        if let Some(limits) = &self.limits {
            limits.validate()?;
        } else if self.accounting.parts() != super::accounting::WorkAccounting::default() {
            return Err(Error::Storage);
        }
        self.accounting.totals()?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootAccount {
    pub used: u32,
    pub epoch: u64,
    pub members: usize,
    pub incomplete: usize,
}
impl RootAccount {
    pub fn validate(&self) -> Result<()> {
        if self.members == 0 || self.incomplete > self.members {
            return Err(Error::Storage);
        }
        Ok(())
    }
}
/// Opaque lifetime identity for one originating coherent reader context.
/// This does not replace the native writer transaction or prove range predicates.
#[derive(Clone)]
pub struct ReadFence(std::sync::Arc<()>);
impl Default for ReadFence {
    fn default() -> Self {
        Self::new()
    }
}
impl ReadFence {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(()))
    }
    pub fn same_context(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
impl std::fmt::Debug for ReadFence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReadFence")
    }
}
/// All methods read one coherent transaction, never serialized caller authority.
/// The fence remains stable for that context and invalidates on local mutation.
pub trait WorkRead {
    fn coherence(&self) -> ReadFence;
    fn header(&self) -> Result<WorkHeader>;
    fn record(&self, id: &str) -> Result<Option<WorkRecord>>;
    fn root(&self, id: &str) -> Result<Option<RootAccount>>;
    /// Lowest relevant ID strictly after after_id; preserve atomic prefix effects.
    fn next_candidate(&self, now: u64, after_id: Option<&str>) -> Result<Option<WorkRecord>>;
}
struct RecordChange {
    before: Option<WorkRecord>,
    after: WorkRecord,
}
struct RootChange {
    before: Option<RootAccount>,
    after: RootAccount,
}
/// Opaque prepared changes. Applying requires the original coherent transaction.
pub struct WorkDelta {
    coherence: ReadFence,
    record_preconditions: BTreeMap<String, Option<WorkRecord>>,
    root_preconditions: BTreeMap<String, Option<RootAccount>>,
    before: WorkHeader,
    after: WorkHeader,
    records: BTreeMap<String, RecordChange>,
    roots: BTreeMap<String, RootChange>,
    result: WorkResult,
}
impl WorkDelta {
    /// Check the original context and every exact read fact before any publication.
    pub fn validate(&self, reader: &impl WorkRead) -> Result<()> {
        if !reader.coherence().same_context(&self.coherence) || reader.header()? != self.before {
            return Err(Error::Conflict);
        }
        for (id, before) in self.record_preconditions() {
            if reader.record(id)?.as_ref() != before {
                return Err(Error::Conflict);
            }
        }
        for (id, before) in self.root_preconditions() {
            if reader.root(id)?.as_ref() != before {
                return Err(Error::Conflict);
            }
        }
        if !reader.coherence().same_context(&self.coherence) {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub fn coherence(&self) -> &ReadFence {
        &self.coherence
    }
    pub fn record_preconditions(&self) -> impl Iterator<Item = (&str, Option<&WorkRecord>)> {
        self.record_preconditions
            .iter()
            .map(|(id, prior)| (id.as_str(), prior.as_ref()))
    }
    pub fn root_preconditions(&self) -> impl Iterator<Item = (&str, Option<&RootAccount>)> {
        self.root_preconditions
            .iter()
            .map(|(id, prior)| (id.as_str(), prior.as_ref()))
    }
    pub fn before_header(&self) -> &WorkHeader {
        &self.before
    }
    pub fn header(&self) -> &WorkHeader {
        &self.after
    }
    pub fn records(&self) -> impl Iterator<Item = (&str, Option<&WorkRecord>, &WorkRecord)> {
        self.records
            .iter()
            .map(|(id, change)| (id.as_str(), change.before.as_ref(), &change.after))
    }
    pub fn roots(&self) -> impl Iterator<Item = (&str, Option<&RootAccount>, &RootAccount)> {
        self.roots
            .iter()
            .map(|(id, change)| (id.as_str(), change.before.as_ref(), &change.after))
    }
    pub fn result(&self) -> &WorkResult {
        &self.result
    }
    pub fn into_result(self) -> WorkResult {
        self.result
    }
}
pub fn prepare_enqueue(
    read: &impl WorkRead,
    limits: &ReactionLimits,
    work: Vec<PendingWork>,
) -> Result<WorkDelta> {
    let mut engine = Engine::new(read)?;
    engine.enqueue(limits, work)?;
    engine.finish(WorkResult::Changed)
}
pub fn prepare_update(read: &impl WorkRead, update: WorkUpdate) -> Result<WorkDelta> {
    let mut engine = Engine::new(read)?;
    engine.compatible_capacity()?;
    let result = if engine.header.limits.is_none() {
        WorkResult::Idle
    } else {
        engine.update(update)?
    };
    engine.finish(result)
}

/// Work portion of one atomic Resource bundle. No method publishes a transition.
/// A transaction-local edit. Any failed step prevents publication of the edit.
pub struct WorkEdit<'tx, R: WorkRead> {
    engine: Engine<'tx, R>,
    failure: Option<Error>,
}
impl<'tx, R: WorkRead> WorkEdit<'tx, R> {
    pub fn new(read: &'tx R) -> Result<Self> {
        Ok(Self {
            engine: Engine::new(read)?,
            failure: None,
        })
    }
    fn ready(&self) -> Result<()> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    fn remember<T>(&mut self, result: Result<T>) -> Result<T> {
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    pub fn claim_retry_epoch(&mut self, claim: &ClaimKey, now: u64) -> Result<u64> {
        self.ready()?;
        let result = (|| {
            let record = self.engine.live_claim(claim, now)?;
            if matches!(
                record.state,
                WorkState::Leased {
                    resolution_only: Some(_),
                    ..
                }
            ) {
                return Err(Error::Conflict);
            }
            Ok(record.pending.cause.retry_epoch)
        })();
        self.remember(result)
    }
    pub fn complete(&mut self, claim: ClaimKey, now: u64) -> Result<()> {
        self.ready()?;
        let result = (|| {
            self.engine.compatible_capacity()?;
            if self.engine.header.limits.is_some() {
                self.engine.update(WorkUpdate::Finish {
                    claim,
                    now,
                    outcome: WorkOutcome::Done,
                })?;
            }
            self.engine.checkpoint()
        })();
        self.remember(result)
    }
    pub fn enqueue(&mut self, limits: &ReactionLimits, work: Vec<PendingWork>) -> Result<()> {
        self.ready()?;
        let result = self.engine.enqueue(limits, work);
        self.remember(result)
    }
    pub fn finish(self) -> Result<WorkDelta> {
        self.ready()?;
        self.engine.finish(WorkResult::Changed)
    }
}
