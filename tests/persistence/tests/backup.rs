use rom::*;
use rom_backup::{Backend, BackupLimits};
use std::os::unix::fs::PermissionsExt;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
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
            Self::Sqlite(d) => d,
            Self::Redb(d) => d,
        }
    }
    fn backup(&self, path: &Path, l: BackupLimits) -> Result<rom_backup::Manifest> {
        match self {
            Self::Sqlite(d) => d.backup_to(path, l),
            Self::Redb(d) => d.backup_to(path, l),
        }
    }
    fn restore(redb: bool, path: &Path, target: &Path, l: BackupLimits) -> Result<Self> {
        if redb {
            rom_redb::Redb::restore_from(path, target, l).map(Self::Redb)
        } else {
            rom_sqlite::Sqlite::restore_from(path, target, l).map(Self::Sqlite)
        }
    }
    fn counts(&self) -> [u64; 4] {
        match self {
            Self::Sqlite(d) => d.counts().unwrap(),
            Self::Redb(d) => d.counts().unwrap(),
        }
    }
    fn lose_ack(&self) {
        let hook = Arc::new(|p| {
            if p == usize::MAX {
                Err(Error::Unknown)
            } else {
                Ok(())
            }
        });
        match self {
            Self::Sqlite(d) => d.on_commit(Some(hook)),
            Self::Redb(d) => d.on_commit(Some(hook)),
        }
    }
}
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-backup-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn p(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn bundle(id: &str) -> Bundle {
    let row = Row {
        key: Key {
            kind: "things".into(),
            id: id.into(),
        },
        revision: 1,
        value: Some(json!({"private":"retained verbatim","optional":null})),
        protected: ProtectedMetadata {
            deletion_authorization: None,
            source_provenance: Some(SourceProvenance {
                source: "config".into(),
                version: "v1".into(),
                generation: 9,
                field_origins: [("private".into(), "environment".into())].into(),
            }),
        },
    };
    Bundle {
        expected: None,
        receipt: Receipt {
            identity: format!("receipt-{id}"),
            fingerprint: "fingerprint".into(),
            row,
        },
        changed: true,
        effects: vec![Intent {
            channel: "audit".into(),
            payload: json!({"private":"intent"}),
            delivery_version: None,
        }],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
fn pending(id: &str, row: &Row) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            root: "chain".into(),
            parent: None,
            depth: 1,
            started_at: 0,
            path: vec![id.into()],
        },
        definition: "deliver".into(),
        version: 1,
        service_key: "service".into(),
        payload: WorkPayload::Notification {
            source: row.clone(),
            payload: json!({"private":"mail"}),
        },
    }
}
fn claim(db: &Db) -> WorkClaim {
    let WorkResult::Claimed(c) = db
        .storage()
        .reaction_update(WorkUpdate::Claim { now: 0 })
        .unwrap()
    else {
        panic!("claim expected")
    };
    *c
}
#[test]
fn full_snapshot_recovers_receipts_protected_values_work_and_fences_old_owners() {
    for redb in [false, true] {
        let dir = Dir::new();
        let source = dir.p("source");
        let archive = dir.p("archive");
        let target = dir.p("restored");
        let db = Db::open(redb, &source);
        let mut b = bundle("one");
        b.reaction_limits = Some(ReactionLimits::default());
        b.reactions = ["a-done", "b-stopped", "c-lost-ack"]
            .map(|id| pending(id, &b.receipt.row))
            .into();
        db.lose_ack();
        assert_eq!(db.storage().commit(&b), Err(Error::Unknown));
        drop(db);
        let db = Db::open(redb, &source);
        let done = claim(&db);
        db.storage()
            .reaction_update(WorkUpdate::Finish {
                claim: done.key(),
                now: 0,
                outcome: WorkOutcome::Done,
            })
            .unwrap();
        let stop = claim(&db);
        db.storage()
            .reaction_update(WorkUpdate::Finish {
                claim: stop.key(),
                now: 0,
                outcome: WorkOutcome::Stop(StopReason::Denied),
            })
            .unwrap();
        let lost = claim(&db);
        db.storage()
            .reaction_update(WorkUpdate::DeliveryStarted {
                claim: lost.key(),
                now: 0,
            })
            .unwrap();
        let mut tomb = bundle("deleted");
        tomb.receipt.row.protected.deletion_authorization = tomb.receipt.row.value.take();
        db.storage().commit(&tomb).unwrap();
        let cursor = db.storage().journal_head("things").unwrap();
        let counts = db.counts();
        let before = db.storage().reaction_records().unwrap();
        let manifest = db.backup(&archive, BackupLimits::default()).unwrap();
        assert_eq!((manifest.rows, manifest.work), (2, 3));
        assert!(!manifest.external_blobs_included);
        assert!(!manifest.external_deliveries_included);
        assert_eq!(
            std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let restored = Db::restore(redb, &archive, &target, BackupLimits::default()).unwrap();
        assert_eq!(restored.counts(), counts);
        assert_eq!(
            restored.storage().load(&tomb.receipt.row.key).unwrap(),
            Some(tomb.receipt.row.clone())
        );
        assert_eq!(
            restored.storage().load(&b.receipt.row.key).unwrap(),
            Some(b.receipt.row.clone())
        );
        assert_eq!(restored.storage().commit(&b).unwrap(), b.receipt);
        assert_eq!(restored.counts(), counts);
        assert!(matches!(
            restored
                .storage()
                .journal("things", Some(&cursor), 10, 100_000),
            Err(Error::HistoryGap)
        ));
        assert_eq!(db.storage().journal_head("things").unwrap(), cursor);
        assert_eq!(db.storage().reaction_records().unwrap(), before);
        assert_eq!(
            restored
                .storage()
                .reaction_update(WorkUpdate::DeliveryFinished {
                    claim: lost.key(),
                    now: 0,
                    outcome: DeliveryOutcome::Accepted
                }),
            Err(Error::Conflict)
        );
        let recovered = claim(&restored);
        assert_eq!(recovered.work.pending, lost.work.pending);
        assert_eq!(recovered.work.attempts, 2);
        assert_eq!(recovered.work.delivery, Some(DeliveryOutcome::Unknown));
        assert!(recovered.work.generation > lost.work.generation);
        restored
            .storage()
            .reaction_update(WorkUpdate::DeliveryFinished {
                claim: recovered.key(),
                now: 0,
                outcome: DeliveryOutcome::Accepted,
            })
            .unwrap();
        drop(restored);
        let reopened = Db::open(redb, &target);
        let records = reopened.storage().reaction_records().unwrap();
        assert_eq!(records[0].state, WorkState::Done);
        assert_eq!(records[1].state, WorkState::Stopped(StopReason::Denied));
        assert_eq!(records[2].state, WorkState::Done);
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn rejects_overwrite_wrong_backend_corruption_and_bounds_without_publication() {
    for redb in [false, true] {
        let dir = Dir::new();
        let db = Db::open(redb, &dir.p("source"));
        db.storage().commit(&bundle("one")).unwrap();
        let archive = dir.p("archive");
        db.backup(&archive, BackupLimits::default()).unwrap();
        let bytes = std::fs::read(&archive).unwrap();
        assert_eq!(
            db.backup(&archive, BackupLimits::default()),
            Err(Error::Conflict)
        );
        assert_eq!(std::fs::read(&archive).unwrap(), bytes);
        let dest = dir.p("exists");
        std::fs::write(&dest, b"keep").unwrap();
        assert!(matches!(
            Db::restore(redb, &archive, &dest, BackupLimits::default()),
            Err(Error::Conflict)
        ));
        assert_eq!(std::fs::read(&dest).unwrap(), b"keep");
        assert!(matches!(
            Db::restore(!redb, &archive, &dir.p("wrong"), BackupLimits::default()),
            Err(Error::Unsupported(_))
        ));
        assert!(!dir.p("wrong").exists());
        for limits in [
            BackupLimits {
                max_bytes: 64,
                max_records: 100,
            },
            BackupLimits {
                max_bytes: 100_000,
                max_records: 1,
            },
        ] {
            assert_eq!(db.backup(&dir.p("small"), limits), Err(Error::TooLarge));
            assert!(!dir.p("small").exists());
            assert!(matches!(
                Db::restore(redb, &archive, &dir.p("small"), limits),
                Err(Error::TooLarge)
            ));
        }
        let mut corrupt = bytes.clone();
        let i = corrupt.len() - 1;
        corrupt[i] ^= 1;
        std::fs::write(&archive, &corrupt).unwrap();
        assert!(matches!(
            Db::restore(redb, &archive, &dir.p("bad"), BackupLimits::default()),
            Err(Error::Storage)
        ));
        assert!(!dir.p("bad").exists());
        std::fs::write(&archive, &bytes[..bytes.len() - 1]).unwrap();
        assert!(Db::restore(redb, &archive, &dir.p("bad"), BackupLimits::default()).is_err());
        std::fs::write(&archive, &bytes).unwrap();
        std::fs::set_permissions(&archive, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Db::restore(redb, &archive, &dir.p("bad"), BackupLimits::default()).is_err());
        assert_eq!(db.counts(), [1, 1, 1, 1]);
    }
}
#[test]
fn writer_rejects_inconsistent_logical_tables() {
    for redb in [false, true] {
        let dir = Dir::new();
        let db = Db::open(redb, &dir.p("source"));
        db.storage().commit(&bundle("one")).unwrap();
        let backend = if redb { Backend::Redb } else { Backend::Sqlite };
        let archive = dir.p("archive");
        db.backup(&archive, BackupLimits::default()).unwrap();
        let (_, mut snapshot) =
            rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
        snapshot.receipts.clear();
        assert!(snapshot.validate().is_err());
        assert!(
            rom_backup::write(dir.p("bad"), backend, &snapshot, BackupLimits::default()).is_err()
        );
        assert!(!dir.p("bad").exists());
        snapshot.state = StorageState::new(StorageLimits::default()).unwrap();
        snapshot.events.clear();
        snapshot.effects.clear();
        assert!(
            snapshot.validate().is_err(),
            "current row requires its authoritative receipt"
        );
    }
}
#[test]
fn concurrent_writer_snapshots_are_complete_bundles() {
    for redb in [false, true] {
        let dir = Dir::new();
        let db = Arc::new(Db::open(redb, &dir.p("source")));
        let writer = db.clone();
        let thread = std::thread::spawn(move || {
            for n in 0..24 {
                writer.storage().commit(&bundle(&n.to_string())).unwrap();
            }
        });
        for i in 0..4 {
            let archive = dir.p(&format!("snapshot{i}"));
            db.backup(&archive, BackupLimits::default()).unwrap();
            let restored = Db::restore(
                redb,
                &archive,
                &dir.p(&format!("restored{i}")),
                BackupLimits::default(),
            )
            .unwrap();
            let [rows, events, receipts, effects] = restored.counts();
            assert_eq!([events, receipts, effects], [rows; 3]);
        }
        thread.join().unwrap();
        assert_eq!(db.counts(), [24; 4]);
    }
}
#[test]
fn archived_limits_and_retention_survive_restore() {
    for redb in [false, true] {
        let dir = Dir::new();
        let limits = StorageLimits {
            journal_rows: 1,
            receipts: 2,
            ..StorageLimits::default()
        };
        let db = if redb {
            Db::Redb(rom_redb::Redb::open_with_limits(dir.p("source"), limits.clone()).unwrap())
        } else {
            Db::Sqlite(
                rom_sqlite::Sqlite::open_with_limits(dir.p("source"), limits.clone()).unwrap(),
            )
        };
        for id in ["one", "two"] {
            db.storage().commit(&bundle(id)).unwrap();
        }
        let archive = dir.p("archive");
        db.backup(&archive, BackupLimits::default()).unwrap();
        let restored =
            Db::restore(redb, &archive, &dir.p("target"), BackupLimits::default()).unwrap();
        assert_eq!(restored.counts(), [2, 1, 2, 2]);
        assert_eq!(
            restored.storage().commit(&bundle("three")),
            Err(Error::Overloaded)
        );
        assert!(matches!(
            restored.storage().journal("things", None, 10, 100_000),
            Err(Error::HistoryGap)
        ));
    }
}
