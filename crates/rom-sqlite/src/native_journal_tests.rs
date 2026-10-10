fn physical_rows(db: &crate::Sqlite) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    let c = db.connection.lock().unwrap();
    let mut names = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT GLOB 'sqlite_*' ORDER BY name").unwrap();
    let tables = names
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    tables
        .into_iter()
        .map(|name| {
            let mut query = c.prepare(&format!("SELECT * FROM {name}")).unwrap();
            let columns = query.column_count();
            let rows = query
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            (name, rows)
        })
        .collect::<Vec<_>>()
}

use crate::native_work_test_support::{bundle, fixture};

fn candidate() -> crate::Sqlite {
    fixture()
}

#[test]
fn candidate_resource_commit_and_replay_use_keyed_journal() {
    use rom::Storage;
    let db = candidate();
    let b = bundle("first");
    assert_eq!(db.commit(&b).unwrap(), b.receipt);
    assert_eq!(db.commit(&b).unwrap(), b.receipt);
    let page = db.journal("native-work", None, 10, 100000).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].identity, b.receipt.identity);
    assert_eq!(page.cursor.position, 1);
    assert_eq!(db.retry_epochs().unwrap().current, 0);
}

#[test]
fn public_replay_rejects_touched_receipt_identity_corruption_without_writes() {
    use crate::native_work_test_support::{Scratch, file_fixture};
    use rom::Storage;
    let scratch = Scratch::new();
    let path = scratch.path("receipt-identity");
    let db = file_fixture(&path);
    let b = bundle("receipt-identity");
    db.commit(&b).unwrap();
    db.close().unwrap();
    // Validate startup and schema before corruption occurs in the live public store.
    let db = crate::Sqlite::open(&path).unwrap();
    assert_eq!(
        db.receipt(&b.receipt.identity).unwrap(),
        Some(b.receipt.clone())
    );
    let raw = rusqlite::Connection::open(&path).unwrap();
    let mut corrupted = b.receipt.clone();
    corrupted.identity = "different-persisted-identity".into();
    raw.execute(
        "UPDATE receipts SET data=? WHERE identity=?",
        rusqlite::params![
            serde_json::to_string(&corrupted).unwrap(),
            b.receipt.identity
        ],
    )
    .unwrap();
    drop(raw);
    let before = physical_rows(&db);
    assert_eq!(db.commit(&b), Err(rom::Error::Storage));
    assert_eq!(db.receipt(&b.receipt.identity), Err(rom::Error::Storage));
    assert_eq!(physical_rows(&db), before);
}

#[test]
fn public_reference_commit_rejects_live_target_key_corruption_without_writes() {
    use crate::native_work_test_support::{Scratch, file_fixture};
    use rom::Storage;
    let scratch = Scratch::new();
    let path = scratch.path("target-key");
    let db = file_fixture(&path);
    db.register(&[rom::Descriptor {
        kind: "reference-source".into(),
        version: 1,
        fields: vec![rom::FieldDescriptor {
            name: "target".into(),
            shape: rom::Shape::Reference {
                kind: "native-work".into(),
            },
        }],
    }])
    .unwrap();
    let target = bundle("target-key");
    db.commit(&target).unwrap();
    db.close().unwrap();
    let db = crate::Sqlite::open(&path).unwrap();
    assert_eq!(
        db.load(&target.receipt.row.key).unwrap(),
        Some(target.receipt.row.clone())
    );
    let raw = rusqlite::Connection::open(&path).unwrap();
    let mut corrupted = target.receipt.row.clone();
    corrupted.key.id = "different-persisted-target".into();
    raw.execute(
        "UPDATE resources SET data=? WHERE kind=? AND id=?",
        rusqlite::params![
            serde_json::to_string(&corrupted).unwrap(),
            target.receipt.row.key.kind,
            target.receipt.row.key.id
        ],
    )
    .unwrap();
    drop(raw);
    let mut referencing = bundle("reference-source");
    referencing.receipt.row.key.kind = "reference-source".into();
    referencing.receipt.row.value = Some(rom::json!({"target": target.receipt.row.key.id}));
    referencing.reactions.clear();
    referencing.reaction_limits = None;
    let before = physical_rows(&db);
    assert_eq!(db.commit(&referencing), Err(rom::Error::Storage));
    assert_eq!(db.load(&target.receipt.row.key), Err(rom::Error::Storage));
    assert_eq!(physical_rows(&db), before);
}

#[test]
fn candidate_point_reads_enforce_configured_physical_budget_before_decode() {
    use rom::Storage;
    let mut db = candidate();
    db.commit(&bundle("bounded")).unwrap();
    db.validation_limits.max_bytes = 1;
    assert_eq!(
        db.journal("native-work", None, 10, 100000),
        Err(rom::Error::TooLarge)
    );
    db.connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE rom_state SET data='invalid oversized json' WHERE id=1",
            [],
        )
        .unwrap();
    assert_eq!(db.retry_epochs(), Err(rom::Error::TooLarge));
}

#[test]
fn candidate_full_projection_preserves_canonical_fields() {
    use rom::Storage;
    let expected = crate::native_work_test_support::predecessor_fixture();
    let actual = candidate();
    for id in ["a", "b", "c"] {
        expected.commit(&bundle(id)).unwrap();
        actual.commit(&bundle(id)).unwrap();
    }
    let limits = rom_backup::BackupLimits::default();
    let old = crate::native_work::reconstruct(&expected.connection.lock().unwrap(), limits)
        .unwrap()
        .state;
    let new = crate::native_work::reconstruct(&actual.connection.lock().unwrap(), limits)
        .unwrap()
        .state;
    // Independent stores generate distinct journal generations.
    let mut value = serde_json::to_value(new).unwrap();
    let wanted = serde_json::to_value(old).unwrap();
    value["generation"] = wanted["generation"].clone();
    assert_eq!(value, wanted);
}

#[test]
fn candidate_maintenance_rejects_nonunique_identity_index() {
    use rom::Storage;
    let db = candidate();
    db.commit(&bundle("schema")).unwrap();
    let c = db.connection.lock().unwrap();
    c.execute_batch(
        "DROP INDEX journal_identity; CREATE INDEX journal_identity ON journal_positions(identity)",
    )
    .unwrap();
    assert!(matches!(
        crate::native_work::reconstruct(&c, rom_backup::BackupLimits::default()),
        Err(rom::Error::Storage)
    ));
}

#[test]
#[ignore = "Requires the preserved genuine current-format10 fixture"]
fn genuine_format10_candidate_conversion_preserves_explicit_restore_transition() {
    use crate::native_work_test_support::Scratch;
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.superpowers/rom-010-format10-writer-SX103q/sqlite");
    let original = fixture.join("source");
    let limits = rom_backup::BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    };
    let scratch = Scratch::new();
    let source = scratch.path("source10");
    let destination = scratch.path("candidate11");
    let bytes = std::fs::read(&original).unwrap();
    std::fs::write(&source, &bytes).unwrap();
    let (_, expected) = rom_backup::read(
        fixture.join("before.rombk"),
        rom_backup::Backend::Sqlite,
        limits,
    )
    .unwrap();
    let db =
        crate::Sqlite::upgrade_journal_candidate(&source, &destination, limits, || Ok(())).unwrap();
    let c = db.connection.lock().unwrap();
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 11);
    let snapshot = crate::snapshot::collect_native_snapshot(&c, limits, true).unwrap();
    let mut expected_state = expected.state;
    let original_generation = serde_json::to_value(&expected_state).unwrap()["generation"].clone();
    expected_state.prepare_restore().unwrap();
    let mut actual_state = serde_json::to_value(&snapshot.state).unwrap();
    let expected_state = serde_json::to_value(&expected_state).unwrap();
    // Each explicit restore gets its own generation; all other fields are exact.
    assert_ne!(actual_state["generation"], original_generation);
    assert_ne!(actual_state["generation"], expected_state["generation"]);
    actual_state["generation"] = expected_state["generation"].clone();
    assert_eq!(actual_state, expected_state);
    assert_eq!(snapshot.rows, expected.rows);
    assert_eq!(snapshot.receipts, expected.receipts);
    assert_eq!(snapshot.events, expected.events);
    let effects = |effects: Vec<rom_backup::StoredEffect>| {
        effects
            .into_iter()
            .map(|e| (e.identity, e.ordinal, e.intent))
            .collect::<Vec<_>>()
    };
    assert_eq!(effects(snapshot.effects), effects(expected.effects));
    assert_eq!(snapshot.descriptors, expected.descriptors);
    assert_eq!(snapshot.references, expected.references);
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(db.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(db.validation_limits.max_records, limits.max_records);
    let rejected = scratch.path("rejected-candidate");
    assert!(matches!(
        crate::Sqlite::upgrade_journal_candidate(&source, &rejected, limits, || Err(
            rom::Error::NotCommitted
        )),
        Err(rom::Error::NotCommitted)
    ));
    assert!(!rejected.exists());
    let insufficient = scratch.path("insufficient-budget");
    assert!(matches!(
        crate::Sqlite::upgrade_journal_candidate(
            &source,
            &insufficient,
            rom_backup::BackupLimits {
                max_bytes: 4096,
                max_records: 4096
            },
            || Ok(())
        ),
        Err(rom::Error::TooLarge)
    ));
    assert!(!insufficient.exists());
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
}

#[test]
fn candidate_unsigned_positions_and_constraints_preserve_full_range() {
    let db = candidate();
    let c = db.connection.lock().unwrap();
    let positions = [i64::MAX as u64, (i64::MAX as u64) + 1, u64::MAX];
    for position in positions {
        c.execute(
            "INSERT INTO journal_positions VALUES(?,?,?)",
            rusqlite::params![
                crate::native_journal::key(position).as_slice(),
                position.to_string(),
                crate::native_journal::key(1).as_slice()
            ],
        )
        .unwrap();
    }
    let mut query = c
        .prepare("SELECT position FROM journal_positions ORDER BY position")
        .unwrap();
    let ordered = query
        .query_map([], |r| r.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|v| u64::from_be_bytes(v.unwrap().try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(ordered, positions);
    assert!(
        c.execute(
            "INSERT INTO journal_positions VALUES(x'01','short',x'0000000000000001')",
            []
        )
        .is_err()
    );
    assert!(
        c.execute(
            "INSERT INTO journal_positions VALUES(x'0000000000000001',?,x'0000000000000001')",
            [positions[0].to_string()]
        )
        .is_err()
    );
}

#[test]
fn candidate_combined_read_facts_and_context_are_revalidated() {
    use rom::{
        Storage,
        storage_support::metadata::{JournalRead, prepare_native_bundle},
    };
    let db = candidate();
    db.commit(&bundle("a")).unwrap();
    let mut c = db.connection.lock().unwrap();
    let tx = c.transaction().unwrap();
    let reader = crate::native_work::Reader::bounded(&tx, db.validation_limits);
    let delta = prepare_native_bundle(&reader, &reader, &bundle("b")).unwrap();
    let other = crate::native_work::Reader::bounded(&tx, db.validation_limits);
    assert_eq!(delta.validate(&other, &other), Err(rom::Error::Conflict));
    let mut parts = JournalRead::header(&reader).unwrap().parts();
    parts.receipts += 1;
    tx.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(&parts).unwrap()],
    )
    .unwrap();
    assert_eq!(delta.validate(&reader, &reader), Err(rom::Error::Conflict));
    let rows: i64 = tx
        .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}

#[test]
fn candidate_retention_and_cursor_match_the_canonical_oracle() {
    use rom::Storage;
    let canonical = crate::native_work_test_support::predecessor_fixture();
    let candidate = candidate();
    for db in [&canonical, &candidate] {
        let c = db.connection.lock().unwrap();
        let raw: String = c
            .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
            .unwrap();
        let mut v: rom::Value = serde_json::from_str(&raw).unwrap();
        v["limits"]["journal_rows"] = rom::json!(2);
        c.execute(
            "UPDATE rom_state SET data=? WHERE id=1",
            [serde_json::to_string(&v).unwrap()],
        )
        .unwrap();
    }
    for id in ["a", "b", "c", "d"] {
        canonical.commit(&bundle(id)).unwrap();
        candidate.commit(&bundle(id)).unwrap();
    }
    assert_eq!(
        candidate.journal("native-work", None, 10, 100000),
        Err(rom::Error::HistoryGap)
    );
    let cursor = rom::JournalCursor {
        generation: candidate.journal_head("native-work").unwrap().generation,
        kind: "native-work".into(),
        position: 2,
    };
    let page = candidate
        .journal("native-work", Some(&cursor), 1, 100000)
        .unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].identity, "create-c");
    assert_eq!(page.cursor.position, 3);
    let after = candidate
        .journal("native-work", Some(&page.cursor), 10, 100000)
        .unwrap();
    assert_eq!(after.events[0].identity, "create-d");
    assert_eq!(candidate.counts().unwrap(), canonical.counts().unwrap());
}

#[cfg(feature = "test-support")]
#[test]
fn candidate_rolls_back_every_publication_boundary_and_replays_unknown_ack() {
    use rom::Storage;
    use std::sync::{Arc, Mutex};
    let db = candidate();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let capture = observed.clone();
    db.on_commit(Some(Arc::new(move |n| {
        capture.lock().unwrap().push(n);
        Ok(())
    })));
    db.commit(&bundle("probe")).unwrap();
    let ordinals = observed.lock().unwrap().clone();
    for ordinal in ordinals.into_iter().filter(|n| *n != usize::MAX) {
        let db = candidate();
        let before =
            crate::native_work::reconstruct(&db.connection.lock().unwrap(), db.validation_limits)
                .unwrap()
                .state;
        db.on_commit(Some(Arc::new(move |n| {
            if n == ordinal {
                Err(rom::Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.commit(&bundle("rollback")),
            Err(rom::Error::NotCommitted)
        );
        let after =
            crate::native_work::reconstruct(&db.connection.lock().unwrap(), db.validation_limits)
                .unwrap()
                .state;
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(after).unwrap()
        );
        assert_eq!(db.counts().unwrap(), [0; 4]);
    }
    let db = candidate();
    let b = bundle("unknown");
    db.inject_fault(5);
    assert_eq!(db.commit(&b), Err(rom::Error::Unknown));
    assert_eq!(db.commit(&b).unwrap(), b.receipt);
    assert_eq!(db.counts().unwrap(), [1, 1, 1, 0]);
}

#[test]
fn candidate_full_inventory_admission_precedes_any_json_decode() {
    use rom::Storage;
    let db = candidate();
    db.commit(&bundle("large")).unwrap();
    let c = db.connection.lock().unwrap();
    c.execute("UPDATE rom_state SET data='invalid json' WHERE id=1", [])
        .unwrap();
    c.execute("UPDATE work_records SET data=?", ["x".repeat(8192)])
        .unwrap();
    let limits = rom_backup::BackupLimits {
        max_bytes: 4096,
        max_records: 4096,
    };
    assert!(matches!(
        crate::native_work::reconstruct(&c, limits),
        Err(rom::Error::TooLarge)
    ));
}

#[test]
fn candidate_reader_budget_is_cumulative_across_both_traits() {
    use rom::storage_support::{metadata::JournalRead, work::WorkRead};
    let db = candidate();
    let c = db.connection.lock().unwrap();
    let length: i64 = c
        .query_row("SELECT length(data) FROM rom_state WHERE id=1", [], |r| {
            r.get(0)
        })
        .unwrap();
    let limits = rom_backup::BackupLimits {
        max_bytes: usize::try_from(length).unwrap() + 1,
        max_records: 4096,
    };
    let reader = crate::native_work::Reader::bounded(&c, limits);
    JournalRead::header(&reader).unwrap();
    assert_eq!(WorkRead::header(&reader), Err(rom::Error::TooLarge));
}

#[test]
fn candidate_work_publication_invalidates_combined_delta() {
    use rom::{
        Storage,
        storage_support::{metadata::prepare_native_bundle, work::prepare_update},
    };
    let db = candidate();
    db.commit(&bundle("a")).unwrap();
    let mut c = db.connection.lock().unwrap();
    let tx = c.transaction().unwrap();
    let mut reader = crate::native_work::Reader::bounded(&tx, db.validation_limits);
    let combined = prepare_native_bundle(&reader, &reader, &bundle("b")).unwrap();
    let work = prepare_update(&reader, rom::WorkUpdate::Claim { now: 0 }).unwrap();
    reader.apply(work, || Ok(())).unwrap();
    assert_eq!(
        combined.validate(&reader, &reader),
        Err(rom::Error::Conflict)
    );
}

#[test]
fn candidate_retired_payload_read_fact_is_not_a_cached_snapshot() {
    use rom::{Storage, storage_support::metadata::prepare_native_bundle};
    let db = candidate();
    db.commit(&bundle("a")).unwrap();
    let mut c = db.connection.lock().unwrap();
    let tx = c.transaction().unwrap();
    let raw: String = tx
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |r| r.get(0))
        .unwrap();
    let mut header: rom::storage_support::metadata::MetadataHeaderParts =
        serde_json::from_str(&raw).unwrap();
    header.limits.journal_rows = 1;
    tx.execute(
        "UPDATE rom_state SET data=? WHERE id=1",
        [serde_json::to_string(&header).unwrap()],
    )
    .unwrap();
    let reader = crate::native_work::Reader::bounded(&tx, db.validation_limits);
    let combined = prepare_native_bundle(&reader, &reader, &bundle("b")).unwrap();
    let mut altered = bundle("a").receipt.row;
    altered.value = Some(rom::json!({"amount":2}));
    tx.execute(
        "UPDATE events SET data=? WHERE identity='create-a'",
        [serde_json::to_string(&altered).unwrap()],
    )
    .unwrap();
    assert_eq!(
        combined.validate(&reader, &reader),
        Err(rom::Error::Conflict)
    );
}

#[test]
fn current_backup_publishes_admitted_keyed_format() {
    use crate::native_work_test_support::Scratch;
    let db = candidate();
    let scratch = Scratch::new();
    let path = scratch.path("false-format.rombk");
    let manifest = db
        .backup_to(&path, rom_backup::BackupLimits::default())
        .unwrap();
    assert_eq!(manifest.archive_version, 7);
    assert_eq!(manifest.storage_format, 11);
    assert!(path.exists());
}

#[test]
fn candidate_point_commit_does_not_decode_untouched_journal_history() {
    use rom::Storage;
    let db = candidate();
    db.commit(&bundle("a")).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE events SET data='invalid untouched history' WHERE identity='create-a'",
            [],
        )
        .unwrap();
    db.commit(&bundle("b")).unwrap();
    assert_eq!(db.journal_head("native-work").unwrap().position, 2);
    assert_eq!(
        db.journal("native-work", None, 10, 100000),
        Err(rom::Error::Storage)
    );
    assert!(matches!(
        crate::native_work::reconstruct(&db.connection.lock().unwrap(), db.validation_limits),
        Err(rom::Error::Storage)
    ));
}

#[test]
fn candidate_orphan_incoming_event_identity_is_corruption_before_publication() {
    use rom::Storage;
    let db = candidate();
    let b = bundle("orphan");
    let raw = serde_json::to_string(&b.receipt.row).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO events VALUES(?,?)",
            rusqlite::params![b.receipt.identity, raw],
        )
        .unwrap();
    assert_eq!(db.commit(&b), Err(rom::Error::Storage));
    assert_eq!(db.counts().unwrap(), [0, 1, 0, 0]);
}

#[test]
fn candidate_operator_control_preserves_keyed_layout_and_journal() {
    use rom::operator::{WorkControlOperation, WorkControlRequest, WorkHandle};
    use rom::*;
    let db = candidate();
    db.commit(&bundle("operator")).unwrap();
    let claim = crate::native_work_test_support::claim(&db);
    db.reaction_update(WorkUpdate::Finish {
        claim: claim.key(),
        now: 0,
        outcome: WorkOutcome::Stop(StopReason::Denied),
    })
    .unwrap();
    let snapshot = db.work_snapshot(100, 1000000).unwrap();
    let control = StorageWorkControl {
        principal: "operator".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("work-operator"),
            expected: snapshot.version(&snapshot.records[0]),
            key: "operator-retry".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        },
        decision: WorkControlDecision::Retry,
        now: 1,
    };
    db.control_work(&control).unwrap();
    assert_eq!(
        db.journal("native-work", None, 10, 100000)
            .unwrap()
            .events
            .len(),
        1
    );
    db.commit(&bundle("after-control")).unwrap();
}

#[test]
fn candidate_touched_receipt_and_resource_are_bounded_before_decode() {
    use rom::Storage;
    for table in ["receipts", "resources"] {
        let mut db = candidate();
        let b = bundle("bounded-touch");
        let oversized = "!".repeat(8192);
        {
            let c = db.connection.lock().unwrap();
            if table == "receipts" {
                c.execute(
                    "INSERT INTO receipts VALUES(?,?)",
                    rusqlite::params![b.receipt.identity, oversized],
                )
                .unwrap();
            } else {
                c.execute(
                    "INSERT INTO resources VALUES(?,?,?,?)",
                    rusqlite::params![b.receipt.row.key.kind, b.receipt.row.key.id, 1, oversized],
                )
                .unwrap();
            }
        }
        db.validation_limits.max_bytes = 4096;
        assert_eq!(
            db.commit(&b),
            Err(rom::Error::TooLarge),
            "unbounded touched {table}"
        );
    }
}

#[test]
fn candidate_touched_descriptor_is_bounded_before_decode() {
    use rom::Storage;
    let mut db = candidate();
    db.connection
        .lock()
        .unwrap()
        .execute("UPDATE schemas SET data=?", ["!".repeat(8192)])
        .unwrap();
    db.validation_limits.max_bytes = 4096;
    assert_eq!(db.commit(&bundle("descriptor")), Err(rom::Error::TooLarge));
}

#[test]
fn candidate_public_load_and_receipt_are_bounded_before_decode() {
    use rom::Storage;
    let mut db = candidate();
    let b = bundle("read-touch");
    {
        let c = db.connection.lock().unwrap();
        c.execute(
            "INSERT INTO receipts VALUES(?,?)",
            rusqlite::params![b.receipt.identity, "!".repeat(8192)],
        )
        .unwrap();
        c.execute(
            "INSERT INTO resources VALUES(?,?,?,?)",
            rusqlite::params![
                b.receipt.row.key.kind,
                b.receipt.row.key.id,
                1,
                "!".repeat(8192)
            ],
        )
        .unwrap();
    }
    db.validation_limits.max_bytes = 4096;
    assert_eq!(db.load(&b.receipt.row.key), Err(rom::Error::TooLarge));
    assert_eq!(db.receipt(&b.receipt.identity), Err(rom::Error::TooLarge));
}

#[test]
fn candidate_old_query_key_is_bounded_before_allocation() {
    use rom::Storage;
    let mut db = candidate();
    let mut next = bundle("query-touch");
    db.commit(&next).unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE query_keys SET encoded=? WHERE kind=? AND id=?",
            rusqlite::params![
                vec![0_u8; 8192],
                next.receipt.row.key.kind,
                next.receipt.row.key.id
            ],
        )
        .unwrap();
    next.expected = Some(1);
    next.receipt.identity = "update-query-touch".into();
    next.receipt.fingerprint = "update-query-touch".into();
    next.receipt.row.revision = 2;
    next.reactions.clear();
    db.validation_limits.max_bytes = 4096;
    assert_eq!(db.commit(&next), Err(rom::Error::TooLarge));
    let c = db.connection.lock().unwrap();
    let revision: i64 = c
        .query_row("SELECT revision FROM resources", [], |row| row.get(0))
        .unwrap();
    let receipts: i64 = c
        .query_row("SELECT COUNT(*) FROM receipts", [], |row| row.get(0))
        .unwrap();
    assert_eq!((revision, receipts), (1, 1));
}

#[test]
fn candidate_previous_reference_edges_are_bounded_before_allocation() {
    use rom::Storage;
    let mut db = candidate();
    let b = bundle("edge-touch");
    db.connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO reference_edges VALUES(?,?,?,?)",
            rusqlite::params![
                b.receipt.row.key.kind,
                b.receipt.row.key.id,
                "native-work",
                "x".repeat(8192)
            ],
        )
        .unwrap();
    db.validation_limits.max_bytes = 4096;
    assert_eq!(db.commit(&b), Err(rom::Error::TooLarge));
    assert_eq!(db.load(&b.receipt.row.key).unwrap(), None);
    assert_eq!(db.receipt(&b.receipt.identity).unwrap(), None);
}

#[test]
fn candidate_query_counter_payload_is_bounded_before_decode() {
    use rom::Storage;
    let mut db = candidate();
    db.connection
        .lock()
        .unwrap()
        .execute_batch("PRAGMA ignore_check_constraints=ON")
        .unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute("UPDATE query_kinds SET generation=?", [vec![0_u8; 8192]])
        .unwrap();
    db.validation_limits.max_bytes = 4096;
    assert_eq!(
        db.commit(&bundle("counter-touch")),
        Err(rom::Error::TooLarge)
    );
}

#[test]
fn candidate_query_counters_consume_one_physical_record() {
    let db = candidate();
    let c = db.connection.lock().unwrap();
    let reader = crate::native_work::Reader::bounded(
        &c,
        rom_backup::BackupLimits {
            max_bytes: 4096,
            max_records: 0,
        },
    );
    assert!(matches!(
        crate::index::prepare(
            &c,
            None,
            &bundle("counter-record").receipt.row,
            None,
            &crate::native_work_test_support::descriptor(),
            Some(&reader)
        ),
        Err(rom::Error::TooLarge)
    ));
}

#[test]
fn predecessor_conversion_has_keyed_journal_and_public_current_layout() {
    let current = crate::native_work_test_support::predecessor_fixture();
    let current_tables: i64 = current
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='journal_positions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(current_tables, 0);
    let candidate = current.make_journal_candidate().unwrap();
    let candidate_tables: i64 = candidate
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='journal_positions'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(candidate_tables, 1);
    // Resource publication is covered by the next genuine behavior test.
    let _ = bundle("candidate");
}
