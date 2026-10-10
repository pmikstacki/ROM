//! Transactional maintenance of scalar memberships and kind counters.
use super::{derivation, encode, metadata};
use rom::{Descriptor, Error, Result, Row};
use rom_backup::{BackupLimits, Snapshot};
use rusqlite::{Connection, params};
use std::collections::BTreeSet;

pub(crate) fn initialize(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE query_keys(
        kind TEXT NOT NULL COLLATE BINARY,field TEXT NOT NULL COLLATE BINARY,id TEXT NOT NULL COLLATE BINARY,
        encoded BLOB NOT NULL CHECK(typeof(encoded)='blob'),PRIMARY KEY(kind,id,field)) WITHOUT ROWID;
        CREATE INDEX query_keys_value ON query_keys(kind,field,encoded,id);
        CREATE TABLE query_kinds(kind TEXT PRIMARY KEY NOT NULL COLLATE BINARY,
        row_count INTEGER NOT NULL CHECK(row_count>=0),row_bytes INTEGER NOT NULL CHECK(row_bytes>=0),
        canonical_row_bytes INTEGER NOT NULL CHECK(canonical_row_bytes>=0),live_count INTEGER NOT NULL CHECK(live_count>=0),
        generation BLOB NOT NULL CHECK(typeof(generation)='blob' AND length(generation)=8)) WITHOUT ROWID;
        CREATE TABLE query_profile(id INTEGER PRIMARY KEY CHECK(id=1),encoding_version INTEGER NOT NULL,
        profile_version INTEGER NOT NULL,store TEXT NOT NULL);
").map_err(|_|Error::NotCommitted)?;
    metadata::reset_profile(c)
}

pub(crate) fn register(c: &Connection, descriptors: &[Descriptor]) -> Result<()> {
    metadata::profile(c)?;
    let mut kinds = BTreeSet::new();
    for descriptor in descriptors {
        let canonical = descriptor.canonical()?;
        if !kinds.insert(canonical.kind.clone()) {
            return Err(Error::Duplicate("resource descriptor".into()));
        }
        if metadata::descriptor(c, &canonical.kind)? != canonical {
            return Err(Error::Unsupported(
                "resource schema migration required".into(),
            ));
        }
        if metadata::load(c, &canonical.kind)?.is_none() {
            let occupied: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM resources WHERE kind=?)",
                    [&canonical.kind],
                    |r| r.get(0),
                )
                .map_err(|_| Error::Storage)?;
            if occupied {
                return Err(Error::Storage);
            }
            metadata::insert(c, &canonical.kind, &metadata::Counters::default())?;
        }
    }
    Ok(())
}

pub(crate) fn replace(
    c: &Connection,
    old: Option<&Row>,
    new: &Row,
    old_bytes: Option<usize>,
    mut checkpoint: impl FnMut() -> Result<()>,
) -> Result<()> {
    let descriptor = metadata::descriptor(c, &new.key.kind)?;
    let prepared = prepare(c, old, new, old_bytes, &descriptor, None)?;
    publish(c, new, &prepared, &mut checkpoint)
}

enum Change {
    Update(String, Vec<u8>),
    Delete(String),
    Insert(String, Vec<u8>),
}
pub(crate) struct Prepared {
    changes: Vec<Change>,
    next: metadata::Counters,
}
pub(crate) fn prepare(
    c: &Connection,
    old: Option<&Row>,
    new: &Row,
    old_bytes: Option<usize>,
    descriptor: &Descriptor,
    reader: Option<&crate::native_work::Reader<'_>>,
) -> Result<Prepared> {
    if old.is_some_and(|row| row.key != new.key) {
        return Err(Error::Storage);
    }
    let current = metadata::load_admitted(c, &new.key.kind, reader)?.ok_or(Error::Storage)?;
    // Check every arithmetic operation before the first physical write.
    let next = current.replace(old, new, old_bytes)?;
    descriptor
        .reference_targets(new.value.as_ref())
        .map_err(|_| Error::Storage)?;
    let mut expected = 0_u64;
    if let Some(old) = old {
        derivation::keys(descriptor, old, |field, encoded| {
            let mut statement = c
                .prepare("SELECT encoded FROM query_keys WHERE kind=? AND field=? AND id=?")
                .map_err(|_| Error::Storage)?;
            let mut rows = statement
                .query(params![new.key.kind, field, new.key.id])
                .map_err(|_| Error::Storage)?;
            let actual = if let Some(row) = rows.next().map_err(|_| Error::Storage)? {
                let raw = row
                    .get_ref(0)
                    .map_err(|_| Error::Storage)?
                    .as_blob()
                    .map_err(|_| Error::Storage)?;
                if let Some(reader) = reader {
                    reader.charge(
                        raw.len()
                            .checked_add(new.key.kind.len())
                            .and_then(|n| n.checked_add(field.len()))
                            .and_then(|n| n.checked_add(new.key.id.len()))
                            .ok_or(Error::TooLarge)?,
                        1,
                    )?;
                }
                Some(raw.to_vec())
            } else {
                None
            };
            if actual.as_ref() != Some(&encoded) {
                return Err(Error::Storage);
            }
            expected = expected.checked_add(1).ok_or(Error::TooLarge)?;
            Ok(())
        })?;
    }
    let actual: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM query_keys WHERE kind=? AND id=?",
            params![new.key.kind, new.key.id],
            |r| r.get(0),
        )
        .map_err(|_| Error::Storage)?;
    if u64::try_from(actual).map_err(|_| Error::Storage)? != expected {
        return Err(Error::Storage);
    }
    let mut changes = Vec::new();
    for field in descriptor
        .fields
        .iter()
        .filter(|field| field.shape.is_scalar())
    {
        let previous = old
            .and_then(|row| row.value.as_ref())
            .map(|value| encode(&field.shape, value.get(&field.name)))
            .transpose()?;
        let next = new
            .value
            .as_ref()
            .map(|value| encode(&field.shape, value.get(&field.name)))
            .transpose()?;
        if previous == next {
            continue;
        }
        let change = match (previous, next) {
            (Some(_), Some(encoded)) => Change::Update(field.name.clone(), encoded),
            (Some(_), None) => Change::Delete(field.name.clone()),
            (None, Some(encoded)) => Change::Insert(field.name.clone(), encoded),
            (None, None) => continue,
        };
        changes.push(change);
    }
    Ok(Prepared { changes, next })
}
pub(crate) fn publish(
    c: &Connection,
    new: &Row,
    prepared: &Prepared,
    mut checkpoint: impl FnMut() -> Result<()>,
) -> Result<()> {
    for change in &prepared.changes {
        let changed = match change {
            Change::Update(field, encoded) => c.execute(
                "UPDATE query_keys SET encoded=? WHERE kind=? AND field=? AND id=?",
                params![encoded, new.key.kind, field, new.key.id],
            ),
            Change::Delete(field) => c.execute(
                "DELETE FROM query_keys WHERE kind=? AND field=? AND id=?",
                params![new.key.kind, field, new.key.id],
            ),
            Change::Insert(field, encoded) => c.execute(
                "INSERT INTO query_keys(kind,field,id,encoded) VALUES(?,?,?,?)",
                params![new.key.kind, field, new.key.id, encoded],
            ),
        }
        .map_err(|_| Error::NotCommitted)?;
        if changed != 1 {
            return Err(Error::Storage);
        }
        checkpoint()?;
    }
    metadata::update(c, &new.key.kind, &prepared.next)?;
    checkpoint()?;
    Ok(())
}

pub(crate) fn rebuild(c: &Connection, snapshot: &Snapshot, limits: BackupLimits) -> Result<()> {
    // Rebuild remains inside the caller's unpublished transaction. No source is repaired.
    c.execute_batch("DELETE FROM query_keys; DELETE FROM query_kinds;")
        .map_err(|_| Error::NotCommitted)?;
    metadata::reset_profile(c)?;
    let mut budget = derivation::Budget::new(limits);
    budget.charge(
        metadata::profile(c)?
            .len()
            .checked_add(24)
            .ok_or(Error::TooLarge)?,
    )?;
    let counts = derivation::rows(c, snapshot, limits, |descriptor, row| {
        derivation::keys(descriptor, row, |field, encoded| {
            budget.charge(derivation::entry_bytes(
                &row.key.kind,
                field,
                &row.key.id,
                &encoded,
            )?)?;
            c.execute(
                "INSERT INTO query_keys(kind,field,id,encoded) VALUES(?,?,?,?)",
                params![row.key.kind, field, row.key.id, encoded],
            )
            .map_err(|_| Error::NotCommitted)?;
            Ok(())
        })
    })?;
    for (kind, count) in counts {
        budget.charge(kind.len().checked_add(40).ok_or(Error::TooLarge)?)?;
        metadata::insert(c, &kind, &count)?;
    }
    Ok(())
}
