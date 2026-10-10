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
        if self.native_format != FORMAT {
            return Err(Error::Unsupported(
                "unreleased native journal archive".into(),
            ));
        }
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        let snapshot =
            snapshot_with_limits(&tx, limits, self.validation_limits, NativeFormat::Current)?;
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
        let destination = rom_backup::NativeOwnership::acquire(
            destination.as_ref(),
            rom_backup::NativeAccess::Fresh,
        )?;
        Self::restore_snapshot(data, destination, limits, || Ok(()))
    }

    pub(super) fn restore_snapshot(
        data: Snapshot,
        destination: rom_backup::NativeOwnership,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        Self::restore_snapshot_in_format(data, destination, limits, before_publish, FORMAT)
    }

    pub(super) fn restore_snapshot_in_format(
        mut data: Snapshot,
        destination: rom_backup::NativeOwnership,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
        native_format: u64,
    ) -> Result<Self> {
        data.state.prepare_restore()?;
        let storage_limits = data.state.storage_limits();
        let stage = Stage::new(destination.path())?;
        let restored = Self::open_owned_in_format(
            rom_backup::NativeOwnership::acquire(
                stage.path(),
                rom_backup::NativeAccess::OpenOrCreate,
            )?,
            storage_limits.clone(),
            limits,
            native_format,
        )?;
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
        if native_format == JOURNAL_FORMAT {
            crate::native_journal::import(&tx, &data.state)?;
        } else {
            crate::native_state::write(&tx, &data.state)?;
        }
        tx.commit().map_err(|_| Error::Unknown)?;
        // Validate the actual rebuilt tables before the destination becomes visible.
        let read = restored.db.begin_read().map_err(|_| Error::Storage)?;
        snapshot_in_format(&read, limits, NativeFormat::Exact(native_format))?.validate()?;
        drop(read);
        drop(restored);
        stage.publish_with(before_publish)?;
        stage.finish_native_publication()?;
        Self::open_owned_in_format(destination, storage_limits, limits, native_format)
            .map_err(|_| Error::Unknown)
    }
}

/// Collect one complete bounded logical snapshot for open, backup and integrity validation.
#[cfg(test)]
pub(super) fn snapshot(tx: &redb::ReadTransaction, limits: BackupLimits) -> Result<Snapshot> {
    snapshot_in_format(tx, limits, NativeFormat::Current)
}

pub(super) enum NativeFormat {
    Upgrade,
    Migration,
    Current,
    Exact(u64),
    #[cfg(test)]
    PredecessorTen,
}

/// Both formats share the same bounded row, receipt, event, effect and state decoder.
pub(super) fn snapshot_in_format(
    tx: &redb::ReadTransaction,
    limits: BackupLimits,
    format: NativeFormat,
) -> Result<Snapshot> {
    snapshot_with_limits(tx, limits, limits, format)
}

/// Logical collection and configured full-native admission are independent bounds.
pub(super) fn snapshot_with_limits(
    tx: &redb::ReadTransaction,
    limits: BackupLimits,
    raw_limits: BackupLimits,
    format: NativeFormat,
) -> Result<Snapshot> {
    let marker = tx.open_table(META).map_err(|_| Error::Storage)?;
    let version = marker
        .get("format")
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?
        .value();
    let supported = match format {
        NativeFormat::Upgrade => {
            matches!(version, 3..=9) || (FORMAT == JOURNAL_FORMAT && version == 10)
        }
        NativeFormat::Migration => matches!(version, 4..=10) || version == FORMAT,
        NativeFormat::Current => version == FORMAT,
        NativeFormat::Exact(expected) => {
            version == expected && matches!(version, 10 | JOURNAL_FORMAT)
        }
        #[cfg(test)]
        NativeFormat::PredecessorTen => version == 10,
    };
    if !supported {
        return Err(Error::Unsupported("redb storage format".into()));
    }
    let table_count = if version == 3 {
        6
    } else if version == JOURNAL_FORMAT {
        13
    } else if version == 10 {
        12
    } else {
        9
    };
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
    crate::admission::check(tx, version, raw_limits)?;
    let state = tx.open_table(STATE).map_err(|_| Error::Storage)?;
    let mut collect = if version == JOURNAL_FORMAT {
        let reader = crate::native_journal::Reader::new(
            tx.open_table(STATE).map_err(|_| Error::Storage)?,
            tx.open_table(POSITIONS).map_err(|_| Error::Storage)?,
            tx.open_table(EVENTS).map_err(|_| Error::Storage)?,
            tx.open_table(WORK).map_err(|_| Error::Storage)?,
            tx.open_table(ROOTS).map_err(|_| Error::Storage)?,
            tx.open_table(ACTIVE).map_err(|_| Error::Storage)?,
            raw_limits,
        );
        let (canonical, _) = crate::native_journal::collect(&reader, raw_limits)?;
        let raw = serde_json::to_string(&canonical).map_err(|_| Error::Storage)?;
        Collector::new(&raw, limits)?
    } else if version == 10 {
        let records = tx.open_table(WORK).map_err(|_| Error::Storage)?;
        let roots = tx.open_table(ROOTS).map_err(|_| Error::Storage)?;
        let active = tx.open_table(ACTIVE).map_err(|_| Error::Storage)?;
        let (canonical, physical) =
            crate::native_state::read(&state, &records, &roots, &active, raw_limits)?;
        let raw = serde_json::to_string(&canonical).map_err(|_| Error::Storage)?;
        let mut collect = Collector::new(&raw, limits)?;
        for size in physical {
            collect.physical(size)?;
        }
        collect
    } else {
        if state.len().map_err(|_| Error::Storage)? != 1 {
            return Err(Error::Storage);
        }
        let value = state
            .get("state")
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        if version < 8 {
            Collector::legacy(value.value(), limits)?
        } else if version == 8 {
            Collector::previous(value.value(), limits)?
        } else {
            Collector::new(value.value(), limits)?
        }
    };
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
    drop(state);
    drop(marker);

    if version < 6 {
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
        } else if version < 6 {
            rom_backup::upgrade_legacy_snapshot(snapshot, descriptors, limits)
        } else {
            rom_backup::upgrade_current_snapshot(snapshot, descriptors, limits)
        }
    })
}
