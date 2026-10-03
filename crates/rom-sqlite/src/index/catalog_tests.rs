use super::{catalog, validation};
use rom::{
    Descriptor, Error, FieldDescriptor, Key, ProtectedMetadata, Row, Shape, StorageState, json,
};
use rom_backup::{BackupLimits, Collector};
use rusqlite::{Connection, params};

fn descriptor(kind: &str) -> Descriptor {
    Descriptor {
        kind: kind.into(),
        version: 1,
        fields: vec![
            FieldDescriptor {
                name: "amount".into(),
                shape: Shape::U64,
            },
            FieldDescriptor {
                name: "note".into(),
                shape: Shape::Optional(Box::new(Shape::Nullable(Box::new(Shape::String)))),
            },
            FieldDescriptor {
                name: "payload".into(),
                shape: Shape::List(Box::new(Shape::String)),
            },
        ],
    }
}

fn connection() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE schemas(kind TEXT PRIMARY KEY, data TEXT NOT NULL);
        CREATE TABLE resources(kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));").unwrap();
    catalog::initialize(&c).unwrap();
    c
}

fn register(c: &Connection, kind: &str) {
    let d = descriptor(kind).canonical().unwrap();
    c.execute(
        "INSERT INTO schemas VALUES (?,?)",
        params![kind, serde_json::to_string(&d).unwrap()],
    )
    .unwrap();
    catalog::register(c, &[d]).unwrap();
}

fn row(kind: &str) -> Row {
    Row {
        key: Key {
            kind: kind.into(),
            id: "a".into(),
        },
        revision: 1,
        value: Some(json!({"amount": u64::MAX,"payload": ["first"]})),
        protected: ProtectedMetadata::default(),
    }
}

fn save(c: &Connection, row: &Row, text: &str) {
    c.execute(
        "INSERT OR REPLACE INTO resources VALUES (?,?,?,?)",
        params![
            row.key.kind,
            row.key.id,
            i64::try_from(row.revision).unwrap(),
            text
        ],
    )
    .unwrap();
}

fn add(c: &Connection, row: &Row) {
    save(c, row, &serde_json::to_string(row).unwrap());
    catalog::replace(c, None, row, None, || Ok(())).unwrap();
}

fn collector(c: &Connection, limits: BackupLimits) -> Collector {
    let state = serde_json::to_string(&StorageState::new(Default::default()).unwrap()).unwrap();
    let mut collector = Collector::new(&state, limits).unwrap();
    let mut s = c
        .prepare("SELECT kind,data FROM schemas ORDER BY kind")
        .unwrap();
    let mut rows = s.query([]).unwrap();
    while let Some(r) = rows.next().unwrap() {
        collector
            .descriptor(
                r.get_ref(0).unwrap().as_str().unwrap(),
                r.get_ref(1).unwrap().as_str().unwrap(),
            )
            .unwrap();
    }
    let mut s = c
        .prepare("SELECT kind,id,revision,data FROM resources ORDER BY kind,id")
        .unwrap();
    let mut rows = s.query([]).unwrap();
    while let Some(r) = rows.next().unwrap() {
        collector
            .row(
                r.get_ref(0).unwrap().as_str().unwrap(),
                r.get_ref(1).unwrap().as_str().unwrap(),
                Some(u64::try_from(r.get::<_, i64>(2).unwrap()).unwrap()),
                r.get_ref(3).unwrap().as_str().unwrap(),
            )
            .unwrap();
    }
    collector
}

fn counters(c: &Connection, kind: &str) -> (i64, i64, i64, i64, Vec<u8>) {
    c.query_row("SELECT row_count,row_bytes,canonical_row_bytes,live_count,generation FROM query_kinds WHERE kind=?",[kind],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap()
}

fn store(c: &Connection) -> String {
    c.query_row("SELECT store FROM query_profile WHERE id=1", [], |r| {
        r.get(0)
    })
    .unwrap()
}

#[test]
fn registration_preserves_omitted_kinds_and_accepts_canonical_field_order() {
    let c = connection();
    register(&c, "items");
    register(&c, "omitted");
    add(&c, &row("omitted"));
    let before = counters(&c, "omitted");
    let mut reordered = descriptor("items");
    reordered.fields.reverse();
    catalog::register(&c, &[reordered]).unwrap();
    assert_eq!(counters(&c, "omitted"), before);
    assert_eq!(
        counters(&c, "items"),
        (0, 0, 0, 0, 0_u64.to_be_bytes().to_vec())
    );
    let mut changed = descriptor("items");
    changed.version = 2;
    assert!(catalog::register(&c, &[changed]).is_err());
    validation::validate(
        &c,
        &mut collector(&c, BackupLimits::default()),
        BackupLimits::default(),
    )
    .unwrap();
}

#[test]
fn key_deltas_change_only_indexed_values_and_tombstones_still_count() {
    let mut c = connection();
    register(&c, "items");
    let original = row("items");
    let tx = c.transaction().unwrap();
    save(&tx, &original, &serde_json::to_string(&original).unwrap());
    let mut writes = 0;
    catalog::replace(&tx, None, &original, None, || {
        writes += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(writes, 3);
    tx.commit().unwrap();
    c.execute_batch("CREATE TABLE observed(operation TEXT); CREATE TRIGGER key_update AFTER UPDATE ON query_keys BEGIN INSERT INTO observed VALUES('update'); END;
        CREATE TRIGGER key_insert AFTER INSERT ON query_keys BEGIN INSERT INTO observed VALUES('insert'); END;
        CREATE TRIGGER key_delete AFTER DELETE ON query_keys BEGIN INSERT INTO observed VALUES('delete'); END;").unwrap();
    let mut next = original.clone();
    next.revision += 1;
    next.value.as_mut().unwrap()["payload"] = json!(["second"]);
    let tx = c.transaction().unwrap();
    save(&tx, &next, &serde_json::to_string(&next).unwrap());
    writes = 0;
    catalog::replace(
        &tx,
        Some(&original),
        &next,
        Some(serde_json::to_string(&original).unwrap().len()),
        || {
            writes += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(writes, 1);
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM observed", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let mut changed = next.clone();
    changed.revision += 1;
    changed.value.as_mut().unwrap()["amount"] = json!(0);
    let tx = c.transaction().unwrap();
    save(&tx, &changed, &serde_json::to_string(&changed).unwrap());
    writes = 0;
    catalog::replace(
        &tx,
        Some(&next),
        &changed,
        Some(serde_json::to_string(&next).unwrap().len()),
        || {
            writes += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(writes, 2);
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM observed", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    let mut deleted = changed.clone();
    deleted.revision += 1;
    deleted.protected.deletion_authorization = deleted.value.take();
    let tx = c.transaction().unwrap();
    save(&tx, &deleted, &serde_json::to_string(&deleted).unwrap());
    writes = 0;
    catalog::replace(
        &tx,
        Some(&changed),
        &deleted,
        Some(serde_json::to_string(&changed).unwrap().len()),
        || {
            writes += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(writes, 3);
    tx.commit().unwrap();
    let n = serde_json::to_string(&deleted).unwrap().len() as i64;
    assert_eq!(
        counters(&c, "items"),
        (1, n, n, 0, 4_u64.to_be_bytes().to_vec())
    );
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM query_keys", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn checkpoint_failure_rolls_back_keys_metadata_and_authoritative_row() {
    let mut c = connection();
    register(&c, "items");
    let original = row("items");
    add(&c, &original);
    let before = counters(&c, "items");
    let mut next = original.clone();
    next.revision += 1;
    next.value.as_mut().unwrap()["amount"] = json!(3);
    {
        let tx = c.transaction().unwrap();
        save(&tx, &next, &serde_json::to_string(&next).unwrap());
        let error = catalog::replace(
            &tx,
            Some(&original),
            &next,
            Some(serde_json::to_string(&original).unwrap().len()),
            || Err(Error::NotCommitted),
        );
        assert_eq!(error, Err(Error::NotCommitted));
    }
    assert_eq!(counters(&c, "items"), before);
    validation::validate(
        &c,
        &mut collector(&c, BackupLimits::default()),
        BackupLimits::default(),
    )
    .unwrap();
    c.execute(
        "UPDATE query_kinds SET generation=?",
        [u64::MAX.to_be_bytes().as_slice()],
    )
    .unwrap();
    let tx = c.transaction().unwrap();
    assert_eq!(
        catalog::replace(
            &tx,
            Some(&original),
            &next,
            Some(serde_json::to_string(&original).unwrap().len()),
            || panic!("overflow must precede writes")
        ),
        Err(Error::TooLarge)
    );
}

#[test]
fn rebuild_counts_native_text_and_preserves_the_complete_catalog() {
    let c = connection();
    register(&c, "items");
    register(&c, "omitted");
    let original = row("items");
    let raw = serde_json::to_string_pretty(&original).unwrap();
    save(&c, &original, &raw);
    let prior_store = store(&c);
    let snapshot = collector(&c, BackupLimits::default()).snapshot;
    catalog::rebuild(&c, &snapshot, BackupLimits::default()).unwrap();
    assert_ne!(store(&c), prior_store);
    assert_eq!(
        counters(&c, "items"),
        (
            1,
            raw.len() as i64,
            serde_json::to_string(&original).unwrap().len() as i64,
            1,
            0_u64.to_be_bytes().to_vec()
        )
    );
    assert_eq!(counters(&c, "omitted").0, 0);
    validation::validate(
        &c,
        &mut collector(&c, BackupLimits::default()),
        BackupLimits::default(),
    )
    .unwrap();
}

#[test]
fn validation_rejects_missing_extra_wrong_membership_and_metadata() {
    for sql in [
        "DELETE FROM query_keys WHERE field='amount'",
        "INSERT INTO query_keys VALUES('items','extra','a',x'02')",
        "UPDATE query_keys SET encoded=x'0200' WHERE field='amount'",
        "UPDATE query_keys SET id='orphan' WHERE field='amount'",
        "UPDATE query_kinds SET row_count=2",
        "UPDATE query_kinds SET row_bytes=row_bytes+1",
        "UPDATE query_kinds SET canonical_row_bytes=canonical_row_bytes+1",
        "UPDATE query_kinds SET live_count=0",
        "DELETE FROM query_kinds",
        "DELETE FROM query_profile",
        "UPDATE query_profile SET encoding_version=99",
        "UPDATE query_profile SET store=''",
    ] {
        let c = connection();
        register(&c, "items");
        add(&c, &row("items"));
        c.execute_batch(sql).unwrap();
        assert!(
            validation::validate(
                &c,
                &mut collector(&c, BackupLimits::default()),
                BackupLimits::default()
            )
            .is_err(),
            "accepted corruption: {sql}"
        );
    }
}

#[test]
fn physical_entries_share_the_logical_record_budget() {
    let c = connection();
    register(&c, "items");
    add(&c, &row("items"));
    let limits = BackupLimits {
        max_records: 5,
        ..BackupLimits::default()
    };
    assert_eq!(
        validation::validate(&c, &mut collector(&c, limits), limits),
        Err(Error::TooLarge)
    );
    let limits = BackupLimits {
        max_records: 6,
        ..BackupLimits::default()
    };
    validation::validate(&c, &mut collector(&c, limits), limits).unwrap();
}

#[test]
fn validation_rejects_changed_index_collation_and_table_layout() {
    for sql in [
        "DROP INDEX query_keys_value; CREATE INDEX query_keys_value ON query_keys(kind COLLATE NOCASE,field,encoded,id)",
        "DROP INDEX query_keys_value; CREATE INDEX query_keys_value ON query_keys(kind,field,encoded,id) WHERE field='amount'",
        "DROP INDEX query_keys_value; ALTER TABLE query_keys RENAME TO previous_keys;
         CREATE TABLE query_keys(kind TEXT NOT NULL,field TEXT NOT NULL,id TEXT NOT NULL,encoded BLOB NOT NULL,PRIMARY KEY(kind,id,field));
         INSERT INTO query_keys SELECT * FROM previous_keys; DROP TABLE previous_keys;
         CREATE INDEX query_keys_value ON query_keys(kind,field,encoded,id)",
    ] {
        let c = connection(); register(&c,"items"); add(&c,&row("items"));
        c.execute_batch(sql).unwrap();
        assert!(validation::validate(&c,&mut collector(&c,BackupLimits::default()),BackupLimits::default()).is_err(),"accepted physical layout change: {sql}");
    }
}
