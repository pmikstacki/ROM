//! Bounded coherent reads of native tables for open, backup and legacy upgrade.
use rom::{Error, Key, Result};
use rom_backup::{BackupLimits, Collector, Snapshot};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Collect a coherent source transaction, including committed WAL records, without writes.
pub(super) fn read_snapshot(
    source: &Path,
    limits: BackupLimits,
    collect: impl FnOnce(&Connection, BackupLimits) -> Result<Snapshot>,
) -> Result<Snapshot> {
    let mut connection = open_offline(source)?;
    let transaction = connection.transaction().map_err(|_| Error::Storage)?;
    collect(&transaction, limits)
}

/// Clean WAL-mode databases otherwise create empty sidecars even in READ_ONLY mode.
/// Keep ordinary WAL reads; reject rollback journals that need host-controlled recovery.
/// The host must keep the source offline: immutable mode skips change detection.
fn open_offline(source: &Path) -> Result<Connection> {
    let source = source.canonicalize().map_err(|_| Error::Storage)?;
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if has_sidecar(&source, "-journal")? {
        return Err(Error::Unsupported(
            "offline SQLite source has a rollback journal".into(),
        ));
    }
    if has_sidecar(&source, "-wal")? {
        return Connection::open_with_flags(source, flags).map_err(|_| Error::Storage);
    }
    let mut uri = url::Url::from_file_path(&source).map_err(|_| Error::Storage)?;
    uri.query_pairs_mut().append_pair("immutable", "1");
    Connection::open_with_flags(uri.as_str(), flags | OpenFlags::SQLITE_OPEN_URI)
        .map_err(|_| Error::Storage)
}
fn has_sidecar(source: &Path, suffix: &str) -> Result<bool> {
    let mut path = source.as_os_str().to_os_string();
    path.push(suffix);
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Error::Storage),
    }
}

pub(super) fn collect_snapshot(c: &Connection, limits: BackupLimits) -> Result<Snapshot> {
    collect_snapshot_for_format(c, limits, rom_backup::STORAGE_FORMAT)
}

pub(super) fn collect_upgrade_snapshot(
    c: &Connection,
    limits: BackupLimits,
    descriptors: &[rom::Descriptor],
) -> Result<Snapshot> {
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    let snapshot = collect_supported_snapshot(c, limits, &[3, 4, 5, 6])?;
    if version == 3 {
        rom_backup::bind_legacy_schema(snapshot, descriptors, limits)
    } else if version == 6 {
        rom_backup::upgrade_current_snapshot(snapshot, descriptors, limits)
    } else {
        rom_backup::upgrade_legacy_snapshot(snapshot, descriptors, limits)
    }
}

pub(super) fn collect_migration_snapshot(c: &Connection, limits: BackupLimits) -> Result<Snapshot> {
    collect_supported_snapshot(c, limits, &[4, 5, 6, rom_backup::STORAGE_FORMAT])
}

fn collect_supported_snapshot(
    c: &Connection,
    limits: BackupLimits,
    supported: &[u32],
) -> Result<Snapshot> {
    let format = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if !supported.contains(&format) {
        return Err(Error::Unsupported("SQLite storage format".into()));
    }
    let snapshot = collect_snapshot_for_format(c, limits, format)?;
    if format < 6 {
        rom_backup::validate_legacy_retry_epochs(&snapshot)?;
    }
    Ok(snapshot)
}

fn validate_inventory(c: &Connection, format: u32) -> Result<()> {
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if version != format {
        return Err(Error::Unsupported("SQLite storage format".into()));
    }
    let expected: &[(&str, &str, &str)] = if format == 3 {
        &[
            ("table", "effects", "effects"),
            ("table", "events", "events"),
            ("table", "receipts", "receipts"),
            ("table", "resources", "resources"),
            ("table", "rom_state", "rom_state"),
        ]
    } else if format == 7 {
        &[
            ("table", "effects", "effects"),
            ("table", "events", "events"),
            ("table", "query_keys", "query_keys"),
            ("index", "query_keys_value", "query_keys"),
            ("table", "query_kinds", "query_kinds"),
            ("table", "query_profile", "query_profile"),
            ("table", "receipts", "receipts"),
            ("table", "reference_edges", "reference_edges"),
            ("index", "reference_edges_target", "reference_edges"),
            ("table", "resources", "resources"),
            ("table", "rom_state", "rom_state"),
            ("table", "schemas", "schemas"),
        ]
    } else {
        &[
            ("table", "effects", "effects"),
            ("table", "events", "events"),
            ("table", "receipts", "receipts"),
            ("table", "reference_edges", "reference_edges"),
            ("index", "reference_edges_target", "reference_edges"),
            ("table", "resources", "resources"),
            ("table", "rom_state", "rom_state"),
            ("table", "schemas", "schemas"),
        ]
    };
    let mut statement = c
        .prepare("SELECT type,name,tbl_name FROM sqlite_master WHERE name NOT GLOB 'sqlite_*' ORDER BY name")
        .map_err(|_| Error::Storage)?;
    let mut rows = statement.query([]).map_err(|_| Error::Storage)?;
    for &(kind, name, table) in expected {
        let row = rows
            .next()
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        if (text(row, 0)?, text(row, 1)?, text(row, 2)?) != (kind, name, table) {
            return Err(Error::Storage);
        }
    }
    if rows.next().map_err(|_| Error::Storage)?.is_some() {
        return Err(Error::Storage);
    }
    let states: i64 = c
        .query_row("SELECT COUNT(*) FROM rom_state", [], |r| r.get(0))
        .map_err(|_| Error::Storage)?;
    if states != 1 {
        return Err(Error::Storage);
    }
    Ok(())
}

fn text<'a>(row: &'a rusqlite::Row<'_>, index: usize) -> Result<&'a str> {
    row.get_ref(index)
        .map_err(|_| Error::Storage)?
        .as_str()
        .map_err(|_| Error::Storage)
}

/// Read the physical graph as stored; validation must detect missing or extra edges.
fn collect_snapshot_for_format(
    c: &Connection,
    limits: BackupLimits,
    format: u32,
) -> Result<rom_backup::Snapshot> {
    collect_records(c, limits, format, true)
}

/// Explicit rebuild trusts only bounded authoritative records; derived contents are discarded.
pub(super) fn collect_rebuild_snapshot(c: &Connection, limits: BackupLimits) -> Result<Snapshot> {
    let snapshot = collect_records(c, limits, rom_backup::STORAGE_FORMAT, false)?;
    snapshot.validate()?;
    Ok(snapshot)
}

fn collect_records(
    c: &Connection,
    limits: BackupLimits,
    format: u32,
    validate_indexes: bool,
) -> Result<Snapshot> {
    validate_inventory(c, format)?;
    let mut state_stmt = c
        .prepare("SELECT data FROM rom_state WHERE id=1")
        .map_err(|_| Error::Storage)?;
    let mut state_rows = state_stmt.query([]).map_err(|_| Error::Storage)?;
    let state_row = state_rows
        .next()
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    let mut collect = Collector::new(text(state_row, 0)?, limits)?;
    for table in ["resources", "receipts", "events", "effects"] {
        let sql = match table {
            "resources" => "SELECT kind,id,revision,data FROM resources",
            "receipts" => "SELECT identity,data FROM receipts",
            "events" => "SELECT identity,data FROM events",
            _ => "SELECT identity,ordinal,data FROM effects",
        };
        let mut stmt = c.prepare(sql).map_err(|_| Error::Storage)?;
        let mut rows = stmt.query([]).map_err(|_| Error::Storage)?;
        while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
            match table {
                "resources" => collect.row(
                    text(row, 0)?,
                    text(row, 1)?,
                    Some(
                        u64::try_from(row.get::<_, i64>(2).map_err(|_| Error::Storage)?)
                            .map_err(|_| Error::Storage)?,
                    ),
                    text(row, 3)?,
                )?,
                "receipts" => collect.receipt(text(row, 0)?, text(row, 1)?)?,
                "events" => collect.event(text(row, 0)?, text(row, 1)?)?,
                _ => collect.effect(
                    text(row, 0)?,
                    u64::try_from(row.get::<_, i64>(1).map_err(|_| Error::Storage)?)
                        .map_err(|_| Error::Storage)?,
                    text(row, 2)?,
                )?,
            }
        }
    }

    if format == 3 {
        return Ok(collect.snapshot);
    }

    let mut schemas = c
        .prepare("SELECT kind,data FROM schemas ORDER BY kind")
        .map_err(|_| Error::Storage)?;
    let mut rows = schemas.query([]).map_err(|_| Error::Storage)?;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        collect.descriptor(text(row, 0)?, text(row, 1)?)?;
    }
    let mut index = c
        .prepare("PRAGMA index_info(reference_edges_target)")
        .map_err(|_| Error::Storage)?;
    let columns = index
        .query_map([], |row| row.get::<_, String>(2))
        .map_err(|_| Error::Storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::Storage)?;
    if columns != ["target_kind", "target_id", "source_kind", "source_id"] {
        return Err(Error::Storage);
    }
    let mut references = c.prepare("SELECT source_kind,source_id,target_kind,target_id FROM reference_edges ORDER BY source_kind,source_id,target_kind,target_id")
            .map_err(|_| Error::Storage)?;
    let mut rows = references.query([]).map_err(|_| Error::Storage)?;
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let parts = [text(row, 0)?, text(row, 1)?, text(row, 2)?, text(row, 3)?];
        let bytes = parts
            .iter()
            .try_fold(0usize, |n, part| n.checked_add(part.len()))
            .ok_or(Error::TooLarge)?;
        if bytes > limits.max_bytes {
            return Err(Error::TooLarge);
        }
        collect.reference(rom::ReferenceEdge {
            source: Key {
                kind: parts[0].into(),
                id: parts[1].into(),
            },
            target: Key {
                kind: parts[2].into(),
                id: parts[3].into(),
            },
        })?;
    }
    if format == rom_backup::STORAGE_FORMAT && validate_indexes {
        crate::index::validate(c, &mut collect, limits)?;
    }
    Ok(collect.snapshot)
}
