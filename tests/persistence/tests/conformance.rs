use rom::Error;
use rom_sqlite::Sqlite;

#[test]
fn sqlite_rejects_unversioned_view_only_database() {
    let path = std::env::temp_dir().join(format!("rom-view-only-{}.sqlite", std::process::id()));
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE VIEW existing_view AS SELECT 1 AS value;")
            .unwrap();
    }
    let result = Sqlite::open(&path);
    let rejected = matches!(result, Err(Error::Unsupported(_)));
    drop(result);
    let c = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    drop(c);
    std::fs::remove_file(path).unwrap();
    assert!(rejected, "a view is existing schema, not an empty database");
    assert_eq!(
        count, 1,
        "rejection must preserve the original schema alone"
    );
    assert_eq!(version, 0, "rejection must not install a ROM format marker");
}

#[test]
fn redb_rejects_unversioned_multimap_only_database() {
    use redb::{MultimapTableDefinition, ReadableDatabase};
    const EXISTING: MultimapTableDefinition<&str, &str> =
        MultimapTableDefinition::new("existing_multimap");
    let path = std::env::temp_dir().join(format!("rom-multimap-only-{}.redb", std::process::id()));
    {
        let db = redb::Database::create(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_multimap_table(EXISTING)
            .unwrap()
            .insert("key", "value")
            .unwrap();
        tx.commit().unwrap();
    }
    let result = rom_redb::Redb::open(&path);
    let rejected = matches!(result, Err(Error::Unsupported(_)));
    drop(result);
    let db = redb::Database::open(&path).unwrap();
    let tx = db.begin_read().unwrap();
    let tables = tx.list_tables().unwrap().count();
    let table = tx.open_multimap_table(EXISTING).unwrap();
    let value = table
        .get("key")
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .value()
        .to_owned();
    drop(table);
    drop(tx);
    drop(db);
    std::fs::remove_file(path).unwrap();
    assert!(
        rejected,
        "a multimap is an existing table, not an empty database"
    );
    assert_eq!(tables, 0, "rejection must not create ROM tables");
    assert_eq!(value, "value");
}

#[test]
fn sqlite_rejects_unversioned_user_table_with_internal_like_prefix() {
    let path =
        std::env::temp_dir().join(format!("rom-prefix-schema-{}.sqlite", std::process::id()));
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(
            "CREATE TABLE sqliteXlegacy(value TEXT); INSERT INTO sqliteXlegacy VALUES ('keep');",
        )
        .unwrap();
    }
    let result = Sqlite::open(&path);
    let rejected = matches!(result, Err(Error::Unsupported(_)));
    drop(result);
    let c = rusqlite::Connection::open(&path).unwrap();
    let value: String = c
        .query_row("SELECT value FROM sqliteXlegacy", [], |r| r.get(0))
        .unwrap();
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    drop(c);
    std::fs::remove_file(path).unwrap();
    assert!(
        rejected,
        "LIKE underscore wildcard must not hide user schema"
    );
    assert_eq!(value, "keep");
    assert_eq!(version, 0);
}

#[test]
fn sqlite_rejects_future_format_before_schema_writes() {
    let path =
        std::env::temp_dir().join(format!("rom-future-format-{}.sqlite", std::process::id()));
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", 999).unwrap();
    }
    let result = Sqlite::open(&path);
    let rejected = matches!(result, Err(Error::Unsupported(_)));
    drop(result);
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        let tables: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0, "rejected format must not acquire ROM tables");
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            999
        );
    }
    std::fs::remove_file(path).unwrap();
    assert!(
        rejected,
        "a future format must be rejected before creating ROM tables"
    );
}

#[test]
fn redb_atomic_bundle_available_through_storage() {
    use rom::{Bundle, Intent, Key, Receipt, Row, Storage, json};
    let path = std::env::temp_dir().join(format!("rom-redb-initial-{}.redb", std::process::id()));
    let store = rom_redb::Redb::open(&path).expect("redb must implement Storage");
    let receipt = Receipt {
        identity: "one".into(),
        fingerprint: "create-one".into(),
        row: Row {
            key: Key {
                kind: "records".into(),
                id: "one".into(),
            },
            revision: 1,
            value: Some(json!({"done":false})),
        },
    };
    let bundle = Bundle {
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
        expected: None,
        receipt: receipt.clone(),
        changed: true,
        effects: vec![Intent::new("notify", json!({"id":"one"}))],
    };
    assert_eq!(store.commit(&bundle).unwrap(), receipt);
    assert_eq!(
        store.load(&receipt.row.key).unwrap(),
        Some(receipt.row.clone())
    );
    assert_eq!(store.receipt("one").unwrap(), Some(receipt));
    drop(store);
    std::fs::remove_file(path).unwrap();
}
