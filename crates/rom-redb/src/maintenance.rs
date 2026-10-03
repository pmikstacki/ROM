//! Host-only synchronous maintenance; run on a bounded blocking executor.
use crate::{
    Redb,
    format::*,
    references::{self, INCOMING, OUTGOING, SCHEMAS},
};
use redb::{Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata};
use rom::{Error, Result};
use rom_backup::{Backend, BackupLimits, Collector, Manifest, Snapshot, Stage};
use std::path::Path;
impl Redb {
    /// Export all logical tables from one consistent read transaction. Refuses overwrite.
    /// Contains protected data; the parent directory must be trusted.
    pub fn backup_to(&self, path: impl AsRef<Path>, limits: BackupLimits) -> Result<Manifest> {
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let snapshot = snapshot(&tx, limits)?;
        drop(tx);
        drop(_gate);
        rom_backup::write(path, Backend::Redb, &snapshot, limits)
    }
    /// Restore a validated archive into a fresh path, fencing cursors and active claims.
    pub fn restore_from(
        archive: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
    ) -> Result<Self> {
        let (_, data) = rom_backup::read(archive, Backend::Redb, limits)?;
        Self::restore_snapshot(data, destination.as_ref(), limits, || Ok(()))
    }

    pub(super) fn restore_snapshot(
        mut data: Snapshot,
        destination: &Path,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        data.state.prepare_restore()?;
        let storage_limits = data.state.storage_limits();
        let stage = Stage::new(destination)?;
        let restored =
            Self::open_with_validation_limits(stage.path(), storage_limits.clone(), limits)?;
        let mut tx = restored.db.begin_write().map_err(|_| Error::Storage)?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        for descriptor in &data.descriptors {
            tx.open_table(SCHEMAS)
                .map_err(|_| Error::Storage)?
                .insert(
                    descriptor.kind.as_str(),
                    serde_json::to_string(descriptor)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::Storage)?;
        }
        for edge in &data.references {
            tx.open_table(OUTGOING)
                .map_err(|_| Error::Storage)?
                .insert(
                    (
                        edge.source.kind.as_str(),
                        edge.source.id.as_str(),
                        edge.target.kind.as_str(),
                        edge.target.id.as_str(),
                    ),
                    1,
                )
                .map_err(|_| Error::Storage)?;
            tx.open_table(INCOMING)
                .map_err(|_| Error::Storage)?
                .insert(
                    (
                        edge.target.kind.as_str(),
                        edge.target.id.as_str(),
                        edge.source.kind.as_str(),
                        edge.source.id.as_str(),
                    ),
                    1,
                )
                .map_err(|_| Error::Storage)?;
        }
        for row in &data.rows {
            tx.open_table(ROWS)
                .map_err(|_| Error::Storage)?
                .insert(
                    (row.key.kind.as_str(), row.key.id.as_str()),
                    serde_json::to_string(row)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::Storage)?;
        }
        for receipt in &data.receipts {
            tx.open_table(RECEIPTS)
                .map_err(|_| Error::Storage)?
                .insert(
                    receipt.identity.as_str(),
                    serde_json::to_string(receipt)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::Storage)?;
        }
        for (id, row) in &data.events {
            tx.open_table(EVENTS)
                .map_err(|_| Error::Storage)?
                .insert(
                    id.as_str(),
                    serde_json::to_string(row)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::Storage)?;
        }
        for effect in &data.effects {
            tx.open_table(EFFECTS)
                .map_err(|_| Error::Storage)?
                .insert(
                    (effect.identity.as_str(), effect.ordinal),
                    serde_json::to_string(&effect.intent)
                        .map_err(|_| Error::Storage)?
                        .as_str(),
                )
                .map_err(|_| Error::Storage)?;
        }
        tx.open_table(STATE)
            .map_err(|_| Error::Storage)?
            .insert(
                "state",
                serde_json::to_string(&data.state)
                    .map_err(|_| Error::Storage)?
                    .as_str(),
            )
            .map_err(|_| Error::Storage)?;
        tx.commit().map_err(|_| Error::Unknown)?;
        // Validate the actual rebuilt tables before the destination becomes visible.
        let read = restored.db.begin_read().map_err(|_| Error::Storage)?;
        snapshot(&read, limits)?.validate()?;
        drop(read);
        drop(restored);
        stage.publish_with(before_publish)?;
        Self::open_with_validation_limits(destination, storage_limits, limits)
            .map_err(|_| Error::Unknown)
    }
}

/// Collect one complete bounded logical snapshot for open, backup and integrity validation.
pub(super) fn snapshot(tx: &redb::ReadTransaction, limits: BackupLimits) -> Result<Snapshot> {
    snapshot_in_format(tx, limits, NativeFormat::Current)
}

pub(super) enum NativeFormat {
    Upgrade,
    Migration,
    Current,
}

/// Both formats share the same bounded row, receipt, event, effect and state decoder.
pub(super) fn snapshot_in_format(
    tx: &redb::ReadTransaction,
    limits: BackupLimits,
    format: NativeFormat,
) -> Result<Snapshot> {
    let marker = tx.open_table(META).map_err(|_| Error::Storage)?;
    let version = marker
        .get("format")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?
        .value();
    let supported = match format {
        NativeFormat::Upgrade => matches!(version, 3..=5),
        NativeFormat::Migration => matches!(version, 4 | 5) || version == FORMAT,
        NativeFormat::Current => version == FORMAT,
    };
    if !supported {
        return Err(Error::Unsupported("redb storage format".into()));
    }
    let table_count = if version == 3 { 6 } else { 9 };
    if marker.len().map_err(|_| Error::Storage)? != 1
        || tx.list_tables().map_err(|_| Error::Storage)?.count() != table_count
        || tx
            .list_multimap_tables()
            .map_err(|_| Error::Storage)?
            .next()
            .is_some()
    {
        return Err(Error::Storage);
    }
    let state = tx.open_table(STATE).map_err(|_| Error::Storage)?;
    if state.len().map_err(|_| Error::Storage)? != 1 {
        return Err(Error::Storage);
    }
    let value = state
        .get("state")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    let mut collect = Collector::new(value.value(), limits)?;
    for entry in tx
        .open_table(ROWS)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        collect.row(key.value().0, key.value().1, None, value.value())?;
    }
    for (table, receipt) in [(RECEIPTS, true), (EVENTS, false)] {
        for entry in tx
            .open_table(table)
            .map_err(|_| Error::Storage)?
            .iter()
            .map_err(|_| Error::Storage)?
        {
            let (key, value) = entry.map_err(|_| Error::Storage)?;
            if receipt {
                collect.receipt(key.value(), value.value())?;
            } else {
                collect.event(key.value(), value.value())?;
            }
        }
    }
    for entry in tx
        .open_table(EFFECTS)
        .map_err(|_| Error::Storage)?
        .iter()
        .map_err(|_| Error::Storage)?
    {
        let (key, value) = entry.map_err(|_| Error::Storage)?;
        collect.effect(key.value().0, key.value().1, value.value())?;
    }
    if version != 3 {
        references::collect(tx, &mut collect)?;
    }
    drop(value);
    drop(state);
    drop(marker);

    if version < FORMAT {
        rom_backup::validate_legacy_retry_epochs(&collect.snapshot)?;
    }
    // Collector accounts for work records and all subsequently collected records.
    Ok(collect.snapshot)
}

/// Collect an offline source without writes; dirty native state is recovered in a private copy.
pub(super) fn read_snapshot(
    source: &Path,
    limits: BackupLimits,
    format: NativeFormat,
) -> Result<Snapshot> {
    crate::preflight::inspect(source, |tx| snapshot_in_format(tx, limits, format))
}

/// Upgrade either legacy layout while retaining the distinction between unbound and persisted catalogs.
pub(super) fn read_upgrade_snapshot(
    source: &Path,
    limits: BackupLimits,
    descriptors: &[rom::Descriptor],
) -> Result<Snapshot> {
    crate::preflight::inspect(source, |tx| {
        let snapshot = snapshot_in_format(tx, limits, NativeFormat::Upgrade)?;
        let marker = tx.open_table(META).map_err(|_| Error::Storage)?;
        let version = marker
            .get("format")
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?
            .value();
        if version == 3 {
            rom_backup::bind_legacy_schema(snapshot, descriptors, limits)
        } else {
            rom_backup::upgrade_legacy_snapshot(snapshot, descriptors, limits)
        }
    })
}
