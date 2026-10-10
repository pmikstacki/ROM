//! Fresh authoritative keyed reads across reusable statement execution.
use crate::Sqlite;
use rom::{Error, Key, Row, Storage};
use rusqlite::{Connection, params};

fn key(id: &str) -> Key {
    Key {
        kind: "cached-read-fixture".into(),
        id: id.into(),
    }
}
fn row(id: &str, revision: u64) -> Row {
    Row {
        key: key(id),
        revision,
        value: Some(rom::json!({"revision":revision})),
        protected: Default::default(),
    }
}
fn fixture(candidate: bool) -> Sqlite {
    if candidate {
        crate::native_work_test_support::fixture()
    } else {
        crate::native_work_test_support::predecessor_fixture()
    }
}
fn write(c: &Connection, row: &Row) {
    c.execute(
        "INSERT OR REPLACE INTO resources(kind,id,revision,data) VALUES (?1,?2,?3,?4)",
        params![
            row.key.kind,
            row.key.id,
            i64::try_from(row.revision).unwrap(),
            serde_json::to_string(row).unwrap()
        ],
    )
    .unwrap();
}
fn read_at(c: &Connection, candidate: bool, id: &str) -> rom::Result<Option<Row>> {
    if candidate {
        Ok(
            crate::native_work::Reader::bounded(c, rom_backup::BackupLimits::default())
                .resource(&key(id))?
                .map(|(r, _)| r),
        )
    } else {
        crate::persistence::row(c, &key(id))
    }
}

#[test]
fn cached_read_alternating_bindings_and_missing_are_fresh() {
    for candidate in [false, true] {
        let db = fixture(candidate);
        {
            let c = db.connection.lock().unwrap();
            write(&c, &row("a", 1));
            write(&c, &row("b", 2));
        }
        for id in ["a", "b", "missing", "a", "missing", "b"] {
            let expected = match id {
                "a" => Some(row("a", 1)),
                "b" => Some(row("b", 2)),
                _ => None,
            };
            assert_eq!(db.load(&key(id)).unwrap(), expected);
        }
        {
            let c = db.connection.lock().unwrap();
            write(&c, &row("missing", 3));
        }
        assert_eq!(db.load(&key("missing")).unwrap(), Some(row("missing", 3)));
    }
}
#[test]
fn cached_read_update_tombstone_and_physical_delete_are_fresh() {
    for candidate in [false, true] {
        let db = fixture(candidate);
        {
            write(&db.connection.lock().unwrap(), &row("a", 1));
        }
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 1)));
        {
            write(&db.connection.lock().unwrap(), &row("a", 2));
        }
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 2)));
        let mut deleted = row("a", 3);
        deleted.value = None;
        {
            write(&db.connection.lock().unwrap(), &deleted);
        }
        assert_eq!(db.load(&key("a")).unwrap(), Some(deleted));
        db.connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM resources WHERE kind=?1 AND id=?2",
                params![key("a").kind, "a"],
            )
            .unwrap();
        assert_eq!(db.load(&key("a")).unwrap(), None);
    }
}
#[test]
fn cached_read_corruption_error_does_not_poison_next_execution() {
    for candidate in [false, true] {
        let db = fixture(candidate);
        {
            write(&db.connection.lock().unwrap(), &row("a", 1));
            write(&db.connection.lock().unwrap(), &row("b", 2));
        }
        assert!(db.load(&key("a")).unwrap().is_some());
        for raw in ["{", "not-json"] {
            db.connection
                .lock()
                .unwrap()
                .execute(
                    "UPDATE resources SET data=?1 WHERE kind=?2 AND id=?3",
                    params![raw, key("a").kind, "a"],
                )
                .unwrap();
            assert_eq!(db.load(&key("a")), Err(Error::Storage));
            assert_eq!(db.load(&key("b")).unwrap(), Some(row("b", 2)));
            write(&db.connection.lock().unwrap(), &row("a", 3));
            assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 3)));
        }
        db.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE resources SET data=?1 WHERE kind=?2 AND id=?3",
                params![vec![0xffu8], key("a").kind, "a"],
            )
            .unwrap();
        assert_eq!(db.load(&key("a")), Err(Error::Storage));
        write(&db.connection.lock().unwrap(), &row("a", 4));
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 4)));
    }
}
#[test]
fn cached_read_transaction_rollback_and_commit_remain_authoritative() {
    for candidate in [false, true] {
        let db = fixture(candidate);
        let mut c = db.connection.lock().unwrap();
        write(&c, &row("a", 1));
        assert_eq!(read_at(&c, candidate, "a").unwrap(), Some(row("a", 1)));
        {
            let tx = c.transaction().unwrap();
            write(&tx, &row("a", 2));
            assert_eq!(read_at(&tx, candidate, "a").unwrap(), Some(row("a", 2)));
            tx.rollback().unwrap();
        }
        assert_eq!(read_at(&c, candidate, "a").unwrap(), Some(row("a", 1)));
        {
            let tx = c.transaction().unwrap();
            write(&tx, &row("a", 3));
            assert_eq!(read_at(&tx, candidate, "a").unwrap(), Some(row("a", 3)));
            tx.commit().unwrap();
        }
        assert_eq!(read_at(&c, candidate, "a").unwrap(), Some(row("a", 3)));
    }
}
#[test]
fn cached_read_schema_change_reprepares_and_missing_table_is_error() {
    for candidate in [false, true] {
        let db = fixture(candidate);
        write(&db.connection.lock().unwrap(), &row("a", 1));
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 1)));
        db.connection
            .lock()
            .unwrap()
            .execute_batch("CREATE INDEX fixture_revision ON resources(revision);")
            .unwrap();
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 1)));
        db.connection
            .lock()
            .unwrap()
            .execute_batch("DROP INDEX fixture_revision;")
            .unwrap();
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 1)));
        db.connection
            .lock()
            .unwrap()
            .execute_batch("ALTER TABLE resources RENAME TO hidden_resources;")
            .unwrap();
        assert_eq!(db.load(&key("a")), Err(Error::Storage));
        db.connection
            .lock()
            .unwrap()
            .execute_batch("ALTER TABLE hidden_resources RENAME TO resources;")
            .unwrap();
        assert_eq!(db.load(&key("a")).unwrap(), Some(row("a", 1)));
    }
}
#[test]
fn cached_read_candidate_still_charges_bytes_before_decode() {
    let db = fixture(true);
    let c = db.connection.lock().unwrap();
    write(&c, &row("a", 1));
    assert_eq!(read_at(&c, true, "a").unwrap(), Some(row("a", 1)));
    let limits = rom_backup::BackupLimits {
        max_bytes: 1,
        ..Default::default()
    };
    assert_eq!(
        crate::native_work::Reader::bounded(&c, limits).resource(&key("a")),
        Err(Error::TooLarge)
    );
    assert_eq!(read_at(&c, true, "a").unwrap(), Some(row("a", 1)));
}
