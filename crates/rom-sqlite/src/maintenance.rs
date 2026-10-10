//! Host-only synchronous maintenance; run on a bounded blocking executor.
use crate::{Sqlite, persistence::save_state, snapshot::collect_snapshot};
use rom::{Error, Result};
use rom_backup::{Backend, BackupLimits, Manifest, Snapshot, Stage};
use rusqlite::params;
use std::path::Path;
impl Sqlite {
    /// Export one coherent native read transaction, including committed WAL contents.
    /// Contains protected data. Parent directory must be trusted; refuses overwrite.
    pub fn backup_to(&self, path: impl AsRef<Path>, limits: BackupLimits) -> Result<Manifest> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c.transaction().map_err(|_| Error::Storage)?;
        let snapshot = collect_snapshot(&tx, limits)?;
        snapshot.validate()?;
        // Release the native snapshot before filesystem publication.
        drop(tx);
        drop(c);
        rom_backup::write(path, Backend::Sqlite, &snapshot, limits)
    }
    /// Restore only into a fresh path. Cursors and in-flight work claims are fenced.
    /// External blob objects, callback code and delivery effects require separate recovery.
    pub fn restore_from(
        archive: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
    ) -> Result<Self> {
        let (_, snapshot) = rom_backup::read(archive, Backend::Sqlite, limits)?;
        let destination = rom_backup::NativeOwnership::acquire(
            destination.as_ref(),
            rom_backup::NativeAccess::Fresh,
        )?;
        Self::restore_snapshot(snapshot, destination, limits, || Ok(()))
    }

    pub(super) fn restore_snapshot(
        mut snapshot: Snapshot,
        destination: rom_backup::NativeOwnership,
        limits: BackupLimits,
        before_publish: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        snapshot.validate()?;
        snapshot.state.prepare_restore()?;
        let storage_limits = snapshot.state.storage_limits();
        let stage = Stage::new(destination.path())?;
        let restored =
            Self::open_with_validation_limits(stage.path(), storage_limits.clone(), limits)?;
        {
            let mut c = restored.connection.lock().map_err(|_| Error::Panicked)?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| Error::Storage)?;
            for descriptor in &snapshot.descriptors {
                tx.execute(
                    "INSERT INTO schemas(kind,data) VALUES (?,?)",
                    params![
                        descriptor.kind,
                        serde_json::to_string(descriptor).map_err(|_| Error::Storage)?
                    ],
                )
                .map_err(|_| Error::Storage)?;
            }
            for row in &snapshot.rows {
                tx.execute(
                    "INSERT INTO resources VALUES (?,?,?,?)",
                    params![
                        row.key.kind,
                        row.key.id,
                        i64::try_from(row.revision).map_err(|_| Error::Storage)?,
                        serde_json::to_string(row).map_err(|_| Error::Storage)?
                    ],
                )
                .map_err(|_| Error::Storage)?;
            }
            for receipt in &snapshot.receipts {
                tx.execute(
                    "INSERT INTO receipts VALUES (?,?)",
                    params![
                        receipt.identity,
                        serde_json::to_string(receipt).map_err(|_| Error::Storage)?
                    ],
                )
                .map_err(|_| Error::Storage)?;
            }
            for (id, row) in &snapshot.events {
                tx.execute(
                    "INSERT INTO events VALUES (?,?)",
                    params![id, serde_json::to_string(row).map_err(|_| Error::Storage)?],
                )
                .map_err(|_| Error::Storage)?;
            }
            for effect in &snapshot.effects {
                tx.execute(
                    "INSERT INTO effects VALUES (?,?,?)",
                    params![
                        effect.identity,
                        i64::try_from(effect.ordinal).map_err(|_| Error::Storage)?,
                        serde_json::to_string(&effect.intent).map_err(|_| Error::Storage)?
                    ],
                )
                .map_err(|_| Error::Storage)?;
            }
            for edge in &snapshot.references {
                tx.execute(
                    "INSERT INTO reference_edges(source_kind,source_id,target_kind,target_id) VALUES (?,?,?,?)",
                    params![edge.source.kind, edge.source.id, edge.target.kind, edge.target.id],
                ).map_err(|_| Error::Storage)?;
            }
            save_state(&tx, &snapshot.state)?;
            crate::index::rebuild(&tx, &snapshot, limits)?;
            tx.commit().map_err(|_| Error::Unknown)?;
        }
        {
            let c = restored.connection.lock().map_err(|_| Error::Panicked)?;
            // Reject malformed reconstruction or exceeded native read limits before publication.
            crate::snapshot::collect_snapshot(&c, limits)?.validate()?;
            let busy: i64 = c
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
                .map_err(|_| Error::Storage)?;
            if busy != 0 {
                return Err(Error::Storage);
            }
            let mode: String = c
                .query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))
                .map_err(|_| Error::Storage)?;
            if mode != "delete" {
                return Err(Error::Storage);
            }
        }
        restored.close()?;
        stage.publish_with(before_publish)?;
        stage.finish_native_publication()?;
        Self::open_owned(destination, storage_limits, limits).map_err(|_| Error::Unknown)
    }
}
