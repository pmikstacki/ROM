//! Persisted schema and indexed restrict relations. All writes use the bundle transaction.
use crate::{Redb, format::*};
use redb::{Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use rom::{Descriptor, Error, Key, Result, Row};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::Ordering;

pub(super) const SCHEMAS: TableDefinition<&str, &str> = TableDefinition::new("rom_schemas");
type EdgeKey<'a> = (&'a str, &'a str, &'a str, &'a str);
pub(super) const OUTGOING: TableDefinition<EdgeKey<'_>, u8> =
    TableDefinition::new("rom_references_out");
pub(super) const INCOMING: TableDefinition<EdgeKey<'_>, u8> =
    TableDefinition::new("rom_references_in");

fn edge_key<'a>(source: &'a Key, target: &'a Key) -> EdgeKey<'a> {
    (&source.kind, &source.id, &target.kind, &target.id)
}
fn schemas(
    table: &impl ReadableTable<&'static str, &'static str>,
) -> Result<BTreeMap<String, Descriptor>> {
    let mut result = BTreeMap::new();
    for entry in table.iter().map_err(|_| Error::Storage)? {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        let descriptor: Descriptor =
            serde_json::from_str(value.value()).map_err(|_| Error::Storage)?;
        if descriptor.kind != key.value()
            || descriptor.canonical().map_err(|_| Error::Storage)? != descriptor
        {
            return Err(Error::Storage);
        }
        result.insert(descriptor.kind.clone(), descriptor);
    }
    rom::validate_descriptors(&result.values().cloned().collect::<Vec<_>>())
        .map_err(|_| Error::Storage)?;
    Ok(result)
}
fn targets(
    table: &impl ReadableTable<EdgeKey<'static>, u8>,
    source: &Key,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<Vec<Key>> {
    let mut result = Vec::new();
    for entry in table
        .range((source.kind.as_str(), source.id.as_str(), "", "")..)
        .map_err(|_| Error::Storage)?
    {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        let (kind, id, target_kind, target_id) = key.value();
        if kind != source.kind || id != source.id {
            break;
        }
        charge(edge_bytes((kind, id, target_kind, target_id))?)?;
        if value.value() != 1 || target_kind.is_empty() || target_id.is_empty() {
            return Err(Error::Storage);
        }
        result.push(Key {
            kind: target_kind.into(),
            id: target_id.into(),
        });
    }
    Ok(result)
}
fn live_target(
    rows: &impl ReadableTable<(&'static str, &'static str), &'static str>,
    target: &Key,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<bool> {
    let Some(value) = rows
        .get((target.kind.as_str(), target.id.as_str()))
        .map_err(|_| Error::Storage)?
    else {
        return Ok(false);
    };
    charge(
        target
            .kind
            .len()
            .checked_add(target.id.len())
            .and_then(|n| n.checked_add(value.value().len()))
            .ok_or(Error::TooLarge)?,
    )?;
    let row: Row = serde_json::from_str(value.value()).map_err(|_| Error::Storage)?;
    if row.key != *target {
        return Err(Error::Storage);
    }
    Ok(row.value.is_some())
}

impl Redb {
    pub(super) fn register_descriptors(&self, descriptors: &[Descriptor]) -> Result<()> {
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        if self.native_format == JOURNAL_FORMAT {
            let read = self.db.begin_read().map_err(|_| Error::Storage)?;
            crate::admission::check(&read, self.native_format, self.validation_limits)?;
        }
        let mut tx = self.db.begin_write().map_err(|_| Error::Storage)?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        let changed = {
            let mut table = tx.open_table(SCHEMAS).map_err(|_| Error::Storage)?;
            let mut all = schemas(&table)?;
            let rows = tx.open_table(ROWS).map_err(|_| Error::Storage)?;
            let mut seen = BTreeSet::new();
            let mut added = Vec::new();
            for descriptor in descriptors {
                let descriptor = descriptor.canonical()?;
                if !seen.insert(descriptor.kind.clone()) {
                    return Err(Error::Unsupported("duplicate storage kind".into()));
                }
                if let Some(prior) = all.get(&descriptor.kind) {
                    if prior != &descriptor {
                        return Err(Error::Unsupported("stored schema differs".into()));
                    }
                } else {
                    if let Some(entry) = rows
                        .range((descriptor.kind.as_str(), "")..)
                        .map_err(|_| Error::Storage)?
                        .next()
                    {
                        let (key, _) = entry.map_err(|_| Error::Storage)?;
                        if key.value().0 == descriptor.kind {
                            return Err(Error::Unsupported(
                                "unbound rows require migration".into(),
                            ));
                        }
                    }
                    added.push(descriptor.clone());
                    all.insert(descriptor.kind.clone(), descriptor);
                }
            }
            rom::validate_descriptors(&all.values().cloned().collect::<Vec<_>>())?;
            for descriptor in &added {
                table
                    .insert(
                        descriptor.kind.as_str(),
                        serde_json::to_string(descriptor)
                            .map_err(|_| Error::Storage)?
                            .as_str(),
                    )
                    .map_err(|_| Error::NotCommitted)?;
            }
            !added.is_empty()
        };
        if changed && tx.commit().is_err() {
            self.uncertain.store(true, Ordering::Release);
            return Err(Error::Unknown);
        }
        Ok(())
    }

    pub(super) fn replace_references(
        &self,
        tx: &redb::WriteTransaction,
        source: &Key,
        old: &[Key],
        new: &[Key],
        ordinal: &mut usize,
    ) -> Result<()> {
        let mut outgoing = tx.open_table(OUTGOING).map_err(|_| Error::Storage)?;
        let mut incoming = tx.open_table(INCOMING).map_err(|_| Error::Storage)?;
        for target in old
            .iter()
            .filter(|target| new.binary_search(target).is_err())
        {
            if outgoing
                .remove(edge_key(source, target))
                .map_err(|_| Error::NotCommitted)?
                .is_none()
            {
                return Err(Error::Storage);
            }
            *ordinal += 1;
            self.checkpoint(*ordinal).map_err(|_| Error::NotCommitted)?;
            if incoming
                .remove(edge_key(target, source))
                .map_err(|_| Error::NotCommitted)?
                .is_none()
            {
                return Err(Error::Storage);
            }
            *ordinal += 1;
            self.checkpoint(*ordinal).map_err(|_| Error::NotCommitted)?;
        }
        for target in new
            .iter()
            .filter(|target| old.binary_search(target).is_err())
        {
            outgoing
                .insert(edge_key(source, target), 1)
                .map_err(|_| Error::NotCommitted)?;
            *ordinal += 1;
            self.checkpoint(*ordinal).map_err(|_| Error::NotCommitted)?;
            incoming
                .insert(edge_key(target, source), 1)
                .map_err(|_| Error::NotCommitted)?;
            *ordinal += 1;
            self.checkpoint(*ordinal).map_err(|_| Error::NotCommitted)?;
        }
        Ok(())
    }
}

pub(super) fn prepare(
    tx: &redb::WriteTransaction,
    rows: &impl ReadableTable<(&'static str, &'static str), &'static str>,
    receipt: &rom::Receipt,
    existing: Option<&Row>,
) -> Result<(Vec<Key>, Vec<Key>)> {
    prepare_with_budget(tx, rows, receipt, existing, &mut |_| Ok(()))
}

pub(super) fn prepare_with_budget(
    tx: &redb::WriteTransaction,
    rows: &impl ReadableTable<(&'static str, &'static str), &'static str>,
    receipt: &rom::Receipt,
    existing: Option<&Row>,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<(Vec<Key>, Vec<Key>)> {
    let row = &receipt.row;
    let table = tx.open_table(SCHEMAS).map_err(|_| Error::Storage)?;
    let value = table
        .get(row.key.kind.as_str())
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Unregistered)?;
    charge(
        row.key
            .kind
            .len()
            .checked_add(value.value().len())
            .ok_or(Error::TooLarge)?,
    )?;
    let descriptor: Descriptor = serde_json::from_str(value.value()).map_err(|_| Error::Storage)?;
    if descriptor.kind != row.key.kind
        || descriptor.canonical().map_err(|_| Error::Storage)? != descriptor
    {
        return Err(Error::Storage);
    }
    receipt.validate_new_version(&descriptor)?;
    if existing.is_some_and(|old| old.key != row.key) {
        return Err(Error::Storage);
    }
    let old = descriptor
        .reference_targets(existing.and_then(|old| old.value.as_ref()))
        .map_err(|_| Error::Storage)?;
    let new = descriptor.reference_targets(row.value.as_ref())?;
    let outgoing = tx.open_table(OUTGOING).map_err(|_| Error::Storage)?;
    let incoming = tx.open_table(INCOMING).map_err(|_| Error::Storage)?;
    if targets(&outgoing, &row.key, charge)? != old {
        return Err(Error::Storage);
    }
    for target in &old {
        let marker = incoming
            .get(edge_key(target, &row.key))
            .map_err(|_| Error::Storage)?;
        if marker.is_some() {
            charge(edge_bytes(edge_key(target, &row.key))?)?;
        }
        if marker.map(|value| value.value()) != Some(1) {
            return Err(Error::Storage);
        }
    }
    for target in &new {
        if *target == row.key {
            if row.value.is_none() {
                return Err(Error::Conflict);
            }
        } else if !live_target(rows, target, charge)? {
            return Err(Error::Conflict);
        }
    }
    if row.value.is_none() {
        for entry in incoming
            .range((row.key.kind.as_str(), row.key.id.as_str(), "", "")..)
            .map_err(|_| Error::Storage)?
        {
            let (key, value) = entry.map_err(|_| Error::Storage)?;
            let (kind, id, source_kind, source_id) = key.value();
            if kind != row.key.kind || id != row.key.id {
                break;
            }
            charge(edge_bytes((kind, id, source_kind, source_id))?)?;
            if value.value() != 1 {
                return Err(Error::Storage);
            }
            if source_kind != row.key.kind || source_id != row.key.id {
                return Err(Error::Conflict);
            }
        }
    }
    Ok((old, new))
}

fn edge_bytes(key: EdgeKey<'_>) -> Result<usize> {
    [key.0, key.1, key.2, key.3]
        .iter()
        .try_fold(1usize, |n, p| n.checked_add(p.len()).ok_or(Error::TooLarge))
}

pub(super) fn collect(
    tx: &redb::ReadTransaction,
    collector: &mut rom_backup::Collector,
) -> Result<()> {
    let outgoing = tx.open_table(OUTGOING).map_err(|_| Error::Storage)?;
    let incoming = tx.open_table(INCOMING).map_err(|_| Error::Storage)?;
    if outgoing.len().map_err(|_| Error::Storage)? != incoming.len().map_err(|_| Error::Storage)? {
        return Err(Error::Storage);
    }
    for entry in tx
        .open_table(SCHEMAS)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        collector.descriptor(key.value(), value.value())?;
    }
    for entry in outgoing.iter().map_err(|_| Error::Storage)? {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        let (sk, si, tk, ti) = key.value();
        if value.value() != 1
            || incoming
                .get((tk, ti, sk, si))
                .map_err(|_| Error::Storage)?
                .map(|v| v.value())
                != Some(1)
        {
            return Err(Error::Storage);
        }
        collector.reference(rom::ReferenceEdge {
            source: Key {
                kind: sk.into(),
                id: si.into(),
            },
            target: Key {
                kind: tk.into(),
                id: ti.into(),
            },
        })?;
    }
    Ok(())
}
