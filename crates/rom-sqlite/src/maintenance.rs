//! Host-only synchronous maintenance; run on a bounded blocking executor.
use super::*;
use rom_backup::{Backend, BackupLimits, Collector, Manifest, Stage};
impl Sqlite {
    /// Export one coherent native read transaction, including committed WAL contents.
    /// Contains protected data. Parent directory must be trusted; refuses overwrite.
    pub fn backup_to(&self, path: impl AsRef<Path>, limits: BackupLimits) -> Result<Manifest> {
        let mut c = self.connection.lock().map_err(|_| Error::Panicked)?;
        let tx = c.transaction().map_err(|_| Error::Storage)?;
        let version: u32 = tx
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| Error::Storage)?;
        let objects: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name NOT GLOB 'sqlite_*'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| Error::Storage)?;
        let states: i64 = tx
            .query_row("SELECT COUNT(*) FROM rom_state", [], |r| r.get(0))
            .map_err(|_| Error::Storage)?;
        if version != 3 || objects != 5 || states != 1 {
            return Err(Error::Storage);
        }
        let mut state_stmt = tx
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
            let mut stmt = tx.prepare(sql).map_err(|_| Error::Storage)?;
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
        // Release the native snapshot before filesystem publication.
        drop(state_rows);
        drop(state_stmt);
        drop(tx);
        drop(c);
        rom_backup::write(path, Backend::Sqlite, &collect.snapshot, limits)
    }
    /// Restore only into a fresh path. Cursors and in-flight work claims are fenced.
    /// External blob objects, callback code and delivery effects require separate recovery.
    pub fn restore_from(
        archive: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        limits: BackupLimits,
    ) -> Result<Self> {
        let (_, mut snapshot) = rom_backup::read(archive, Backend::Sqlite, limits)?;
        snapshot.state.prepare_restore()?;
        let storage_limits = snapshot.state.storage_limits();
        let stage = Stage::new(destination.as_ref())?;
        let restored = Self::open_with_limits(stage.path(), storage_limits.clone())?;
        {
            let mut c = restored.connection.lock().map_err(|_| Error::Panicked)?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| Error::Storage)?;
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
            save_state(&tx, &snapshot.state)?;
            tx.commit().map_err(|_| Error::Unknown)?;
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
        restored
            .connection
            .into_inner()
            .map_err(|_| Error::Panicked)?
            .close()
            .map_err(|_| Error::Storage)?;
        stage.publish()?;
        Self::open_with_limits(destination, storage_limits).map_err(|_| Error::Unknown)
    }
}
fn text<'a>(row: &'a rusqlite::Row<'_>, index: usize) -> Result<&'a str> {
    row.get_ref(index)
        .map_err(|_| Error::Storage)?
        .as_str()
        .map_err(|_| Error::Storage)
}
