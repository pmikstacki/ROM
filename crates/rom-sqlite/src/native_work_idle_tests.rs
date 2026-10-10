//! File-backed WAL/FULL characterization. Measurements do not assume idle SQL adds frames.
use crate::native_work_test_support::{Scratch, bundle, descriptor, file_fixture};
use crate::{PublicationCategory, Sqlite, StageObservation, StageOperation};
use rom::{Storage, WorkResult, WorkState, WorkUpdate};
use std::{io::Read, path::Path};

fn wal_frames(path: &Path) -> (u64, u64) {
    let wal = path.with_file_name(format!(
        "{}-wal",
        path.file_name().unwrap().to_str().unwrap()
    ));
    let length = match std::fs::metadata(&wal) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (0, 0),
        Err(error) => panic!("WAL metadata: {error}"),
    };
    assert!(length <= 4 * 1024 * 1024, "bounded characterization WAL");
    if length == 0 {
        return (0, 0);
    }
    let mut header = [0u8; 32];
    std::fs::File::open(wal)
        .unwrap()
        .read_exact(&mut header)
        .unwrap();
    let magic = u32::from_be_bytes(header[..4].try_into().unwrap());
    assert!(matches!(magic, 0x377f0682 | 0x377f0683));
    let page_size = u32::from_be_bytes(header[8..12].try_into().unwrap()) as u64;
    assert!(page_size.is_power_of_two() && (512..=65536).contains(&page_size));
    assert_eq!((length - 32) % (page_size + 24), 0);
    (length, (length - 32) / (page_size + 24))
}
fn checkpoint_setup(db: &Sqlite) {
    let connection = db.connection.lock().unwrap();
    let mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let synchronous: i64 = connection
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(synchronous, 2, "FULL durability remains configured");
    let (busy, _, _): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(busy, 0, "no competing reader or writer in this fixture");
}
fn header(db: &Sqlite) -> String {
    db.connection
        .lock()
        .unwrap()
        .query_row("SELECT data FROM work_header WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap()
}
fn publications(observation: &StageObservation) -> u64 {
    observation
        .publication_snapshot()
        .entries
        .into_iter()
        .find(|entry| {
            entry.operation == StageOperation::WorkUpdate
                && entry.category == PublicationCategory::WorkHeader
        })
        .unwrap()
        .samples
}
fn record(case: &str, iterations: u64, before: (u64, u64), after: (u64, u64), writes: u64) {
    // This is an actual bounded observation, not a latency target or presumed RED.
    assert!(writes <= iterations);
    assert!(
        after.0 >= before.0,
        "small fixture stays below automatic checkpoint threshold"
    );
    println!(
        "{}",
        serde_json::json!({"schema":"rom-sqlite-idle-wal-characterization-v1","case":case,"iterations":iterations,"wal_bytes_before":before.0,"wal_bytes_after":after.0,"frames_before":before.1,"frames_after":after.1,"header_publications":writes,"synchronous":"FULL","acceptance":false})
    );
}

#[test]
fn unchanged_empty_claims_preserve_state_and_record_actual_wal_growth() {
    let scratch = Scratch::new();
    let path = scratch.path("database");
    let db = file_fixture(&path);
    checkpoint_setup(&db);
    let before_header = header(&db);
    let before_ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    let before_epochs = db.retry_epochs().unwrap();
    let before = wal_frames(&path);
    let observation = db.observe_stages();
    for _ in 0..16 {
        assert!(db.reaction_claim_prefix(0, 32).unwrap().is_empty());
    }
    let after = wal_frames(&path);
    assert_eq!(header(&db), before_header);
    assert_eq!(
        db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before_ledger
    );
    assert_eq!(db.retry_epochs().unwrap(), before_epochs);
    record(
        "empty-claims",
        16,
        before,
        after,
        publications(&observation),
    );
    drop(db);
    let reopened = Sqlite::open(&path).unwrap();
    reopened.register(&[descriptor()]).unwrap();
    assert_eq!(header(&reopened), before_header);
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before_ledger
    );
    assert_eq!(reopened.retry_epochs().unwrap(), before_epochs);
}

#[test]
fn live_lease_idle_claims_stay_unchanged_but_expiry_reclaims_durably() {
    let scratch = Scratch::new();
    let path = scratch.path("database");
    let db = file_fixture(&path);
    let receipt = db.commit(&bundle("lease")).unwrap();
    let first = match db.reaction_update(WorkUpdate::Claim { now: 0 }).unwrap() {
        WorkResult::Claimed(claim) => claim,
        other => panic!("expected first genuine claim, got {other:?}"),
    };
    let until = match first.work.state {
        WorkState::Leased { until, .. } => until,
        _ => panic!("expected leased work"),
    };
    assert!(until > 0);
    checkpoint_setup(&db);
    let before_header = header(&db);
    let before_ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    let before = wal_frames(&path);
    let observation = db.observe_stages();
    for _ in 0..16 {
        assert_eq!(
            db.reaction_update(WorkUpdate::Claim { now: until - 1 })
                .unwrap(),
            WorkResult::Idle
        );
    }
    let after_idle = wal_frames(&path);
    assert_eq!(header(&db), before_header);
    assert_eq!(
        db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before_ledger
    );
    record(
        "live-lease-idle",
        16,
        before,
        after_idle,
        publications(&observation),
    );
    let renewed = match db
        .reaction_update(WorkUpdate::Claim { now: until })
        .unwrap()
    {
        WorkResult::Claimed(claim) => claim,
        other => panic!("expired claim must not be treated as unchanged: {other:?}"),
    };
    assert!(renewed.work.generation > first.work.generation);
    assert!(
        wal_frames(&path).1 > after_idle.1,
        "real expired-lease transition appends WAL frames"
    );
    let final_ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    assert_ne!(final_ledger, before_ledger);
    assert_eq!(
        db.receipt(&receipt.identity).unwrap(),
        Some(receipt.clone())
    );
    drop(db);
    let reopened = Sqlite::open(&path).unwrap();
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        final_ledger
    );
    assert_eq!(reopened.receipt(&receipt.identity).unwrap(), Some(receipt));
}

#[test]
fn completed_work_idle_batch_preserves_receipt_and_reopen_state() {
    let scratch = Scratch::new();
    let path = scratch.path("database");
    let db = file_fixture(&path);
    let receipt = db.commit(&bundle("done")).unwrap();
    crate::native_work_test_support::done(&db);
    checkpoint_setup(&db);
    let before_header = header(&db);
    let before_ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    assert_eq!(before_ledger.records[0].state, WorkState::Done);
    let observation = db.observe_stages();
    let before = wal_frames(&path);
    assert_eq!(
        db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 16])
            .unwrap(),
        vec![WorkResult::Idle; 16]
    );
    record(
        "done-idle-batch",
        16,
        before,
        wal_frames(&path),
        publications(&observation),
    );
    assert_eq!(header(&db), before_header);
    assert_eq!(
        db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before_ledger
    );
    assert_eq!(
        db.receipt(&receipt.identity).unwrap(),
        Some(receipt.clone())
    );
    drop(db);
    let reopened = Sqlite::open(&path).unwrap();
    assert_eq!(header(&reopened), before_header);
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before_ledger
    );
    assert_eq!(reopened.receipt(&receipt.identity).unwrap(), Some(receipt));
}
