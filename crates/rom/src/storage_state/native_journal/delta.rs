use super::*;
use crate::storage_support::work::{ReadFence, WorkDelta, WorkRead};
/// Exact journal reads and a candidate; native adapters validate before any write.
pub struct JournalDelta {
    pub(super) coherence: ReadFence,
    pub(super) before: MetadataHeader,
    pub(super) after: MetadataHeader,
    pub(super) appended: Option<JournalEntry>,
    pub(super) retired: Vec<JournalEntry>,
    pub(super) facts: BTreeMap<u64, Option<JournalEntry>>,
}
impl JournalDelta {
    pub fn coherence(&self) -> &ReadFence {
        &self.coherence
    }
    pub fn before_header(&self) -> &MetadataHeader {
        &self.before
    }
    pub fn header(&self) -> &MetadataHeader {
        &self.after
    }
    pub fn appended(&self) -> Option<&JournalEntry> {
        self.appended.as_ref()
    }
    pub fn retired(&self) -> &[JournalEntry] {
        &self.retired
    }
    pub fn entry_preconditions(&self) -> impl Iterator<Item = (u64, &Option<JournalEntry>)> {
        self.facts.iter().map(|(p, e)| (*p, e))
    }
    pub fn validate(&self, reader: &impl JournalRead) -> Result<()> {
        if !reader.coherence().same_context(&self.coherence) || reader.header()? != self.before {
            return Err(Error::Conflict);
        }
        for (p, e) in &self.facts {
            if &reader.entry(*p)? != e {
                return Err(Error::Conflict);
            }
        }
        if !reader.coherence().same_context(&self.coherence) {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}
/// Journal and Work candidates must be validated jointly before publication.
/// Native readers share a fence and invalidate it after combined publication.
pub struct NativeBundleDelta {
    pub(super) journal: JournalDelta,
    pub(super) work: WorkDelta,
}
impl NativeBundleDelta {
    pub fn journal(&self) -> &JournalDelta {
        &self.journal
    }
    pub fn work(&self) -> &WorkDelta {
        &self.work
    }
    pub fn into_parts(self) -> (JournalDelta, WorkDelta) {
        (self.journal, self.work)
    }
    pub fn validate(&self, journal: &impl JournalRead, work: &impl WorkRead) -> Result<()> {
        self.journal.validate(journal)?;
        self.work.validate(work)?;
        if !work.coherence().same_context(self.work.coherence())
            || !journal.coherence().same_context(self.journal.coherence())
        {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}
