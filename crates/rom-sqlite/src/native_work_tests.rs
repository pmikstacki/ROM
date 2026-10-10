//! Real SQLite layout and transaction regressions for incremental durable Work.
use crate::Sqlite;

#[test]
fn native_layout_separates_retained_work_from_metadata() {
    let db = Sqlite::open(":memory:").unwrap();
    let connection = db.connection.lock().unwrap();
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 11);
    let text: String = connection
        .query_row("SELECT data FROM rom_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(metadata.get("work").is_none());
    assert!(metadata.get("operator").is_none());
    for table in [
        "work_header",
        "work_records",
        "work_roots",
        "work_active",
        "operator_state",
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing {table}");
    }
}

use crate::native_work;
use crate::native_work_test_support::*;
use rom::storage_support::work::{WorkRead, prepare_update};
use rom::*;

#[test]
fn retained_done_history_is_not_examined_by_native_claim() {
    let db = fixture();
    for id in 0..200 {
        db.commit(&bundle(&format!("{id:04}"))).unwrap();
        done(&db);
    }
    db.commit(&bundle("9999")).unwrap();
    let connection = db.connection.lock().unwrap();
    let reader = native_work::Reader::new(&connection);
    let delta = prepare_update(&reader, WorkUpdate::Claim { now: 0 }).unwrap();
    assert!(
        matches!(delta.result(), WorkResult::Claimed(claim) if claim.work.pending.id == "work-9999")
    );
    assert_eq!(reader.examined.get(), 1);
    assert!(reader.decoded.get() <= 3);
    let canonical =
        native_work::reconstruct(&connection, rom_backup::BackupLimits::default()).unwrap();
    assert_eq!(canonical.state.work.records().len(), 201);
    assert_eq!(
        canonical
            .state
            .work
            .records()
            .iter()
            .filter(|record| record.state == WorkState::Done)
            .count(),
        200
    );
}

#[test]
fn prepared_delta_cannot_cross_native_reader_contexts() {
    let db = fixture();
    db.commit(&bundle("one")).unwrap();
    let connection = db.connection.lock().unwrap();
    let first = native_work::Reader::new(&connection);
    let delta = prepare_update(&first, WorkUpdate::Claim { now: 0 }).unwrap();
    let mut foreign = native_work::Reader::new(&connection);
    assert_eq!(foreign.apply(delta, || Ok(())), Err(Error::Conflict));
    assert_eq!(
        foreign.record("work-one").unwrap().unwrap().state,
        WorkState::Pending
    );
}

#[test]
fn native_canonical_reconstruction_rejects_corrupt_derived_projections() {
    for mutation in [
        "UPDATE work_roots SET data=json_set(data,'$.used',99)",
        "UPDATE work_header SET data=json_set(data,'$.accounting.work.entries',99) WHERE id=1",
        "DELETE FROM work_active",
        "INSERT INTO work_active VALUES('foreign')",
        "UPDATE work_records SET data=json_set(data,'$.pending.id','foreign')",
    ] {
        let db = fixture();
        db.commit(&bundle("one")).unwrap();
        let connection = db.connection.lock().unwrap();
        connection.execute_batch(mutation).unwrap();
        assert!(
            native_work::reconstruct(&connection, rom_backup::BackupLimits::default()).is_err(),
            "accepted {mutation}"
        );
    }
}

#[test]
fn native_raw_whitespace_is_bounded_before_canonical_normalization() {
    let db = fixture();
    let connection = db.connection.lock().unwrap();
    connection
        .execute(
            "UPDATE work_header SET data=data || ? WHERE id=1",
            [" ".repeat(4096)],
        )
        .unwrap();
    assert!(matches!(
        native_work::reconstruct(
            &connection,
            rom_backup::BackupLimits {
                max_bytes: 2048,
                max_records: 100
            }
        ),
        Err(Error::TooLarge)
    ));
}

#[cfg(feature = "test-support")]
#[test]
fn every_native_work_write_rolls_back_or_recovers_unknown_acknowledgement() {
    use std::sync::Arc;
    for point in 1..=4 {
        let db = fixture();
        db.commit(&bundle("one")).unwrap();
        let before = serde_json::to_value(
            crate::persistence::state(&db.connection.lock().unwrap(), db.validation_limits)
                .unwrap(),
        )
        .unwrap();
        db.on_commit(Some(Arc::new(move |ordinal| {
            if ordinal == point {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.reaction_update(WorkUpdate::Claim { now: 0 }),
            Err(Error::NotCommitted)
        );
        db.on_commit(None);
        let after = serde_json::to_value(
            crate::persistence::state(&db.connection.lock().unwrap(), db.validation_limits)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after, before, "partial write at {point}");
    }
    let db = fixture();
    db.commit(&bundle("one")).unwrap();
    db.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == usize::MAX {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    assert_eq!(
        db.reaction_update(WorkUpdate::Claim { now: 0 }),
        Err(Error::Unknown)
    );
    db.on_commit(None);
    let record = db.reaction_records().unwrap().pop().unwrap();
    assert!(matches!(
        record.state,
        WorkState::Leased { generation: 1, .. }
    ));
    assert_eq!(record.attempts, 1);
    assert_eq!(
        db.reaction_update(WorkUpdate::Claim { now: 0 }),
        Ok(WorkResult::Idle)
    );
}

#[test]
fn native_reopen_validates_inventory_and_every_stored_projection() {
    for mutation in [
        "UPDATE work_roots SET data=json_set(data,'$.incomplete',0)",
        "DELETE FROM work_active",
        "INSERT INTO work_active VALUES('foreign')",
        "CREATE TABLE extra_work(id TEXT)",
        "DROP TABLE operator_state",
        "UPDATE work_header SET id=2",
    ] {
        let scratch = Scratch::new();
        let path = scratch.path("store.db");
        let db = file_fixture(&path);
        db.commit(&bundle("one")).unwrap();
        db.close().unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        if mutation == "UPDATE work_header SET id=2" {
            connection
                .execute_batch("PRAGMA ignore_check_constraints=ON")
                .unwrap();
        }
        connection.execute_batch(mutation).unwrap();
        drop(connection);
        assert!(Sqlite::open(&path).is_err(), "open accepted {mutation}");
    }
}

#[test]
fn accepted_format_nine_upgrade_is_fresh_and_source_preserving() {
    let scratch = Scratch::new();
    let source = scratch.path("legacy.db");
    let destination = scratch.path("upgraded.db");
    let db = file_fixture(&source);
    db.commit(&bundle("done")).unwrap();
    done(&db);
    db.commit(&bundle("pending")).unwrap();
    let state =
        crate::persistence::state(&db.connection.lock().unwrap(), db.validation_limits).unwrap();
    let expected_records = state.work.records();
    let counts = db.counts().unwrap();
    db.close().unwrap();
    let connection = rusqlite::Connection::open(&source).unwrap();
    connection.execute_batch("PRAGMA journal_mode=DELETE; DROP TABLE work_header; DROP TABLE work_records; DROP TABLE work_roots; DROP TABLE work_active; DROP TABLE operator_state; DROP TABLE journal_positions;").unwrap();
    connection
        .execute(
            "UPDATE rom_state SET data=? WHERE id=1",
            [serde_json::to_string(&state).unwrap()],
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 9).unwrap();
    drop(connection);
    let original = std::fs::read(&source).unwrap();
    assert!(matches!(Sqlite::open(&source), Err(Error::Unsupported(_))));
    assert_eq!(std::fs::read(&source).unwrap(), original);
    let upgraded = Sqlite::upgrade_from(
        &source,
        &destination,
        &[descriptor()],
        rom_backup::BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert_eq!(upgraded.counts().unwrap(), counts);
    assert_eq!(upgraded.reaction_records().unwrap(), expected_records);
    upgraded.close().unwrap();
    let reopened = Sqlite::open(&destination).unwrap();
    assert_eq!(reopened.counts().unwrap(), counts);
    assert_eq!(reopened.reaction_records().unwrap(), expected_records);
}

#[test]
fn canonical_backup_restore_preserves_retained_history_and_fences_claims() {
    let scratch = Scratch::new();
    let source = scratch.path("source.db");
    let archive = scratch.path("backup.rom");
    let destination = scratch.path("restored.db");
    let db = file_fixture(&source);
    db.commit(&bundle("done")).unwrap();
    done(&db);
    db.commit(&bundle("pending")).unwrap();
    let active_claim = claim(&db);
    let counts = db.counts().unwrap();
    db.backup_to(&archive, rom_backup::BackupLimits::default())
        .unwrap();
    let restored =
        Sqlite::restore_from(&archive, &destination, rom_backup::BackupLimits::default()).unwrap();
    assert_eq!(restored.counts().unwrap(), counts);
    assert_eq!(restored.reaction_records().unwrap().len(), 2);
    assert_eq!(
        restored
            .reaction_records()
            .unwrap()
            .iter()
            .filter(|record| record.state == WorkState::Done)
            .count(),
        1
    );
    assert_eq!(
        restored.reaction_update(WorkUpdate::Materialize {
            claim: active_claim.key(),
            now: 0,
            children: vec![]
        }),
        Err(Error::Conflict)
    );
    restored.close().unwrap();
    let reopened = Sqlite::open(&destination).unwrap();
    assert_eq!(reopened.reaction_records().unwrap().len(), 2);
}

#[cfg(feature = "test-support")]
#[test]
fn resource_bundle_work_writes_are_atomic_and_unknown_ack_replays() {
    use std::sync::{Arc, Mutex};
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let db = fixture();
    let observed = recorded.clone();
    db.on_commit(Some(Arc::new(move |ordinal| {
        observed.lock().unwrap().push(ordinal);
        Ok(())
    })));
    db.commit(&bundle("probe")).unwrap();
    let last_write = *recorded
        .lock()
        .unwrap()
        .iter()
        .filter(|n| **n != 0 && **n != usize::MAX)
        .max()
        .unwrap();
    // Final writes are Work record, active ID, root, Work header and bounded metadata.
    for point in last_write - 4..=last_write {
        let scratch = Scratch::new();
        let path = scratch.path("store.db");
        let db = file_fixture(&path);
        db.on_commit(Some(Arc::new(move |ordinal| {
            if ordinal == point {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(db.commit(&bundle("one")), Err(Error::NotCommitted));
        db.on_commit(None);
        db.close().unwrap();
        let reopened = Sqlite::open(&path).unwrap();
        assert_eq!(reopened.counts().unwrap(), [0; 4]);
        assert!(reopened.reaction_records().unwrap().is_empty());
    }
    let scratch = Scratch::new();
    let path = scratch.path("unknown.db");
    let db = file_fixture(&path);
    let submitted = bundle("one");
    db.inject_fault(5);
    assert_eq!(db.commit(&submitted), Err(Error::Unknown));
    db.close().unwrap();
    let reopened = Sqlite::open(&path).unwrap();
    assert_eq!(reopened.commit(&submitted).unwrap(), submitted.receipt);
    assert_eq!(reopened.counts().unwrap(), [1, 1, 1, 0]);
    assert_eq!(reopened.reaction_records().unwrap().len(), 1);
}

#[test]
fn readonly_record_preconditions_survive_an_unchanged_native_delta() {
    let db = fixture();
    let submitted = bundle("one");
    db.commit(&submitted).unwrap();
    let connection = db.connection.lock().unwrap();
    let mut reader = native_work::Reader::new(&connection);
    let delta =
        rom::storage_support::work::prepare_enqueue(&reader, &limits(), submitted.reactions)
            .unwrap();
    assert_eq!(delta.records().count(), 0);
    assert!(delta.record_preconditions().any(|(id, _)| id == "work-one"));
    connection.execute("UPDATE work_records SET data=json_set(data,'$.pending.definition','other-source') WHERE id='work-one'", []).unwrap();
    assert_eq!(reader.apply(delta, || Ok(())), Err(Error::Conflict));
    assert_eq!(
        reader
            .record("work-one")
            .unwrap()
            .unwrap()
            .pending
            .definition,
        "other-source"
    );
}

#[test]
fn operator_snapshot_rejects_oversized_raw_work_before_decode() {
    let db = fixture();
    db.connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO work_records VALUES('oversized',?)",
            ["x".repeat(8192)],
        )
        .unwrap();
    assert_eq!(db.operator_snapshot(10, 128), Err(Error::TooLarge));
}

#[test]
fn operator_snapshot_preserves_logical_record_and_byte_bounds() {
    let db = fixture();
    for id in 0..100 {
        let mut submitted = bundle(&format!("row-{id}"));
        submitted.reactions.clear();
        submitted.reaction_limits = None;
        db.commit(&submitted).unwrap();
    }
    db.commit(&bundle("work")).unwrap();
    let snapshot = db.operator_snapshot(1, 1_000_000).unwrap();
    let exact_bytes = serde_json::to_vec(&snapshot).unwrap().len();
    assert_eq!(db.operator_snapshot(1, exact_bytes).unwrap(), snapshot);
    assert_eq!(
        db.operator_snapshot(1, exact_bytes - 1),
        Err(Error::TooLarge)
    );
}

#[test]
fn configured_native_budget_applies_to_operator_and_bulk_work_reads() {
    let db = Sqlite::open_with_validation_limits(
        ":memory:",
        StorageLimits::default(),
        rom_backup::BackupLimits {
            max_records: 100,
            max_bytes: 4096,
        },
    )
    .unwrap();
    db.connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO work_records VALUES('oversized',?)",
            ["x".repeat(8192)],
        )
        .unwrap();
    // The logical caller budget admits these raw payload bytes. The configured
    // native validation budget must reject them before invalid JSON is decoded.
    assert_eq!(db.operator_snapshot(100, 16_384), Err(Error::TooLarge));
    assert_eq!(db.reaction_records(), Err(Error::TooLarge));
}

#[test]
fn prepared_native_delta_rechecks_the_persisted_header() {
    let db = fixture();
    db.commit(&bundle("one")).unwrap();
    let connection = db.connection.lock().unwrap();
    let mut reader = native_work::Reader::new(&connection);
    let delta = prepare_update(&reader, WorkUpdate::Claim { now: 0 }).unwrap();
    connection
        .execute(
            "UPDATE work_header SET data=json_set(data,'$.retry_epochs.current',1) WHERE id=1",
            [],
        )
        .unwrap();
    assert_eq!(reader.apply(delta, || Ok(())), Err(Error::Conflict));
    assert_eq!(
        reader.record("work-one").unwrap().unwrap().state,
        WorkState::Pending
    );
}
