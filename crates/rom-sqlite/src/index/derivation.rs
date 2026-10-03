//! Bounded derivation from authoritative descriptors and Resource rows.
use super::{encode, metadata::Counters};
use rom::{Descriptor, Error, Result, Row};
use rom_backup::{BackupLimits, Snapshot};
use rusqlite::{Connection, params};
use std::collections::BTreeMap;

pub(super) fn keys(
    descriptor: &Descriptor,
    row: &Row,
    mut visit: impl FnMut(&str, Vec<u8>) -> Result<()>,
) -> Result<()> {
    if row.key.kind != descriptor.kind {
        return Err(Error::Storage);
    }
    descriptor
        .reference_targets(row.value.as_ref())
        .map_err(|_| Error::Storage)?;
    let Some(value) = row.value.as_ref() else {
        return Ok(());
    };
    for field in &descriptor.fields {
        if field.shape.is_scalar() {
            visit(&field.name, encode(&field.shape, value.get(&field.name))?)?;
        }
    }
    Ok(())
}

pub(super) fn entry_bytes(kind: &str, field: &str, id: &str, encoded: &[u8]) -> Result<usize> {
    [kind.len(), field.len(), id.len(), encoded.len()]
        .into_iter()
        .try_fold(0usize, |n, len| n.checked_add(len).ok_or(Error::TooLarge))
}

pub(super) struct Budget {
    records: usize,
    bytes: usize,
    limits: BackupLimits,
}
impl Budget {
    pub(super) fn new(limits: BackupLimits) -> Self {
        Self {
            records: 0,
            bytes: 0,
            limits,
        }
    }
    pub(super) fn charge(&mut self, bytes: usize) -> Result<()> {
        self.records = self.records.checked_add(1).ok_or(Error::TooLarge)?;
        self.bytes = self.bytes.checked_add(bytes).ok_or(Error::TooLarge)?;
        if self.records > self.limits.max_records || self.bytes > self.limits.max_bytes {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
}

/// Verify supplied logical rows against actual native text before deriving physical state.
/// Only the bounded descriptor/counter map is retained; memberships are visited individually.
pub(super) fn rows(
    c: &Connection,
    snapshot: &Snapshot,
    limits: BackupLimits,
    mut visit: impl FnMut(&Descriptor, &Row) -> Result<()>,
) -> Result<BTreeMap<String, Counters>> {
    if snapshot.descriptors.len() > limits.max_records || snapshot.rows.len() > limits.max_records {
        return Err(Error::TooLarge);
    }
    let catalog: BTreeMap<_, _> = snapshot
        .descriptors
        .iter()
        .map(|d| (d.kind.as_str(), d))
        .collect();
    if catalog.len() != snapshot.descriptors.len() {
        return Err(Error::Storage);
    }
    for descriptor in catalog.values() {
        if descriptor.canonical().map_err(|_| Error::Storage)? != **descriptor {
            return Err(Error::Storage);
        }
    }
    let actual: i64 = c
        .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if actual != i64::try_from(snapshot.rows.len()).map_err(|_| Error::TooLarge)? {
        return Err(Error::Storage);
    }
    let mut counts: BTreeMap<_, _> = catalog
        .keys()
        .map(|kind| ((*kind).to_owned(), Counters::default()))
        .collect();
    let mut budget = Budget::new(limits);
    let mut statement = c
        .prepare("SELECT revision,data FROM resources WHERE kind=? AND id=?")
        .map_err(|_| Error::Storage)?;
    for row in &snapshot.rows {
        let descriptor = catalog.get(row.key.kind.as_str()).ok_or(Error::Storage)?;
        let mut cursor = statement
            .query(params![row.key.kind, row.key.id])
            .map_err(|_| Error::Storage)?;
        let native = cursor
            .next()
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        let revision = u64::try_from(native.get::<_, i64>(0).map_err(|_| Error::Storage)?)
            .map_err(|_| Error::Storage)?;
        let text = native
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        budget.charge(text.len())?;
        let decoded: Row = serde_json::from_str(text).map_err(|_| Error::Storage)?;
        if revision != row.revision || decoded != *row {
            return Err(Error::Storage);
        }
        counts
            .get_mut(&row.key.kind)
            .ok_or(Error::Storage)?
            .add(row, text.len())?;
        if cursor.next().map_err(|_| Error::Storage)?.is_some() {
            return Err(Error::Storage);
        }
        visit(descriptor, row)?;
    }
    Ok(counts)
}
