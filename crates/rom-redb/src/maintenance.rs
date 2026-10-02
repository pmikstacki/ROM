//! Host-only synchronous maintenance; run on a bounded blocking executor.
use super::*;
use rom_backup::{Backend, BackupLimits, Collector, Manifest, Stage};
impl Redb {
    /// Export all logical tables from one consistent read transaction. Refuses overwrite.
    /// Contains protected data; the parent directory must be trusted.
    pub fn backup_to(&self, path: impl AsRef<Path>, limits: BackupLimits) -> Result<Manifest> {
        let _gate = self.commit_gate.lock().map_err(|_| Error::Panicked)?;
        self.available()?;
        let tx = self.db.begin_read().map_err(|_| Error::Storage)?;
        if tx.list_tables().map_err(|_| Error::Storage)?.count() != 6
            || tx
                .list_multimap_tables()
                .map_err(|_| Error::Storage)?
                .next()
                .is_some()
        {
            return Err(Error::Storage);
        }
        let marker = tx.open_table(META).map_err(|_| Error::Storage)?;
        if marker.len().map_err(|_| Error::Storage)? != 1
            || marker
                .get("format")
                .map_err(|_| Error::Storage)?
                .map(|v| v.value())
                != Some(FORMAT)
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
        drop(value);
        drop(state);
        drop(marker);
        drop(tx);
        drop(_gate);
        rom_backup::write(path, Backend::Redb, &collect.snapshot, limits)
    }
    /// Restore a validated archive into a fresh path, fencing cursors and active claims.
    pub fn restore_from(
        archive: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
    ) -> Result<Self> {
        let (_, mut snapshot) = rom_backup::read(archive, Backend::Redb, limits)?;
        snapshot.state.prepare_restore()?;
        let storage_limits = snapshot.state.storage_limits();
        let stage = Stage::new(destination.as_ref())?;
        let restored = Self::open_with_limits(stage.path(), storage_limits.clone())?;
        let mut tx = restored.db.begin_write().map_err(|_| Error::Storage)?;
        tx.set_durability(Durability::Immediate)
            .map_err(|_| Error::Storage)?;
        for row in &snapshot.rows {
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
        for receipt in &snapshot.receipts {
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
        for (id, row) in &snapshot.events {
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
        for effect in &snapshot.effects {
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
                serde_json::to_string(&snapshot.state)
                    .map_err(|_| Error::Storage)?
                    .as_str(),
            )
            .map_err(|_| Error::Storage)?;
        tx.commit().map_err(|_| Error::Unknown)?;
        drop(restored);
        stage.publish()?;
        Self::open_with_limits(destination, storage_limits).map_err(|_| Error::Unknown)
    }
}
