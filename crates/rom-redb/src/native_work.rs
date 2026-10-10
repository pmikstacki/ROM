//! Keyed Work transitions inside the originating redb writer transaction.
use crate::format::{ACTIVE, ROOTS, WORK};
use redb::{ReadableTable, Table, WriteTransaction};
use rom::storage_support::work::{
    ReadFence, RootAccount, WorkDelta, WorkHeader, WorkHeaderParts, WorkRead,
};
use rom::{Error, Result, WorkRecord, WorkResult, WorkState};
mod publication;
pub(super) use publication::PreparedWork;
use std::ops::Bound::{Excluded, Unbounded};

pub(super) struct NativeWork<'tx> {
    #[cfg(test)]
    examined: std::cell::Cell<usize>,
    header: WorkHeader,
    fence: ReadFence,
    records: Table<'tx, &'static str, &'static str>,
    roots: Table<'tx, &'static str, &'static str>,
    active: Table<'tx, &'static str, u8>,
}
impl<'tx> NativeWork<'tx> {
    pub fn new(
        tx: &'tx WriteTransaction,
        state: &impl ReadableTable<&'static str, &'static str>,
    ) -> Result<Self> {
        let raw = state
            .get("work_header")
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        let parts: WorkHeaderParts =
            serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?;
        Ok(Self {
            #[cfg(test)]
            examined: std::cell::Cell::new(0),
            header: WorkHeader::from_parts(parts)?,
            fence: ReadFence::new(),
            records: tx.open_table(WORK).map_err(|_| Error::Storage)?,
            roots: tx.open_table(ROOTS).map_err(|_| Error::Storage)?,
            active: tx.open_table(ACTIVE).map_err(|_| Error::Storage)?,
        })
    }
    #[cfg(test)]
    pub fn examined_candidates(&self) -> usize {
        self.examined.get()
    }
    pub fn apply(
        &mut self,
        delta: WorkDelta,
        state: &mut Table<'_, &'static str, &'static str>,
        checkpoint: &mut impl FnMut() -> Result<()>,
    ) -> Result<WorkResult> {
        let raw = state
            .get("work_header")
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        let parts = serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?;
        drop(raw);
        let persisted = WorkHeader::from_parts(parts)?;
        if persisted != self.header {
            return Err(Error::Conflict);
        }
        delta.validate(self)?;
        let prepared = PreparedWork::new(&delta)?;
        prepared.publish(
            state,
            &mut self.records,
            &mut self.roots,
            &mut self.active,
            checkpoint,
        )?;
        self.header = delta.header().clone();
        self.fence = ReadFence::new();
        Ok(delta.into_result())
    }
}
impl WorkRead for NativeWork<'_> {
    fn coherence(&self) -> ReadFence {
        self.fence.clone()
    }
    fn header(&self) -> Result<WorkHeader> {
        Ok(self.header.clone())
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        let record: Option<WorkRecord> = self
            .records
            .get(id)
            .map_err(|_| Error::Storage)?
            .map(|raw| serde_json::from_str(raw.value()).map_err(|_| Error::Storage))
            .transpose()?;
        if record
            .as_ref()
            .is_some_and(|record| record.pending.id != id)
        {
            return Err(Error::Storage);
        }
        Ok(record)
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        self.roots
            .get(id)
            .map_err(|_| Error::Storage)?
            .map(|raw| serde_json::from_str(raw.value()).map_err(|_| Error::Storage))
            .transpose()
    }
    fn next_candidate(&self, now: u64, after_id: Option<&str>) -> Result<Option<WorkRecord>> {
        next_candidate(
            &self.active,
            &self.header,
            now,
            after_id,
            |_| {
                #[cfg(test)]
                self.examined
                    .set(self.examined.get().checked_add(1).ok_or(Error::TooLarge)?);
                Ok(())
            },
            |id| self.record(id),
        )
    }
}

#[cfg(all(test, feature = "test-support"))]
mod publication_tests;

/// Shared active selection preserves the existing Work ordering and lifecycle rules.
pub(super) fn next_candidate(
    active: &impl ReadableTable<&'static str, u8>,
    header: &WorkHeader,
    now: u64,
    after_id: Option<&str>,
    mut admit: impl FnMut(&str) -> Result<()>,
    mut record: impl FnMut(&str) -> Result<Option<WorkRecord>>,
) -> Result<Option<WorkRecord>> {
    let Some(limits) = &header.limits else {
        return Ok(None);
    };
    let start = after_id.map_or(Unbounded, Excluded);
    for entry in active
        .range::<&str>((start, Unbounded))
        .map_err(|_| Error::Storage)?
    {
        let (id, marker) = entry.map_err(|_| Error::Storage)?;
        admit(id.value())?;
        if marker.value() != 1 {
            return Err(Error::Storage);
        }
        let record = record(id.value())?.ok_or(Error::Storage)?;
        let eligible = match record.state {
            WorkState::Pending => {
                record.due <= now
                    || now.saturating_sub(record.pending.cause.started_at) >= limits.max_age_seconds
            }
            WorkState::Leased { until, .. } => until <= now,
            _ => return Err(Error::Storage),
        };
        if eligible {
            return Ok(Some(record));
        }
    }
    Ok(None)
}
