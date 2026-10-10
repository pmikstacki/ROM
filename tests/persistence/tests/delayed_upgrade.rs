//! Populated operator-aware format-8 upgrade into a fresh current database.
#[path = "support/legacy_native.rs"]
mod legacy_native;
use redb::ReadableTable;
use rom::operator::{WorkControlOperation, WorkControlRequest, WorkHandle};
use rom::*;
use rom_backup::{Backend, BackupLimits};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "upgrade-delayed-items")]
struct Item {
    name: String,
}
enum Native {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
}
impl Native {
    fn open(redb: bool, path: &Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        }
    }
    fn store(&self) -> &dyn Storage {
        match self {
            Self::Sqlite(s) => s,
            Self::Redb(s) => s,
        }
    }
    fn backup(&self, path: &Path) {
        match self {
            Self::Sqlite(s) => s.backup_to(path, BackupLimits::default()).unwrap(),
            Self::Redb(s) => s.backup_to(path, BackupLimits::default()).unwrap(),
        };
    }
    fn upgrade(redb: bool, source: &Path, destination: &Path) -> Result<Self> {
        if redb {
            rom_redb::Redb::upgrade_from(
                source,
                destination,
                &[Item::descriptor()],
                BackupLimits::default(),
            )
            .map(Self::Redb)
        } else {
            rom_sqlite::Sqlite::upgrade_from(
                source,
                destination,
                &[Item::descriptor()],
                BackupLimits::default(),
            )
            .map(Self::Sqlite)
        }
    }
}
fn directory(redb: bool) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rom-delayed-upgrade-{}-{redb}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
fn pending(id: &str, row: &Row) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            retry_epoch: 0,
            root: id.into(),
            parent: None,
            depth: 0,
            started_at: 10,
            path: vec![],
        },
        definition: "mail".into(),
        version: 1,
        service_key: "service".into(),
        delivery_profile: DeliveryProfile::AtLeastOnce,
        not_before: None,
        payload: WorkPayload::Notification {
            source: row.clone(),
            payload: json!({"name":"retained"}),
        },
    }
}
fn populate(store: &dyn Storage) -> (WorkClaim, WorkControlReceipt) {
    store.register(&[Item::descriptor()]).unwrap();
    let row = Row {
        key: Key {
            kind: Item::KIND.into(),
            id: "live".into(),
        },
        revision: 1,
        value: Some(
            Item {
                name: "original".into(),
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    store
        .commit(&Bundle {
            expected: None,
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: Some(1),
                identity: "create-live".into(),
                fingerprint: "original-request".into(),
                row: row.clone(),
            },
            changed: true,
            effects: vec![Intent::new("audit", json!({"name":"original"}))],
            reactions: vec![pending("a-active", &row), pending("b-pending", &row)],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
    let tombstone = Row {
        key: Key {
            kind: Item::KIND.into(),
            id: "deleted".into(),
        },
        revision: 1,
        value: None,
        protected: Default::default(),
    };
    store
        .commit(&Bundle {
            expected: None,
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: Some(1),
                identity: "tombstone".into(),
                fingerprint: "delete-request".into(),
                row: tombstone,
            },
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
    let WorkResult::Claimed(first) = store
        .reaction_update(WorkUpdate::Claim { now: 10 })
        .unwrap()
    else {
        panic!()
    };
    store
        .reaction_update(WorkUpdate::Finish {
            claim: first.key(),
            now: 10,
            outcome: WorkOutcome::Stop(StopReason::Denied),
        })
        .unwrap();
    let view = store.work_snapshot(16, 65536).unwrap();
    let control = StorageWorkControl {
        principal: "operator".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("a-active"),
            expected: view.version(&view.records[0]),
            key: "operator-retry".into(),
            retry_epoch: 0,
            operation: WorkControlOperation::Retry,
        },
        decision: WorkControlDecision::Retry,
        now: 11,
    };
    let receipt = store.control_work(&control).unwrap();
    let WorkResult::Claimed(active) = store
        .reaction_update(WorkUpdate::Claim { now: 11 })
        .unwrap()
    else {
        panic!()
    };
    store
        .reaction_update(WorkUpdate::DeliveryStarted {
            claim: active.key(),
            now: 11,
        })
        .unwrap();
    (*active, receipt)
}
/// No scheduling fields are emitted for None. All tables and operator data match format 8.
fn predecessor_marker(redb: bool, path: &Path) {
    legacy_native::mark(redb, path, 8, false);
}
#[test]
fn populated_format_eight_upgrade_preserves_source_receipts_work_and_operator_history() {
    for redb in [false, true] {
        let root = directory(redb);
        let source = root.join("format8");
        let destination = root.join("format9");
        let before_archive = root.join("before.rombk");
        let after_archive = root.join("after.rombk");
        let backend = if redb { Backend::Redb } else { Backend::Sqlite };
        let store = Native::open(redb, &source);
        let (claim, operator) = populate(store.store());
        let old_cursor = store.store().journal_head(Item::KIND).unwrap();
        store.backup(&before_archive);
        drop(store);
        predecessor_marker(redb, &source);
        let original = std::fs::read(&source).unwrap();
        assert!(match redb {
            true => matches!(rom_redb::Redb::open(&source), Err(Error::Unsupported(_))),
            false => matches!(
                rom_sqlite::Sqlite::open(&source),
                Err(Error::Unsupported(_))
            ),
        });
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let upgraded = Native::upgrade(redb, &source, &destination).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(matches!(
            upgraded
                .store()
                .journal(Item::KIND, Some(&old_cursor), 16, 65536),
            Err(Error::HistoryGap)
        ));
        assert_eq!(
            upgraded
                .store()
                .reaction_update(WorkUpdate::DeliveryFinished {
                    claim: claim.key(),
                    now: 12,
                    outcome: DeliveryOutcome::Accepted
                }),
            Err(Error::Conflict)
        );
        upgraded.backup(&after_archive);
        let (_, before) =
            rom_backup::read(&before_archive, backend, BackupLimits::default()).unwrap();
        let (manifest, after) =
            rom_backup::read(&after_archive, backend, BackupLimits::default()).unwrap();
        assert_eq!((manifest.archive_version, manifest.storage_format), (7, 11));
        assert_eq!(manifest.operator_receipts, 1);
        assert_eq!(after.rows, before.rows);
        assert_eq!(after.receipts, before.receipts);
        assert_eq!(after.events, before.events);
        assert_eq!(
            serde_json::to_value(&after.effects).unwrap(),
            serde_json::to_value(&before.effects).unwrap()
        );
        assert_eq!(after.descriptors, before.descriptors);
        assert_eq!(after.references, before.references);
        let before_records = before.state.work.records();
        let after_records = after.state.work.records();
        for (old, new) in before_records.iter().zip(&after_records) {
            assert_eq!(new.pending, old.pending);
            assert_eq!(new.attempts, old.attempts);
            if matches!(old.state, WorkState::Leased { .. }) {
                assert!(new.generation > old.generation);
            } else {
                assert_eq!(new, old);
            }
        }
        let before_wire = serde_json::to_value(&before.state).unwrap();
        let after_wire = serde_json::to_value(&after.state).unwrap();
        assert_eq!(after_wire["operator"], before_wire["operator"]);
        assert_eq!(after_wire["work"]["roots"], before_wire["work"]["roots"]);
        assert_eq!(operator.request.key, "operator-retry");
        println!(
            "delayed-upgrade-evidence {}",
            json!({"adapter":if redb {"redb"} else {"sqlite"}, "source":source, "destination":destination, "before_archive":before_archive, "after_archive":after_archive})
        );
    }
}
fn inject_legacy_timing(redb: bool, path: &Path, location: &str) {
    if redb {
        let db = redb::Database::open(path).unwrap();
        let tx = db.begin_write().unwrap();
        if location == "pending" {
            let mut table = tx
                .open_table(redb::TableDefinition::<&str, &str>::new("rom_state"))
                .unwrap();
            let mut state: serde_json::Value =
                serde_json::from_str(table.get("state").unwrap().unwrap().value()).unwrap();
            state["work"]["work"]["a-active"]["pending"]["not_before"] = json!(20);
            table
                .insert("state", serde_json::to_string(&state).unwrap().as_str())
                .unwrap();
        } else {
            let mut table = tx
                .open_table(redb::TableDefinition::<(&str, u64), &str>::new("effects"))
                .unwrap();
            let mut intent: serde_json::Value =
                serde_json::from_str(table.get(("create-live", 0)).unwrap().unwrap().value())
                    .unwrap();
            intent["not_before"] = json!(20);
            table
                .insert(
                    ("create-live", 0),
                    serde_json::to_string(&intent).unwrap().as_str(),
                )
                .unwrap();
        }
        tx.commit().unwrap();
    } else {
        let db = rusqlite::Connection::open(path).unwrap();
        let (sql, update) = if location == "pending" {
            (
                "SELECT data FROM rom_state WHERE id=1",
                "UPDATE rom_state SET data=? WHERE id=1",
            )
        } else {
            (
                "SELECT data FROM effects WHERE identity='create-live' AND ordinal=0",
                "UPDATE effects SET data=? WHERE identity='create-live' AND ordinal=0",
            )
        };
        let data: String = db.query_row(sql, [], |row| row.get(0)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&data).unwrap();
        if location == "pending" {
            value["work"]["work"]["a-active"]["pending"]["not_before"] = json!(20);
        } else {
            value["not_before"] = json!(20);
        }
        db.execute(update, [serde_json::to_string(&value).unwrap()])
            .unwrap();
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    }
}
#[test]
fn format_eight_cannot_smuggle_new_pending_or_intent_timing() {
    for redb in [false, true] {
        for location in ["pending", "intent"] {
            let root = directory(redb);
            let source = root.join("format8-invalid");
            let destination = root.join("destination");
            let store = Native::open(redb, &source);
            populate(store.store());
            drop(store);
            predecessor_marker(redb, &source);
            inject_legacy_timing(redb, &source, location);
            let original = std::fs::read(&source).unwrap();
            assert!(matches!(
                Native::upgrade(redb, &source, &destination),
                Err(Error::Storage)
            ));
            assert!(!destination.exists());
            assert_eq!(std::fs::read(&source).unwrap(), original);
        }
    }
}
