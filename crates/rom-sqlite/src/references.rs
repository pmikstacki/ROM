//! Persisted schema registration and reference checks inside native transactions.
use crate::{Sqlite, persistence::row};
use rom::{Descriptor, Error, Key, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{BTreeMap, BTreeSet};

fn descriptor(c: &Connection, kind: &str) -> Result<Option<Descriptor>> {
    let data: Option<String> = c
        .query_row("SELECT data FROM schemas WHERE kind=?", [kind], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| Error::Storage)?;
    data.map(|data| decode_descriptor(kind, &data)).transpose()
}

pub(crate) fn decode_descriptor(kind: &str, data: &str) -> Result<Descriptor> {
    let descriptor: Descriptor = serde_json::from_str(data).map_err(|_| Error::Storage)?;
    if descriptor.kind != kind || descriptor.canonical()? != descriptor {
        return Err(Error::Storage);
    }
    Ok(descriptor)
}

pub(super) fn register(c: &Connection, definitions: &[Descriptor]) -> Result<()> {
    let mut supplied = BTreeMap::new();
    for definition in definitions {
        let canonical = definition.canonical()?;
        if supplied.insert(canonical.kind.clone(), canonical).is_some() {
            return Err(Error::Duplicate("resource descriptor".into()));
        }
    }
    let mut all = BTreeMap::new();
    let mut statement = c
        .prepare("SELECT kind,data FROM schemas ORDER BY kind")
        .map_err(|_| Error::Storage)?;
    let mut cursor = statement.query([]).map_err(|_| Error::Storage)?;
    while let Some(row) = cursor.next().map_err(|_| Error::Storage)? {
        let kind: String = row.get(0).map_err(|_| Error::Storage)?;
        let data: String = row.get(1).map_err(|_| Error::Storage)?;
        let stored = decode_descriptor(&kind, &data)?;
        all.insert(kind, stored);
    }
    drop(cursor);
    drop(statement);
    let mut additions = Vec::new();
    for (kind, definition) in supplied {
        if let Some(stored) = all.get(&kind) {
            if stored != &definition {
                return Err(Error::Unsupported(
                    "resource schema migration required".into(),
                ));
            }
        } else {
            let occupied: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM resources WHERE kind=?)",
                    [&kind],
                    |r| r.get(0),
                )
                .map_err(|_| Error::Storage)?;
            if occupied {
                return Err(Error::Unsupported(
                    "unbound resource schema migration required".into(),
                ));
            }
            all.insert(kind, definition.clone());
            additions.push(definition);
        }
    }
    rom::validate_descriptors(&all.into_values().collect::<Vec<_>>())?;
    for definition in additions {
        c.execute(
            "INSERT INTO schemas(kind,data) VALUES (?,?)",
            params![
                definition.kind,
                serde_json::to_string(&definition).map_err(|_| Error::Storage)?
            ],
        )
        .map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}

pub(super) fn prepare(c: &Connection, receipt: &rom::Receipt) -> Result<Vec<Key>> {
    let definition = descriptor(c, &receipt.row.key.kind)?.ok_or(Error::Unregistered)?;
    prepare_with(c, receipt, &definition, |target| row(c, target))
}
pub(crate) fn prepare_with(
    c: &Connection,
    receipt: &rom::Receipt,
    definition: &Descriptor,
    mut load: impl FnMut(&Key) -> Result<Option<rom::Row>>,
) -> Result<Vec<Key>> {
    let candidate = &receipt.row;
    receipt.validate_new_version(definition)?;
    let targets = definition.reference_targets(candidate.value.as_ref())?;
    for target in &targets {
        if target == &candidate.key && candidate.value.is_some() {
            continue;
        }
        if !load(target)?.is_some_and(|row| row.value.is_some()) {
            return Err(Error::Conflict);
        }
    }
    if candidate.value.is_none() {
        let incoming: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM reference_edges INDEXED BY reference_edges_target
                 WHERE target_kind=?1 AND target_id=?2
                 AND (source_kind<>?1 OR source_id<>?2))",
                params![candidate.key.kind, candidate.key.id],
                |r| r.get(0),
            )
            .map_err(|_| Error::Storage)?;
        if incoming {
            return Err(Error::Conflict);
        }
    }
    Ok(targets)
}

pub(super) fn replace(
    storage: &Sqlite,
    c: &Connection,
    source: &Key,
    targets: &[Key],
    ordinal: &mut usize,
) -> Result<()> {
    let previous = previous(c, source, None)?;
    publish(storage, c, source, targets, &previous, ordinal)
}
pub(crate) fn previous(
    c: &Connection,
    source: &Key,
    reader: Option<&crate::native_work::Reader<'_>>,
) -> Result<BTreeSet<Key>> {
    let mut statement = c
        .prepare(
            "SELECT target_kind,target_id FROM reference_edges WHERE source_kind=? AND source_id=?",
        )
        .map_err(|_| Error::Storage)?;
    let mut rows = statement
        .query(params![source.kind, source.id])
        .map_err(|_| Error::Storage)?;
    let mut result = BTreeSet::new();
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let kind = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let id = row
            .get_ref(1)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        if let Some(reader) = reader {
            reader.charge(
                kind.len()
                    .checked_add(id.len())
                    .and_then(|n| n.checked_add(source.kind.len()))
                    .and_then(|n| n.checked_add(source.id.len()))
                    .ok_or(Error::TooLarge)?,
                1,
            )?;
        }
        result.insert(Key {
            kind: kind.to_owned(),
            id: id.to_owned(),
        });
    }
    Ok(result)
}
pub(crate) fn publish(
    storage: &Sqlite,
    c: &Connection,
    source: &Key,
    targets: &[Key],
    previous: &BTreeSet<Key>,
    ordinal: &mut usize,
) -> Result<()> {
    let current: BTreeSet<_> = targets.iter().cloned().collect();
    for target in previous.difference(&current) {
        c.execute(
            "DELETE FROM reference_edges WHERE source_kind=? AND source_id=? AND target_kind=? AND target_id=?",
            params![source.kind, source.id, target.kind, target.id],
        )
        .map_err(|_| Error::NotCommitted)?;
        *ordinal += 1;
        storage
            .checkpoint(*ordinal)
            .map_err(|_| Error::NotCommitted)?;
    }
    for target in current.difference(previous) {
        c.execute(
            "INSERT INTO reference_edges(source_kind,source_id,target_kind,target_id) VALUES (?,?,?,?)",
            params![source.kind, source.id, target.kind, target.id],
        )
        .map_err(|_| Error::NotCommitted)?;
        *ordinal += 1;
        storage
            .checkpoint(*ordinal)
            .map_err(|_| Error::NotCommitted)?;
    }
    Ok(())
}
