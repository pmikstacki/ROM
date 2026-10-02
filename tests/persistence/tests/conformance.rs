use rom::Error;
use rom_sqlite::Sqlite;
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
