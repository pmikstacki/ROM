//! Canonical import and a coherent keyed reader for the shared transition engine.
//! Full validation occurs at import; ordinary reads do not rebuild the ledger.
use super::accounting::{LedgerBytes, entry_for_root, entry_for_work};
use super::incremental::*;
use super::retention::completed;
use super::*;

pub struct WorkImage {
    coherence: ReadFence,
    header: WorkHeader,
    records: BTreeMap<String, WorkRecord>,
    roots: BTreeMap<String, RootAccount>,
    active: BTreeSet<String>,
}
impl WorkImage {
    /// Validate a bounded native import against every stored derived projection.
    pub fn from_native(
        header: WorkHeader,
        records: BTreeMap<String, WorkRecord>,
        roots: BTreeMap<String, RootAccount>,
        active: BTreeSet<String>,
    ) -> Result<Self> {
        let ledger = WorkLedger {
            limits: header.limits.clone(),
            work: records,
            roots: roots
                .iter()
                .map(|(id, root)| (id.clone(), root.used))
                .collect(),
        };
        let imported = Self::from_ledger(ledger, header.retry_epochs)?;
        if imported.header != header || imported.roots != roots || imported.active != active {
            return Err(Error::Storage);
        }
        Ok(imported)
    }
    pub fn from_ledger(ledger: WorkLedger, retry_epochs: RetryEpochs) -> Result<Self> {
        ledger.validate_archive()?;
        ledger.validate_retry_epochs(retry_epochs)?;
        let mut accounting = LedgerBytes::empty(ledger.limits.as_ref())?;
        let mut roots = BTreeMap::<String, RootAccount>::new();
        let mut active = BTreeSet::new();
        for (id, record) in &ledger.work {
            accounting.replace_work(None, Some(entry_for_work(id, record)?))?;
            let cause = &record.pending.cause;
            let root = roots.entry(cause.root.clone()).or_insert(RootAccount {
                used: 0,
                epoch: cause.retry_epoch,
                members: 0,
                incomplete: 0,
            });
            root.used = root
                .used
                .checked_add(record.attempts)
                .ok_or(Error::TooLarge)?;
            root.members = root.members.checked_add(1).ok_or(Error::TooLarge)?;
            root.incomplete = root
                .incomplete
                .checked_add(usize::from(!completed(record)))
                .ok_or(Error::TooLarge)?;
            if matches!(record.state, WorkState::Pending | WorkState::Leased { .. }) {
                active.insert(id.clone());
            }
        }
        for (id, used) in &ledger.roots {
            accounting.replace_root(None, Some(entry_for_root(id, *used)?))?;
        }
        Ok(Self {
            coherence: ReadFence::new(),
            header: WorkHeader {
                limits: ledger.limits,
                retry_epochs,
                accounting,
            },
            records: ledger.work,
            roots,
            active,
        })
    }
    /// Explicit archive/test reconstruction; ordinary transition paths do not call it.
    pub fn canonical(&self) -> WorkLedger {
        WorkLedger {
            limits: self.header.limits.clone(),
            work: self.records.clone(),
            roots: self
                .roots
                .iter()
                .map(|(id, root)| (id.clone(), root.used))
                .collect(),
        }
    }
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
    /// Keyed canonical records for explicit native import and full validation.
    pub fn records(&self) -> impl Iterator<Item = (&str, &WorkRecord)> {
        self.records
            .iter()
            .map(|(id, record)| (id.as_str(), record))
    }
    pub fn roots(&self) -> impl Iterator<Item = (&str, &RootAccount)> {
        self.roots.iter().map(|(id, root)| (id.as_str(), root))
    }
    pub fn active_ids(&self) -> impl Iterator<Item = &str> {
        self.active.iter().map(String::as_str)
    }
    pub fn apply(&mut self, delta: WorkDelta) -> Result<WorkResult> {
        delta.validate(self)?;
        // No fallible state mutation occurs until every read precondition agrees.
        for (id, _, record) in delta.records() {
            if matches!(record.state, WorkState::Pending | WorkState::Leased { .. }) {
                self.active.insert(id.to_owned());
            } else {
                self.active.remove(id);
            }
            self.records.insert(id.to_owned(), record.clone());
        }
        for (id, _, root) in delta.roots() {
            self.roots.insert(id.to_owned(), *root);
        }
        self.header = delta.header().clone();
        self.coherence = ReadFence::new();
        Ok(delta.into_result())
    }
}
impl WorkRead for WorkImage {
    fn coherence(&self) -> ReadFence {
        self.coherence.clone()
    }
    fn header(&self) -> Result<WorkHeader> {
        Ok(self.header.clone())
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        Ok(self.records.get(id).cloned())
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        Ok(self.roots.get(id).copied())
    }
    /// Scans active IDs only. Worst-case cost remains proportional to active work.
    fn next_candidate(&self, now: u64, after_id: Option<&str>) -> Result<Option<WorkRecord>> {
        use std::ops::Bound::{Excluded, Unbounded};
        let Some(limits) = &self.header.limits else {
            return Ok(None);
        };
        let start = after_id.map_or(Unbounded, Excluded);
        for id in self.active.range::<str, _>((start, Unbounded)) {
            let record = self.records.get(id).ok_or(Error::Storage)?;
            let eligible = match record.state {
                WorkState::Pending => {
                    record.due <= now
                        || now.saturating_sub(record.pending.cause.started_at)
                            >= limits.max_age_seconds
                }
                WorkState::Leased { until, .. } => until <= now,
                _ => return Err(Error::Storage),
            };
            if eligible {
                return Ok(Some(record.clone()));
            }
        }
        Ok(None)
    }
}
