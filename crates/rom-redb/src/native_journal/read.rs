use crate::format::*;
use redb::{ReadOnlyTable, ReadTransaction, ReadableTable, Table, WriteTransaction};
use rom::storage_support::{
    metadata::{JournalEntry, JournalRead, MetadataHeader, MetadataHeaderParts},
    work::{ReadFence, RootAccount, WorkHeader, WorkHeaderParts, WorkRead},
};
use rom::{Error, JournalEvent, Result, Row, WorkRecord};
use rom_backup::BackupLimits;
use std::cell::{Cell, RefCell};

pub(crate) struct Budget {
    limits: BackupLimits,
    bytes: Cell<usize>,
    records: Cell<usize>,
}
impl Budget {
    pub(crate) fn new(limits: BackupLimits) -> Self {
        Self {
            limits,
            bytes: Cell::new(0),
            records: Cell::new(0),
        }
    }
    pub(crate) fn charge(&self, bytes: usize) -> Result<()> {
        let next = self.bytes.get().checked_add(bytes).ok_or(Error::TooLarge)?;
        let records = self.records.get().checked_add(1).ok_or(Error::TooLarge)?;
        if next > self.limits.max_bytes || records > self.limits.max_records {
            return Err(Error::TooLarge);
        }
        self.bytes.set(next);
        self.records.set(records);
        Ok(())
    }
}

pub(crate) fn header(
    state: &impl ReadableTable<&'static str, &'static str>,
    budget: &Budget,
) -> Result<MetadataHeader> {
    let raw = state
        .get("metadata")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    budget.charge(
        "metadata"
            .len()
            .checked_add(raw.value().len())
            .ok_or(Error::TooLarge)?,
    )?;
    let parts: MetadataHeaderParts =
        serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?;
    MetadataHeader::from_parts(parts)
}

/// One object implements both contracts: native facts and invalidation share authority.
pub(crate) struct Reader<S, P, E, W, R, A> {
    pub(crate) state: S,
    pub(crate) positions: P,
    pub(crate) events: E,
    pub(crate) records: W,
    pub(crate) roots: R,
    pub(crate) active: A,
    pub(crate) budget: Budget,
    fence: RefCell<ReadFence>,
    #[cfg(test)]
    pub(crate) examined: Cell<usize>,
}
type WriteReader<'tx> = Reader<
    Table<'tx, &'static str, &'static str>,
    Table<'tx, u64, &'static str>,
    Table<'tx, &'static str, &'static str>,
    Table<'tx, &'static str, &'static str>,
    Table<'tx, &'static str, &'static str>,
    Table<'tx, &'static str, u8>,
>;
type ReadReader = Reader<
    ReadOnlyTable<&'static str, &'static str>,
    ReadOnlyTable<u64, &'static str>,
    ReadOnlyTable<&'static str, &'static str>,
    ReadOnlyTable<&'static str, &'static str>,
    ReadOnlyTable<&'static str, &'static str>,
    ReadOnlyTable<&'static str, u8>,
>;
impl ReadReader {
    pub(crate) fn read_transaction(tx: &ReadTransaction, limits: BackupLimits) -> Result<Self> {
        Ok(Self::new(
            tx.open_table(STATE).map_err(|_| Error::Storage)?,
            tx.open_table(POSITIONS).map_err(|_| Error::Storage)?,
            tx.open_table(EVENTS).map_err(|_| Error::Storage)?,
            tx.open_table(WORK).map_err(|_| Error::Storage)?,
            tx.open_table(ROOTS).map_err(|_| Error::Storage)?,
            tx.open_table(ACTIVE).map_err(|_| Error::Storage)?,
            limits,
        ))
    }
}
impl<'tx> WriteReader<'tx> {
    pub(crate) fn writer(tx: &'tx WriteTransaction, limits: BackupLimits) -> Result<Self> {
        Ok(Self::new(
            tx.open_table(STATE).map_err(|_| Error::Storage)?,
            tx.open_table(POSITIONS).map_err(|_| Error::Storage)?,
            tx.open_table(EVENTS).map_err(|_| Error::Storage)?,
            tx.open_table(WORK).map_err(|_| Error::Storage)?,
            tx.open_table(ROOTS).map_err(|_| Error::Storage)?,
            tx.open_table(ACTIVE).map_err(|_| Error::Storage)?,
            limits,
        ))
    }
}
impl<S, P, E, W, R, A> Reader<S, P, E, W, R, A> {
    pub(crate) fn new(
        state: S,
        positions: P,
        events: E,
        records: W,
        roots: R,
        active: A,
        limits: BackupLimits,
    ) -> Self {
        Self {
            state,
            positions,
            events,
            records,
            roots,
            active,
            budget: Budget::new(limits),
            fence: RefCell::new(ReadFence::new()),
            #[cfg(test)]
            examined: Cell::new(0),
        }
    }
    pub(crate) fn invalidate(&self) {
        *self.fence.borrow_mut() = ReadFence::new();
    }
    pub(crate) fn decode<T: serde::de::DeserializeOwned>(
        &self,
        table: &impl ReadableTable<&'static str, &'static str>,
        key: &str,
    ) -> Result<Option<T>> {
        table
            .get(key)
            .map_err(|_| Error::Storage)?
            .map(|raw| {
                self.budget.charge(
                    key.len()
                        .checked_add(raw.value().len())
                        .ok_or(Error::TooLarge)?,
                )?;
                serde_json::from_str(raw.value()).map_err(|_| Error::Storage)
            })
            .transpose()
    }
}
impl<S, P, E, W, R, A> JournalRead for Reader<S, P, E, W, R, A>
where
    S: ReadableTable<&'static str, &'static str>,
    P: ReadableTable<u64, &'static str>,
    E: ReadableTable<&'static str, &'static str>,
{
    fn coherence(&self) -> ReadFence {
        self.fence.borrow().clone()
    }
    fn header(&self) -> Result<MetadataHeader> {
        header(&self.state, &self.budget)
    }
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>> {
        let Some(raw) = self.positions.get(position).map_err(|_| Error::Storage)? else {
            return Ok(None);
        };
        self.budget.charge(
            8usize
                .checked_add(raw.value().len())
                .ok_or(Error::TooLarge)?,
        )?;
        #[cfg(test)]
        self.examined.set(self.examined.get() + 1);
        let index: super::write::Position =
            serde_json::from_str(raw.value()).map_err(|_| Error::Storage)?;
        if position == 0 || index.identity.is_empty() {
            return Err(Error::Storage);
        }
        let row: Row = self
            .decode(&self.events, &index.identity)?
            .ok_or(Error::Storage)?;
        let event = JournalEvent {
            position,
            identity: index.identity,
            row,
        };
        let actual = serde_json::to_vec(&event)
            .map_err(|_| Error::Storage)?
            .len();
        let bytes = usize::try_from(index.canonical_bytes).map_err(|_| Error::TooLarge)?;
        if actual != bytes {
            return Err(Error::Storage);
        }
        Ok(Some(JournalEntry {
            event,
            encoded_bytes: bytes,
        }))
    }
}
impl<S, P, E, W, R, A> WorkRead for Reader<S, P, E, W, R, A>
where
    S: ReadableTable<&'static str, &'static str>,
    W: ReadableTable<&'static str, &'static str>,
    R: ReadableTable<&'static str, &'static str>,
    A: ReadableTable<&'static str, u8>,
{
    fn coherence(&self) -> ReadFence {
        self.fence.borrow().clone()
    }
    fn header(&self) -> Result<WorkHeader> {
        let parts: WorkHeaderParts = self
            .decode(&self.state, "work_header")?
            .ok_or(Error::Storage)?;
        WorkHeader::from_parts(parts)
    }
    fn record(&self, id: &str) -> Result<Option<WorkRecord>> {
        let record: Option<WorkRecord> = self.decode(&self.records, id)?;
        if record.as_ref().is_some_and(|r| r.pending.id != id) {
            return Err(Error::Storage);
        }
        Ok(record)
    }
    fn root(&self, id: &str) -> Result<Option<RootAccount>> {
        self.decode(&self.roots, id)
    }
    fn next_candidate(&self, now: u64, after_id: Option<&str>) -> Result<Option<WorkRecord>> {
        crate::native_work::next_candidate(
            &self.active,
            &WorkRead::header(self)?,
            now,
            after_id,
            |id| {
                self.budget
                    .charge(id.len().checked_add(1).ok_or(Error::TooLarge)?)?;
                Ok(())
            },
            |id| self.record(id),
        )
    }
}
