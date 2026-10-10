//! Offline fixtures with the historical state shape and native inventory.
#[path = "legacy_wire.rs"]
mod legacy_wire;
#[path = "native_canonical.rs"]
pub mod native_canonical;
use redb::ReadableDatabase;
use std::path::Path;

pub fn mark(redb: bool, path: &Path, version: u32, erase_catalog: bool) {
    assert!(matches!(version, 4..=8));
    let canonical = canonical(redb, path);
    let state = if version < 8 {
        legacy_wire::state(canonical)
    } else {
        serde_json::to_string(&canonical).unwrap()
    };
    let _owner =
        rom_backup::NativeOwnership::acquire(path, rom_backup::NativeAccess::Existing).unwrap();
    if redb {
        let db = redb::Database::open(path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<&str, u64>::new("rom_metadata"))
            .unwrap()
            .insert("format", u64::from(version))
            .unwrap();
        let state_table = redb::TableDefinition::<&str, &str>::new("rom_state");
        {
            let mut table = tx.open_table(state_table).unwrap();
            table.retain(|_, _| false).unwrap();
            table.insert("state", state.as_str()).unwrap();
        }
        for name in ["work_records", "work_roots"] {
            tx.delete_table(redb::TableDefinition::<&str, &str>::new(name))
                .unwrap();
        }
        tx.delete_table(redb::TableDefinition::<&str, u8>::new("work_active"))
            .unwrap();
        tx.delete_table(redb::TableDefinition::<u64, &str>::new("journal_positions"))
            .unwrap();
        if erase_catalog {
            tx.open_table(redb::TableDefinition::<&str, &str>::new("rom_schemas"))
                .unwrap()
                .retain(|_, _| false)
                .unwrap();
        }
        tx.commit().unwrap();
    } else {
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS work_header;
DROP TABLE IF EXISTS operator_state;
DROP TABLE IF EXISTS work_records;
DROP TABLE IF EXISTS work_roots;
DROP TABLE IF EXISTS work_active;
DROP TABLE IF EXISTS journal_positions;",
            )
            .unwrap();
        if version < 7 {
            connection
                .execute_batch(
                    "DROP TABLE query_keys; DROP TABLE query_kinds; DROP TABLE query_profile;",
                )
                .unwrap();
        }
        connection
            .execute("UPDATE rom_state SET data=? WHERE id=1", [state])
            .unwrap();
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        if erase_catalog {
            connection.execute("DELETE FROM schemas", []).unwrap();
        }
    }
}

fn canonical(redb: bool, path: &Path) -> serde_json::Value {
    let raw = if redb {
        let db = redb::Database::open(path).unwrap();
        let tx = db.begin_read().unwrap();
        let table = tx
            .open_table(redb::TableDefinition::<&str, &str>::new("rom_state"))
            .unwrap();
        let legacy = table.get("state").unwrap();
        match legacy {
            Some(raw) => raw.value().to_owned(),
            None => table.get("metadata").unwrap().unwrap().value().to_owned(),
        }
    } else {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row("SELECT data FROM rom_state WHERE id=1", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap()
    };
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    if value.get("work").is_some() {
        value
    } else {
        let limits = if value.get("journal_records").is_some() {
            let parts: rom::storage_support::metadata::MetadataHeaderParts =
                serde_json::from_str(&raw).unwrap();
            rom::storage_support::metadata::MetadataHeader::from_parts(parts)
                .unwrap()
                .parts()
                .limits
        } else {
            let metadata: rom::storage_support::metadata::StorageMetadata =
                serde_json::from_str(&raw).unwrap();
            metadata.storage_limits()
        };
        native_canonical::state(redb, path, limits)
    }
}
