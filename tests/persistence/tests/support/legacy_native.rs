//! Offline fixtures with an actual pre-format-8 state and native inventory.
#[path = "legacy_wire.rs"]
mod legacy_wire;
use redb::ReadableTable;
use std::path::Path;

pub fn mark(redb: bool, path: &Path, version: u32, erase_catalog: bool) {
    assert!(matches!(version, 4..=7));
    if redb {
        let db = redb::Database::open(path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<&str, u64>::new("rom_metadata"))
            .unwrap()
            .insert("format", u64::from(version))
            .unwrap();
        let state_table = redb::TableDefinition::<&str, &str>::new("rom_state");
        let state = {
            let table = tx.open_table(state_table).unwrap();
            let raw = table.get("state").unwrap().unwrap();
            legacy_wire::state(serde_json::from_str(raw.value()).unwrap())
        };
        tx.open_table(state_table)
            .unwrap()
            .insert("state", state.as_str())
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
        if version < 7 {
            connection
                .execute_batch(
                    "DROP TABLE query_keys; DROP TABLE query_kinds; DROP TABLE query_profile;",
                )
                .unwrap();
        }
        let raw: String = connection
            .query_row("SELECT data FROM rom_state WHERE id=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        connection
            .execute(
                "UPDATE rom_state SET data=? WHERE id=1",
                [legacy_wire::state(serde_json::from_str(&raw).unwrap())],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        if erase_catalog {
            connection.execute("DELETE FROM schemas", []).unwrap();
        }
    }
}
