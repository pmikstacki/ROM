//! Actual format-11 atomic batch transactions through the public Storage API.
#[path = "support/work_atomic_batch_contract.rs"]
mod contract;
#[path = "support/work_atomic_batch_fixture.rs"]
mod fixture;
use rom::{Error, Storage, WorkResult, WorkState, WorkUpdate};
use rom_backup::{Backend, BackupLimits};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

enum Db {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
}
impl Db {
    fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
    fn storage(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(s) => s,
            Self::Redb(s) => s,
        }
    }
    fn observe(&self, observer: Option<Arc<dyn Fn(usize) -> rom::Result<()> + Send + Sync>>) {
        match self {
            Self::Sqlite(s) => s.on_commit(observer),
            Self::Redb(s) => s.on_commit(observer),
        }
    }
    fn canonical(&self, path: &Path) -> serde_json::Value {
        let backend = match self {
            Self::Sqlite(s) => {
                s.backup_to(path, BackupLimits::default()).unwrap();
                Backend::Sqlite
            }
            Self::Redb(s) => {
                s.backup_to(path, BackupLimits::default()).unwrap();
                Backend::Redb
            }
        };
        let (_, snapshot) = rom_backup::read(path, backend, BackupLimits::default()).unwrap();
        snapshot.validate().unwrap();
        serde_json::to_value(snapshot).unwrap()
    }
}
fn directory(label: &str, redb: bool) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rom-atomic-batch-{label}-{redb}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
fn claims() -> Vec<WorkUpdate> {
    vec![WorkUpdate::Claim { now: 0 }; 2]
}
fn materialize_with_child(claim: &rom::WorkClaim) -> WorkUpdate {
    let mut cause = claim.work.pending.cause.clone();
    cause.parent = Some(claim.work.pending.id.clone());
    cause.path.push("0".into());
    let id = format!("{}-child", claim.work.pending.id);
    let invocation = rom::Invocation {
        retry_epoch: cause.retry_epoch,
        kind: "batch-source".into(),
        id: "b".into(),
        expected: Some(1),
        idempotency: id.clone(),
        operation: rom::Operation::Action {
            name: "set".into(),
            input: rom::json!(true),
        },
    };
    WorkUpdate::Materialize {
        claim: claim.key(),
        now: 0,
        children: vec![rom::PendingWork {
            id,
            cause,
            definition: claim.work.pending.definition.clone(),
            version: claim.work.pending.version,
            service_key: claim.work.pending.service_key.clone(),
            delivery_profile: rom::DeliveryProfile::AtLeastOnce,
            not_before: None,
            payload: rom::WorkPayload::Action(serde_json::to_value(invocation).unwrap()),
        }],
    }
}

#[test]
fn atomic_materialization_ack_loss_persists_all_children_and_parent_completions() {
    for redb in [false, true] {
        let dir = directory("materialize-unknown", redb);
        let path = dir.join("db");
        let db = Db::open(redb, &path);
        fixture::seed(db.storage());
        let upstream = db.storage().journal_head("batch-source").unwrap();
        let updates = db
            .storage()
            .reaction_updates_atomic(claims())
            .unwrap()
            .into_iter()
            .map(|result| match result {
                WorkResult::Claimed(claim) => materialize_with_child(&claim),
                other => panic!("claim: {other:?}"),
            })
            .collect();
        db.observe(Some(Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.storage().reaction_updates_atomic(updates),
            Err(Error::Unknown)
        );
        drop(db);
        let reopened = Db::open(redb, &path);
        let records = reopened.storage().reaction_records().unwrap();
        assert_eq!(records.len(), 4);
        assert_eq!(
            records
                .iter()
                .filter(|r| r.state == WorkState::Done && r.attempts == 1)
                .count(),
            2
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.state == WorkState::Pending && r.attempts == 0)
                .count(),
            2
        );
        assert_eq!(
            reopened.storage().journal_head("batch-source").unwrap(),
            upstream
        );
        reopened.canonical(&dir.join("resolved.rombk"));
    }
}

#[test]
fn atomic_empty_and_oversized_count_have_no_publications() {
    for redb in [false, true] {
        let dir = directory("admission", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        let before = db.canonical(&dir.join("before.rombk"));
        db.observe(Some(Arc::new(|_| Err(Error::Storage))));
        assert_eq!(db.storage().reaction_updates_atomic(vec![]), Ok(vec![]));
        assert_eq!(
            db.storage()
                .reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 33]),
            Err(Error::TooLarge)
        );
        db.observe(None);
        assert_eq!(db.canonical(&dir.join("after.rombk")), before);
    }
}

#[test]
fn atomic_input_byte_overflow_rejects_before_the_first_claim() {
    for redb in [false, true] {
        let dir = directory("bytes", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        let before = db.canonical(&dir.join("before.rombk"));
        let oversized = WorkUpdate::Finish {
            claim: rom::ClaimKey {
                id: "x".repeat(fixture::limits().max_bytes),
                generation: 1,
            },
            now: 0,
            outcome: rom::WorkOutcome::Done,
        };
        assert_eq!(
            db.storage()
                .reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }, oversized]),
            Err(Error::TooLarge)
        );
        assert_eq!(db.canonical(&dir.join("after.rombk")), before);
    }
}

#[test]
fn atomic_exact_32_updates_observe_prior_claims_without_reclaiming() {
    for redb in [false, true] {
        let dir = directory("count32", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        let results = db
            .storage()
            .reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 32])
            .unwrap();
        assert_eq!(results.len(), 32);
        assert!(
            results[..2]
                .iter()
                .all(|result| matches!(result, WorkResult::Claimed(_)))
        );
        assert!(
            results[2..]
                .iter()
                .all(|result| *result == WorkResult::Idle)
        );
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|record| record.attempts == 1 && record.revision == 1)
        );
    }
}

#[test]
fn atomic_second_transition_failure_preserves_full_canonical_archive_after_reopen() {
    for redb in [false, true] {
        let dir = directory("result-rollback", redb);
        let path = dir.join("db");
        let db = Db::open(redb, &path);
        fixture::seed(db.storage());
        let results = db.storage().reaction_updates_atomic(claims()).unwrap();
        let keys: Vec<_> = results
            .into_iter()
            .map(|result| match result {
                WorkResult::Claimed(claim) => claim.key(),
                other => panic!("claim: {other:?}"),
            })
            .collect();
        let before = db.canonical(&dir.join("before.rombk"));
        assert_eq!(
            db.storage().reaction_updates_atomic(vec![
                WorkUpdate::Materialize {
                    claim: keys[0].clone(),
                    now: 0,
                    children: vec![]
                },
                WorkUpdate::Materialize {
                    claim: rom::ClaimKey {
                        id: keys[1].id.clone(),
                        generation: 0
                    },
                    now: 0,
                    children: vec![]
                },
            ]),
            Err(Error::Conflict)
        );
        assert_eq!(db.canonical(&dir.join("after-live.rombk")), before);
        drop(db);
        let reopened = Db::open(redb, &path);
        assert_eq!(reopened.canonical(&dir.join("after-reopen.rombk")), before);
    }
}

#[test]
fn atomic_ordered_claims_match_sequential_coherent_image() {
    for redb in [false, true] {
        let dir = directory("oracle", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        contract::assert_atomic_claims_match_sequential_image(db.storage(), 0, &fixture::limits());
    }
}
#[test]
fn atomic_second_failure_rolls_back_while_singleton_keeps_prefix() {
    for redb in [false, true] {
        let dir = directory("prefix", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        contract::assert_atomic_results_rollback_and_singleton_prefix(db.storage(), 0);
    }
}
#[test]
fn atomic_batch_has_one_commit_and_global_publication_ordinals() {
    for redb in [false, true] {
        let dir = directory("ordinals", redb);
        let db = Db::open(redb, &dir.join("db"));
        fixture::seed(db.storage());
        let trace = Arc::new(Mutex::new(vec![]));
        let observed = trace.clone();
        db.observe(Some(Arc::new(move |point| {
            observed.lock().unwrap().push(point);
            Ok(())
        })));
        let results = db.storage().reaction_updates_atomic(claims()).unwrap();
        assert_eq!(results.len(), 2);
        let trace = trace.lock().unwrap();
        let writes: Vec<_> = trace
            .iter()
            .copied()
            .filter(|p| *p != 0 && *p != usize::MAX)
            .collect();
        assert_eq!(writes, (1..=writes.len()).collect::<Vec<_>>());
        assert_eq!(trace.iter().filter(|p| **p == 0).count(), 1);
        assert_eq!(trace.iter().filter(|p| **p == usize::MAX).count(), 1);
        assert_eq!(trace.last(), Some(&usize::MAX));
    }
}
#[test]
fn atomic_every_publication_fault_and_precommit_leave_full_archive_unchanged_after_reopen() {
    for redb in [false, true] {
        let probe_dir = directory("fault-probe", redb);
        let probe = Db::open(redb, &probe_dir.join("db"));
        fixture::seed(probe.storage());
        let trace = Arc::new(Mutex::new(vec![]));
        let observed = trace.clone();
        probe.observe(Some(Arc::new(move |point| {
            observed.lock().unwrap().push(point);
            Ok(())
        })));
        probe.storage().reaction_updates_atomic(claims()).unwrap();
        let points: Vec<_> = trace
            .lock()
            .unwrap()
            .iter()
            .copied()
            .filter(|p| *p != usize::MAX)
            .collect();
        for fail in points {
            let dir = directory(&format!("fault-{fail}"), redb);
            let path = dir.join("db");
            let db = Db::open(redb, &path);
            fixture::seed(db.storage());
            let before = db.canonical(&dir.join("before.rombk"));
            db.observe(Some(Arc::new(move |point| {
                if point == fail {
                    Err(Error::Storage)
                } else {
                    Ok(())
                }
            })));
            assert_eq!(
                db.storage().reaction_updates_atomic(claims()),
                Err(Error::NotCommitted)
            );
            db.observe(None);
            assert_eq!(db.canonical(&dir.join("after-live.rombk")), before);
            drop(db);
            let reopened = Db::open(redb, &path);
            assert_eq!(reopened.canonical(&dir.join("after-reopen.rombk")), before);
        }
    }
}
#[test]
fn atomic_acknowledgement_loss_preserves_complete_claims_after_reopen() {
    for redb in [false, true] {
        let dir = directory("unknown", redb);
        let path = dir.join("db");
        let db = Db::open(redb, &path);
        fixture::seed(db.storage());
        let upstream = db.storage().journal_head("batch-source").unwrap();
        db.observe(Some(Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.storage().reaction_updates_atomic(claims()),
            Err(Error::Unknown)
        );
        drop(db);
        let reopened = Db::open(redb, &path);
        let records = reopened.storage().reaction_records().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.attempts == 1
            && r.generation == 1
            && matches!(r.state, WorkState::Leased { .. })));
        assert_eq!(
            reopened.storage().journal_head("batch-source").unwrap(),
            upstream
        );
        reopened.canonical(&dir.join("resolved.rombk"));
        // Resolve the committed claims by their stored generations, without claiming again.
        let updates = records
            .into_iter()
            .map(|r| WorkUpdate::Materialize {
                claim: rom::ClaimKey {
                    id: r.pending.id,
                    generation: r.generation,
                },
                now: 0,
                children: vec![],
            })
            .collect();
        assert_eq!(
            reopened.storage().reaction_updates_atomic(updates),
            Ok(vec![WorkResult::Changed; 2])
        );
        assert!(
            reopened
                .storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.state == WorkState::Done && r.attempts == 1)
        );
    }
}
