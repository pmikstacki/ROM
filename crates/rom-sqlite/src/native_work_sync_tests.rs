//! Explicit controlled-filesystem measurements; never ordinary test performance acceptance.
use crate::native_work_test_support::{bundle, descriptor, file_fixture};
use crate::{Sqlite, StageObservation, StageOperation, StorageStage};
use rom::{Error, Storage, WorkClaim, WorkResult, WorkState, WorkUpdate};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

struct Probe(PathBuf);
impl Probe {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(
            std::env::var_os("ROM_SQLITE_SYNC_PROBE_ROOT")
                .expect("ROOT-controlled probe root required; no temporary-directory fallback"),
        );
        let backing =
            std::path::Path::new("/var/tmp/rom-010-authentik-20261007/run/volume/private");
        assert!(root.starts_with(backing) && root != backing);
        assert_eq!(root.canonicalize().unwrap(), root);
        let metadata = std::fs::metadata(&root).unwrap();
        let parent = std::fs::metadata(backing).unwrap();
        assert!(metadata.is_dir());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        let expected_device: u64 = std::env::var("ROM_SQLITE_SYNC_BACKING_DEVICE")
            .expect("ROOT-pinned actual mixed backing device required")
            .parse()
            .unwrap();
        assert_eq!(metadata.dev(), expected_device);
        assert_eq!(
            std::fs::metadata("/root/ROM/.superpowers").unwrap().dev(),
            expected_device,
            "actual mixed backing source device"
        );
        assert_eq!(metadata.uid(), parent.uid());
        let path = root.join(case);
        std::fs::create_dir(&path).expect("exclusive new probe case required");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().dev(), expected_device);
        Self(path)
    }
    fn database(&self) -> PathBuf {
        self.0.join("database")
    }
}
// Evidence directories intentionally survive success and failure for ROOT review.

fn setup(db: &Sqlite) {
    let connection = db.connection.lock().unwrap();
    let mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let synchronous: i64 = connection
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(synchronous, 2);
    let (busy, _, _): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(busy, 0);
}
fn wal_counts(db: &Sqlite, probe: &Probe) -> (u64, u64) {
    let files = std::fs::read_dir(&probe.0)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(files.len() <= 8);
    let mut total_bytes = 0u64;
    for file in files {
        let metadata = file.metadata().unwrap();
        assert!(metadata.is_file());
        total_bytes = total_bytes.checked_add(metadata.len()).unwrap();
    }
    assert!(
        total_bytes <= 4 * 1024 * 1024,
        "bounded complete probe case"
    );
    let bytes = match std::fs::metadata(probe.0.join("database-wal")) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => panic!("WAL metadata: {error}"),
    };
    assert!(bytes <= 4 * 1024 * 1024);
    if bytes == 0 {
        return (0, 0);
    }
    let page_size: u64 = db
        .connection
        .lock()
        .unwrap()
        .pragma_query_value(None, "page_size", |row| row.get::<_, i64>(0))
        .unwrap()
        .try_into()
        .unwrap();
    assert!(bytes >= 32);
    assert_eq!((bytes - 32) % (page_size + 24), 0);
    (bytes, (bytes - 32) / (page_size + 24))
}
fn now_ns() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}
fn report(
    case: &str,
    operation: StageOperation,
    observation: &StageObservation,
    before: (u64, u64),
    after: (u64, u64),
    elapsed: std::time::Duration,
    phase: (String, String),
) -> u64 {
    let snapshot = observation.snapshot();
    assert!(!snapshot.poison_recovered);
    assert!(
        snapshot
            .entries
            .iter()
            .all(|entry| entry.dropped_samples == 0 && !entry.saturated)
    );
    let commit = snapshot
        .entries
        .iter()
        .find(|entry| entry.operation == operation && entry.stage == StorageStage::NativeCommit)
        .unwrap();
    assert!(
        after.1 > before.1,
        "genuine changed transaction publishes WAL frames"
    );
    println!(
        "{}",
        serde_json::json!({"schema":"rom-sqlite-changed-sync-characterization-v1","case":case,"operations":16,"native_commits":commit.samples,"native_commit_ns":commit.elapsed_ns,"elapsed_ns":elapsed.as_nanos().to_string(),"phase_start_unix_ns":phase.0,"phase_end_unix_ns":phase.1,"wal_bytes_before":before.0,"wal_bytes_after":after.0,"frames_before":before.1,"frames_after":after.1,"synchronous":"FULL","acceptance":false})
    );
    commit.samples
}
fn claimed_fixture(case: &str) -> (Probe, Sqlite, Vec<WorkClaim>) {
    let probe = Probe::new(case);
    let db = file_fixture(&probe.database());
    for index in 0..16 {
        db.commit(&bundle(&format!("{index:02}"))).unwrap();
    }
    let claims = db.reaction_claim_prefix(0, 16).unwrap();
    assert_eq!(claims.len(), 16);
    setup(&db);
    (probe, db, claims)
}
fn materialize(claim: &WorkClaim) -> WorkUpdate {
    WorkUpdate::Materialize {
        claim: claim.key(),
        now: 0,
        children: vec![],
    }
}

#[test]
#[ignore = "requires ROOT-controlled mixed-backing probe directory"]
fn changed_work_singletons_and_atomic_batch_preserve_same_durable_result() {
    let mut ledgers = Vec::new();
    for (case, grouped) in [("work-singleton16", false), ("work-atomic16", true)] {
        let (probe, db, claims) = claimed_fixture(case);
        let before = wal_counts(&db, &probe);
        let observation = db.observe_stages();
        let start = now_ns();
        let timer = Instant::now();
        let results = if grouped {
            db.reaction_updates_atomic(claims.iter().map(materialize).collect())
                .unwrap()
        } else {
            claims
                .iter()
                .map(|claim| db.reaction_update(materialize(claim)).unwrap())
                .collect()
        };
        let elapsed = timer.elapsed();
        let end = now_ns();
        assert_eq!(results, vec![WorkResult::Changed; 16]);
        let after = wal_counts(&db, &probe);
        assert_eq!(
            report(
                case,
                StageOperation::WorkUpdate,
                &observation,
                before,
                after,
                elapsed,
                (start, end)
            ),
            if grouped { 1 } else { 16 }
        );
        let ledger = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
        assert_eq!(ledger.records.len(), 16);
        assert!(
            ledger
                .records
                .iter()
                .all(|record| record.state == WorkState::Done)
        );
        let receipts = (0..16)
            .map(|index| db.receipt(&format!("create-{index:02}")).unwrap())
            .collect::<Vec<_>>();
        assert!(receipts.iter().all(Option::is_some));
        drop(db);
        let reopened = Sqlite::open(probe.database()).unwrap();
        assert_eq!(
            reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
            ledger
        );
        for (index, receipt) in receipts.into_iter().enumerate() {
            assert_eq!(
                reopened.receipt(&format!("create-{index:02}")).unwrap(),
                receipt
            );
        }
        ledgers.push(ledger);
        drop(reopened);
    }
    // Independent databases have distinct ledger identities; each exact identity
    // was checked across reopen above. Their durable Work semantics must match.
    assert_ne!(ledgers[0].generation, ledgers[1].generation);
    assert_eq!(ledgers[0].records, ledgers[1].records);
    assert_eq!(ledgers[0].roots, ledgers[1].roots);
    assert_eq!(ledgers[0].retry_epochs, ledgers[1].retry_epochs);
    assert_eq!(ledgers[0].operator, ledgers[1].operator);
    assert_eq!(ledgers[0].limits, ledgers[1].limits);
}

#[test]
#[ignore = "requires ROOT-controlled mixed-backing probe directory"]
fn sixteen_resource_bundles_reopen_with_exact_receipts_and_events() {
    let probe = Probe::new("resource16");
    let db = file_fixture(&probe.database());
    setup(&db);
    let before = wal_counts(&db, &probe);
    let observation = db.observe_stages();
    let start = now_ns();
    let timer = Instant::now();
    let receipts = (0..16)
        .map(|index| db.commit(&bundle(&format!("{index:02}"))).unwrap())
        .collect::<Vec<_>>();
    let elapsed = timer.elapsed();
    let end = now_ns();
    assert_eq!(
        report(
            "resource16",
            StageOperation::Commit,
            &observation,
            before,
            wal_counts(&db, &probe),
            elapsed,
            (start, end)
        ),
        16
    );
    let events = db.journal("native-work", None, 32, 1024 * 1024).unwrap();
    assert_eq!(events.events.len(), 16);
    drop(db);
    let reopened = Sqlite::open(probe.database()).unwrap();
    reopened.register(&[descriptor()]).unwrap();
    for receipt in receipts {
        assert_eq!(
            reopened.receipt(&receipt.identity).unwrap(),
            Some(receipt.clone())
        );
        assert_eq!(reopened.load(&receipt.row.key).unwrap(), Some(receipt.row));
    }
    assert_eq!(
        reopened
            .journal("native-work", None, 32, 1024 * 1024)
            .unwrap(),
        events
    );
}

#[test]
#[ignore = "requires ROOT-controlled mixed-backing probe directory"]
fn changed_batch_precommit_failure_rolls_back_all_sixteen_transitions() {
    let (probe, db, claims) = claimed_fixture("rollback16");
    let before = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    db.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == 0 {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    assert_eq!(
        db.reaction_updates_atomic(claims.iter().map(materialize).collect()),
        Err(Error::NotCommitted)
    );
    db.on_commit(None);
    assert_eq!(db.work_snapshot(4096, 16 * 1024 * 1024).unwrap(), before);
    drop(db);
    let reopened = Sqlite::open(probe.database()).unwrap();
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        before
    );
}

#[test]
#[ignore = "requires ROOT-controlled mixed-backing probe directory"]
fn changed_batch_ack_failure_remains_unknown_and_reopens_committed_state() {
    let (probe, db, claims) = claimed_fixture("unknown16");
    db.on_commit(Some(Arc::new(|ordinal| {
        if ordinal == usize::MAX {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    assert_eq!(
        db.reaction_updates_atomic(claims.iter().map(materialize).collect()),
        Err(Error::Unknown)
    );
    db.on_commit(None);
    let committed = db.work_snapshot(4096, 16 * 1024 * 1024).unwrap();
    assert_eq!(committed.records.len(), 16);
    assert!(
        committed
            .records
            .iter()
            .all(|record| record.state == WorkState::Done)
    );
    drop(db);
    let reopened = Sqlite::open(probe.database()).unwrap();
    assert_eq!(
        reopened.work_snapshot(4096, 16 * 1024 * 1024).unwrap(),
        committed
    );
}
