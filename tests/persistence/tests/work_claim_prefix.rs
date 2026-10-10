//! Draft external RED fixture; install as tests/persistence/tests/work_claim_prefix.rs.
//! Intentionally requires the proposed additive Storage::reaction_claim_prefix API.
use rom::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
type Observer = Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
enum Db {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
}
impl Db {
    fn open_path(redb: bool, path: &std::path::Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
    fn canonical(&self, path: &std::path::Path) -> serde_json::Value {
        let backend = match self {
            Self::Sqlite(db) => {
                db.backup_to(path, rom_backup::BackupLimits::default())
                    .unwrap();
                rom_backup::Backend::Sqlite
            }
            Self::Redb(db) => {
                db.backup_to(path, rom_backup::BackupLimits::default())
                    .unwrap();
                rom_backup::Backend::Redb
            }
        };
        let (_, snapshot) =
            rom_backup::read(path, backend, rom_backup::BackupLimits::default()).unwrap();
        snapshot.validate().unwrap();
        serde_json::to_value(snapshot).unwrap()
    }
    fn open(redb: bool, label: &str) -> Self {
        let dir: PathBuf = std::env::temp_dir().join(format!(
            "rom-prefix-{label}-{redb}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        if redb {
            Self::Redb(rom_redb::Redb::open(dir.join("db")).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(dir.join("db")).unwrap())
        }
    }
    fn storage(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(s) => s,
            Self::Redb(s) => s,
        }
    }
    fn observe(&self, observer: Option<Observer>) {
        match self {
            Self::Sqlite(s) => s.on_commit(observer),
            Self::Redb(s) => s.on_commit(observer),
        }
    }
}
fn seed(storage: &dyn Storage, specs: &[(&str, &str, &str)], limits: ReactionLimits) {
    storage
        .register(&[Descriptor {
            kind: "prefix-source".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "enabled".into(),
                shape: Shape::Bool,
            }],
        }])
        .unwrap();
    for (id, root, kind) in specs {
        let row = Row {
            key: Key {
                kind: "prefix-source".into(),
                id: (*id).into(),
            },
            revision: 1,
            value: Some(json!({"enabled":true})),
            protected: Default::default(),
        };
        let payload = match *kind {
            "source" | "aged" | "live" | "future" => WorkPayload::Source(row.clone()),
            "action" => WorkPayload::Action(
                serde_json::to_value(Invocation {
                    retry_epoch: 0,
                    kind: "prefix-source".into(),
                    id: (*id).into(),
                    expected: Some(1),
                    idempotency: format!("action-{id}"),
                    operation: Operation::Action {
                        name: "set".into(),
                        input: json!(true),
                    },
                })
                .unwrap(),
            ),
            "notification" => WorkPayload::Notification {
                source: row.clone(),
                payload: json!({}),
            },
            _ => panic!("fixture kind"),
        };
        storage
            .commit(&Bundle {
                expected: None,
                changed: true,
                effects: vec![],
                completed_work: None,
                receipt: Receipt {
                    retry_epoch: 0,
                    replay_version: None,
                    identity: format!("seed-{id}"),
                    fingerprint: format!("seed-{id}"),
                    row,
                },
                reaction_limits: Some(limits.clone()),
                reactions: vec![PendingWork {
                    id: (*id).into(),
                    cause: Cause {
                        retry_epoch: 0,
                        root: (*root).into(),
                        parent: None,
                        depth: 1,
                        started_at: match *kind {
                            "live" => 1,
                            "future" => 2,
                            _ => 0,
                        },
                        path: vec![(*id).into()],
                    },
                    definition: "prefix-map".into(),
                    version: 1,
                    service_key: "prefix-service".into(),
                    delivery_profile: DeliveryProfile::AtLeastOnce,
                    not_before: (*kind == "aged").then_some(2),
                    payload,
                }],
            })
            .unwrap();
    }
}
#[test]
fn source_prefix_commits_distinct_roots_once_in_candidate_order() {
    for redb in [false, true] {
        let db = Db::open(redb, "distinct");
        seed(
            db.storage(),
            &[("a", "r-a", "source"), ("b", "r-b", "source")],
            ReactionLimits::default(),
        );
        let trace = Arc::new(Mutex::new(vec![]));
        let captured = trace.clone();
        db.observe(Some(Arc::new(move |point| {
            captured.lock().unwrap().push(point);
            Ok(())
        })));
        let claims = db.storage().reaction_claim_prefix(0, 32).unwrap();
        assert_eq!(
            claims
                .iter()
                .map(|claim| claim.work.pending.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert!(
            claims
                .iter()
                .all(|claim| claim.work.attempts == 1 && claim.work.generation == 1)
        );
        assert_eq!(
            trace
                .lock()
                .unwrap()
                .iter()
                .filter(|point| **point == usize::MAX)
                .count(),
            1
        );
    }
}
#[test]
fn source_prefix_inspects_same_root_delta_before_applying_it_or_skipping_past_it() {
    for redb in [false, true] {
        let db = Db::open(redb, "same-root");
        seed(
            db.storage(),
            &[
                ("a", "root", "source"),
                ("b", "root", "source"),
                ("c", "other", "source"),
            ],
            ReactionLimits::default(),
        );
        let claims = db.storage().reaction_claim_prefix(0, 32).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].work.pending.id, "a");
        let records = db.storage().reaction_records().unwrap();
        assert!(
            records[1..]
                .iter()
                .all(|record| record.state == WorkState::Pending
                    && record.attempts == 0
                    && record.generation == 0)
        );
    }
}
#[test]
fn source_prefix_never_commits_later_action_or_notification_barriers() {
    for redb in [false, true] {
        for kind in ["action", "notification"] {
            let db = Db::open(redb, kind);
            seed(
                db.storage(),
                &[("a", "a", "source"), ("b", "b", kind), ("c", "c", "source")],
                ReactionLimits::default(),
            );
            assert_eq!(db.storage().reaction_claim_prefix(0, 32).unwrap().len(), 1);
            assert!(
                db.storage().reaction_records().unwrap()[1..]
                    .iter()
                    .all(|r| r.state == WorkState::Pending && r.attempts == 0 && r.generation == 0)
            );
        }
    }
}
#[test]
fn first_non_source_claim_routes_as_exact_singleton() {
    for redb in [false, true] {
        for kind in ["action", "notification"] {
            let db = Db::open(redb, &format!("first-{kind}"));
            seed(
                db.storage(),
                &[("a", "a", kind), ("b", "b", "source")],
                ReactionLimits::default(),
            );
            let claims = db.storage().reaction_claim_prefix(0, 32).unwrap();
            assert_eq!(claims.len(), 1);
            assert_eq!(claims[0].work.pending.id, "a");
            assert_eq!(db.storage().reaction_records().unwrap()[1].attempts, 0);
        }
    }
}
#[test]
fn source_prefix_does_not_charge_the_same_root_budget_in_advance() {
    for redb in [false, true] {
        let db = Db::open(redb, "budget");
        seed(
            db.storage(),
            &[("a", "root", "source"), ("b", "root", "source")],
            ReactionLimits {
                max_work: 1,
                ..ReactionLimits::default()
            },
        );
        assert_eq!(db.storage().reaction_claim_prefix(0, 32).unwrap().len(), 1);
        let b = db.storage().reaction_records().unwrap().remove(1);
        assert_eq!(
            (b.attempts, b.generation, b.state),
            (0, 0, WorkState::Pending)
        );
        let WorkResult::Claimed(next) = db
            .storage()
            .reaction_update(WorkUpdate::Claim { now: 0 })
            .unwrap()
        else {
            panic!("resolution")
        };
        assert!(next.resolution_only);
        assert_eq!(next.stop_reason, Some(StopReason::WorkBudget));
    }
}
#[test]
fn changed_first_preserves_expiry_effects_and_returns_no_claim() {
    for redb in [false, true] {
        let db = Db::open(redb, "changed");
        seed(
            db.storage(),
            &[("a", "root", "aged")],
            ReactionLimits {
                max_age_seconds: 1,
                ..ReactionLimits::default()
            },
        );
        assert!(
            db.storage()
                .reaction_claim_prefix(1, 32)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            db.storage().reaction_records().unwrap()[0].state,
            WorkState::Stopped(StopReason::Age)
        );
    }
}
#[test]
fn aged_candidate_prefix_effects_do_not_hide_a_later_currently_live_source() {
    for redb in [false, true] {
        let db = Db::open(redb, "age-plus-live");
        seed(
            db.storage(),
            &[("a", "aged", "aged"), ("b", "live", "live")],
            ReactionLimits {
                max_age_seconds: 1,
                ..ReactionLimits::default()
            },
        );
        let claims = db.storage().reaction_claim_prefix(1, 32).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].work.pending.id, "b");
        assert!(!claims[0].resolution_only);
        assert_eq!(
            db.storage().reaction_records().unwrap()[0].state,
            WorkState::Stopped(StopReason::Age)
        );
    }
}
#[test]
fn changed_first_then_future_source_keeps_later_call_recovery_without_spinning() {
    for redb in [false, true] {
        let db = Db::open(redb, "changed-then-future");
        seed(
            db.storage(),
            &[("a", "aged", "aged"), ("b", "future", "future")],
            ReactionLimits {
                max_age_seconds: 1,
                ..ReactionLimits::default()
            },
        );
        assert!(
            db.storage()
                .reaction_claim_prefix(1, 32)
                .unwrap()
                .is_empty()
        );
        let records = db.storage().reaction_records().unwrap();
        assert_eq!(records[0].state, WorkState::Stopped(StopReason::Age));
        assert_eq!(records[1].state, WorkState::Pending);
        assert_eq!(records[1].attempts, 0);
        let next = db.storage().reaction_claim_prefix(2, 32).unwrap();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].work.pending.id, "b");
        assert!(!next[0].resolution_only);
    }
}
#[test]
fn claim_prefix_unknown_acknowledgement_returns_no_dispatchable_claims() {
    for redb in [false, true] {
        let db = Db::open(redb, "unknown");
        seed(
            db.storage(),
            &[("a", "a", "source"), ("b", "b", "source")],
            ReactionLimits::default(),
        );
        db.observe(Some(Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.storage().reaction_claim_prefix(0, 32),
            Err(Error::Unknown)
        );
        db.observe(None);
        assert!(
            db.storage()
                .reaction_records()
                .unwrap()
                .iter()
                .all(|r| r.attempts == 1 && matches!(r.state, WorkState::Leased { .. }))
        );
    }
}
#[test]
fn claim_prefix_rejects_zero_and_over_32_without_claiming() {
    for redb in [false, true] {
        let db = Db::open(redb, "bounds");
        seed(
            db.storage(),
            &[("a", "a", "source")],
            ReactionLimits::default(),
        );
        assert_eq!(
            db.storage().reaction_claim_prefix(0, 0),
            Err(Error::TooLarge)
        );
        assert_eq!(
            db.storage().reaction_claim_prefix(0, 33),
            Err(Error::TooLarge)
        );
        assert_eq!(db.storage().reaction_records().unwrap()[0].attempts, 0);
    }
}

fn fault_directory(redb: bool, label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rom-prefix-fault-{label}-{redb}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}
#[test]
fn every_prefix_publication_and_precommit_failure_preserves_full_snapshot_live_and_reopened() {
    for redb in [false, true] {
        let dir = fault_directory(redb, "trace");
        let db = Db::open_path(redb, &dir.join("db"));
        seed(
            db.storage(),
            &[("a", "a", "source"), ("b", "b", "source")],
            ReactionLimits::default(),
        );
        let trace = Arc::new(Mutex::new(vec![]));
        let observed = trace.clone();
        db.observe(Some(Arc::new(move |point| {
            observed.lock().unwrap().push(point);
            Ok(())
        })));
        assert_eq!(db.storage().reaction_claim_prefix(0, 2).unwrap().len(), 2);
        db.observe(None);
        let trace = trace.lock().unwrap().clone();
        assert_eq!(trace.iter().filter(|p| **p == 0).count(), 1);
        assert_eq!(trace.iter().filter(|p| **p == usize::MAX).count(), 1);
        let publications: Vec<_> = trace
            .iter()
            .copied()
            .filter(|p| *p != 0 && *p != usize::MAX)
            .collect();
        assert_eq!(publications, (1..=publications.len()).collect::<Vec<_>>());
        for point in trace.into_iter().filter(|p| *p != usize::MAX) {
            let dir = fault_directory(redb, &point.to_string());
            let path = dir.join("db");
            let db = Db::open_path(redb, &path);
            seed(
                db.storage(),
                &[("a", "a", "source"), ("b", "b", "source")],
                ReactionLimits::default(),
            );
            let before = db.canonical(&dir.join("before.backup"));
            db.observe(Some(Arc::new(move |current| {
                if current == point {
                    Err(Error::Storage)
                } else {
                    Ok(())
                }
            })));
            assert_eq!(
                db.storage().reaction_claim_prefix(0, 2),
                Err(Error::NotCommitted)
            );
            db.observe(None);
            assert_eq!(
                db.canonical(&dir.join("live.backup")),
                before,
                "{redb} checkpoint {point}"
            );
            drop(db);
            let reopened = Db::open_path(redb, &path);
            assert_eq!(
                reopened.canonical(&dir.join("reopened.backup")),
                before,
                "{redb} reopen checkpoint {point}"
            );
        }
    }
}
#[test]
fn prefix_unknown_ack_persists_exact_claim_generations_and_full_snapshot_across_reopen() {
    for redb in [false, true] {
        let dir = fault_directory(redb, "unknown");
        let path = dir.join("db");
        let db = Db::open_path(redb, &path);
        seed(
            db.storage(),
            &[("a", "a", "source"), ("b", "b", "source")],
            ReactionLimits::default(),
        );
        db.observe(Some(Arc::new(|point| {
            if point == usize::MAX {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            db.storage().reaction_claim_prefix(0, 2),
            Err(Error::Unknown)
        );
        db.observe(None);
        let records = db.storage().reaction_records().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.attempts == 1
            && r.generation == 1
            && matches!(r.state, WorkState::Leased { generation: 1, .. })));
        let committed = db.canonical(&dir.join("committed.backup"));
        drop(db);
        let reopened = Db::open_path(redb, &path);
        assert_eq!(reopened.canonical(&dir.join("reopened.backup")), committed);
        assert_eq!(reopened.storage().reaction_records().unwrap(), records);
    }
}
