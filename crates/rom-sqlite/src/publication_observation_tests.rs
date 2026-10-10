//! Real SQL byte accounting and outcome equivalence, not durable-write attribution.
use crate::native_work_test_support::{bundle, claim, fixture};
use crate::{PublicationCategory as Category, PublicationSnapshot, StageOperation as Operation};
use rom::*;

fn entry(
    snapshot: &PublicationSnapshot,
    op: Operation,
    category: Category,
) -> crate::PublicationMeasurement {
    *snapshot
        .entries
        .iter()
        .find(|e| e.operation == op && e.category == category)
        .unwrap()
}
fn raw_size(db: &crate::Sqlite, table: &str) -> u64 {
    u64::try_from(
        db.connection
            .lock()
            .unwrap()
            .query_row(
                &format!("SELECT sum(length(CAST(data AS BLOB))) FROM {table}"),
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn publication_bytes_match_real_persisted_strings_and_work_updates() {
    let db = fixture();
    let observation = db.observe_stages();
    let first = bundle("one");
    db.commit(&first).unwrap();
    let snapshot = observation.publication_snapshot();
    assert_eq!(snapshot.entries.len(), 10);
    for (category, table) in [
        (Category::Metadata, "rom_state"),
        (Category::EventPayload, "events"),
        (Category::WorkHeader, "work_header"),
        (Category::WorkRecord, "work_records"),
        (Category::WorkRoot, "work_roots"),
    ] {
        let actual = entry(&snapshot, Operation::Commit, category);
        assert_eq!(actual.samples, 1, "{category:?}");
        assert_eq!(actual.encoded_bytes, raw_size(&db, table));
    }
    let leased = claim(&db);
    let claim_sizes = [
        raw_size(&db, "work_header"),
        raw_size(&db, "work_records"),
        raw_size(&db, "work_roots"),
    ];
    db.reaction_update(WorkUpdate::Materialize {
        claim: leased.key(),
        now: 0,
        children: vec![],
    })
    .unwrap();
    let after = observation.publication_snapshot();
    for ((category, table), before) in [
        (Category::WorkHeader, "work_header"),
        (Category::WorkRecord, "work_records"),
        (Category::WorkRoot, "work_roots"),
    ]
    .into_iter()
    .zip(claim_sizes)
    {
        let actual = entry(&after, Operation::WorkUpdate, category);
        assert_eq!(actual.samples, 2);
        assert_eq!(actual.encoded_bytes, before + raw_size(&db, table));
    }
    assert_eq!(
        entry(&after, Operation::WorkUpdate, Category::Metadata).samples,
        0
    );
    assert_eq!(
        entry(&after, Operation::WorkUpdate, Category::EventPayload).samples,
        0
    );
    assert_eq!(db.counts().unwrap(), [1, 1, 1, 0]);
    assert_eq!(db.reaction_records().unwrap()[0].state, WorkState::Done);
}

fn outcomes(db: &crate::Sqlite) -> (Vec<Receipt>, Vec<WorkRecord>, [u64; 4]) {
    let first = bundle("first");
    let accepted = db.commit(&first).unwrap();
    let claimed = claim(db);
    db.reaction_update(WorkUpdate::Materialize {
        claim: claimed.key(),
        now: 0,
        children: vec![],
    })
    .unwrap();
    db.inject_fault(2);
    assert_eq!(
        db.commit(&bundle("rolled-back-event")),
        Err(Error::NotCommitted)
    );
    assert!(
        db.load(&bundle("rolled-back-event").receipt.row.key)
            .unwrap()
            .is_none()
    );
    assert!(db.receipt("create-rolled-back-event").unwrap().is_none());
    let lost = bundle("lost");
    db.inject_fault(5);
    assert_eq!(db.commit(&lost), Err(Error::Unknown));
    let replay = db.commit(&lost).unwrap();
    assert_eq!(replay, lost.receipt);
    assert_eq!(accepted.row.value, Some(json!({"amount":1})));
    let records = db.reaction_records().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .iter()
            .find(|r| r.pending.id == "work-first")
            .unwrap()
            .state,
        WorkState::Done
    );
    assert_eq!(
        records
            .iter()
            .find(|r| r.pending.id == "work-lost")
            .unwrap()
            .state,
        WorkState::Pending
    );
    assert_eq!(db.counts().unwrap(), [2, 2, 2, 0]);
    (vec![accepted, replay], records, db.counts().unwrap())
}
#[test]
fn publications_include_rolled_back_sql_and_unknown_ack_but_not_replay() {
    let disabled = fixture();
    let db = fixture();
    let observation = db.observe_stages();
    assert_eq!(outcomes(&db), outcomes(&disabled));
    assert!(disabled.stage_observation.get().is_none());
    let snapshot = observation.publication_snapshot();
    assert_eq!(
        entry(&snapshot, Operation::Commit, Category::Metadata).samples,
        2
    );
    for category in [
        Category::WorkHeader,
        Category::WorkRecord,
        Category::WorkRoot,
    ] {
        assert_eq!(entry(&snapshot, Operation::Commit, category).samples, 2);
        assert_eq!(entry(&snapshot, Operation::WorkUpdate, category).samples, 2);
    }
    let event = entry(&snapshot, Operation::Commit, Category::EventPayload);
    assert_eq!(event.samples, 3);
    assert_eq!(
        event.encoded_bytes,
        raw_size(&db, "events")
            + u64::try_from(
                serde_json::to_string(&bundle("rolled-back-event").receipt.row)
                    .unwrap()
                    .len()
            )
            .unwrap()
    );
    let before = observation.publication_snapshot();
    db.commit(&bundle("lost")).unwrap();
    db.retry_epochs().unwrap();
    assert_eq!(before, observation.publication_snapshot());
}

#[test]
fn metadata_bytes_accumulate_actual_scalar_header_values() {
    let db = fixture();
    let handle = db.observe_stages();
    db.commit(&bundle("first")).unwrap();
    let first = raw_size(&db, "rom_state");
    db.commit(&bundle("second")).unwrap();
    let second = raw_size(&db, "rom_state");
    assert_eq!(second, first);
    let metadata = entry(
        &handle.publication_snapshot(),
        Operation::Commit,
        Category::Metadata,
    );
    assert_eq!(metadata.samples, 2);
    assert_eq!(metadata.encoded_bytes, first + second);
}

#[test]
fn rejected_sql_statement_is_not_a_successful_publication_sample() {
    let db = fixture();
    let observer = db.observe_stages();
    db.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'controlled event write rejection'); END;").unwrap();
    assert_eq!(db.commit(&bundle("rejected-sql")), Err(Error::NotCommitted));
    db.connection
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_event;")
        .unwrap();
    assert_eq!(db.counts().unwrap(), [0, 0, 0, 0]);
    assert!(
        observer
            .publication_snapshot()
            .entries
            .iter()
            .all(|entry| entry.samples == 0 && entry.encoded_bytes == 0)
    );
    assert!(
        db.load(&bundle("rejected-sql").receipt.row.key)
            .unwrap()
            .is_none()
    );
}
