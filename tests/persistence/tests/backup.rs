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

#[derive(Clone, Resource)]
#[resource(name = "things")]
struct Thing {
    private: String,
    optional: Option<String>,
}

enum Db {
    Sqlite(rom_sqlite::Sqlite),
    Redb(rom_redb::Redb),
}
impl Db {
    fn open(redb: bool, path: &Path) -> Self {
        let db = if redb {
            Self::Redb(rom_redb::Redb::open(path).unwrap())
        } else {
            Self::Sqlite(rom_sqlite::Sqlite::open(path).unwrap())
        };
        db.storage().register(&[Thing::descriptor()]).unwrap();
        db
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
            replay_version: None,
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
        db.storage().register(&[Thing::descriptor()]).unwrap();
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

#[test]
fn archive_rejects_noncanonical_descriptors_before_publication() {
    for redb in [false, true] {
        let dir = Dir::new();
        let backend = if redb { Backend::Redb } else { Backend::Sqlite };
        let snapshot = rom_backup::Snapshot {
            state: StorageState::new(StorageLimits::default()).unwrap(),
            rows: vec![],
            receipts: vec![],
            events: vec![],
            effects: vec![],
            descriptors: vec![Descriptor {
                kind: "unordered".into(),
                version: 1,
                fields: vec![
                    FieldDescriptor {
                        name: "z".into(),
                        shape: Shape::String,
                    },
                    FieldDescriptor {
                        name: "a".into(),
                        shape: Shape::Bool,
                    },
                ],
            }],
            references: vec![],
        };
        let archive = dir.p("archive");
        let destination = dir.p("restored");
        assert!(matches!(
            rom_backup::write(&archive, backend, &snapshot, BackupLimits::default()),
            Err(Error::Storage)
        ));
        assert!(!archive.exists());
        assert!(!destination.exists());
    }
}

#[derive(Clone, Resource)]
#[resource(name = "links")]
struct Link {
    target: ResourceRef<Thing>,
}

fn link_bundle(target: &str, changed: bool) -> Bundle {
    let mut b = bundle("link");
    b.receipt.row.key.kind = Link::KIND.into();
    b.receipt.row.value = Some(
        Link {
            target: ResourceRef::new(target).unwrap(),
        }
        .encode(),
    );
    b.effects.clear();
    b.receipt.row.protected = Default::default();
    if changed {
        b.expected = Some(1);
        b.receipt.identity = "replace-link".into();
        b.receipt.fingerprint = "link-to-b".into();
        b.receipt.row.revision = 2;
    }
    b
}
fn delete_thing(id: &str) -> Bundle {
    let mut b = bundle(id);
    b.expected = Some(1);
    b.receipt.identity = format!("delete-{id}");
    b.receipt.fingerprint = format!("delete-{id}");
    b.receipt.row.revision = 2;
    b.receipt.row.value = None;
    b.effects.clear();
    b
}
fn populate_links(db: &Db) {
    db.storage().register(&[Link::descriptor()]).unwrap();
    db.storage().commit(&bundle("a")).unwrap();
    db.storage().commit(&bundle("b")).unwrap();
    db.storage().commit(&link_bundle("a", false)).unwrap();
}
type CommitObserver = Arc<dyn Fn(usize) -> Result<()> + Send + Sync>;
fn observe(db: &Db, hook: Option<CommitObserver>) {
    match db {
        Db::Sqlite(db) => db.on_commit(hook),
        Db::Redb(db) => db.on_commit(hook),
    }
}

#[test]
fn reference_replacement_rolls_back_each_write_and_resolves_lost_ack() {
    use std::sync::Mutex;
    for redb in [false, true] {
        let dir = Dir::new();
        let db = Db::open(redb, &dir.p("baseline"));
        populate_links(&db);
        let points = Arc::new(Mutex::new(Vec::new()));
        let captured = points.clone();
        observe(
            &db,
            Some(Arc::new(move |point| {
                captured.lock().unwrap().push(point);
                Ok(())
            })),
        );
        db.storage().commit(&link_bundle("b", true)).unwrap();
        observe(&db, None);
        let points = points.lock().unwrap().clone();
        assert!(points.contains(&0));
        assert!(points.contains(&usize::MAX));
        assert!(
            points
                .iter()
                .filter(|&&n| n != 0 && n != usize::MAX)
                .count()
                > 4,
            "must exercise additional edge writes"
        );
        drop(db);
        for point in points {
            let db = Db::open(redb, &dir.p(&format!("fault-{point}")));
            populate_links(&db);
            observe(
                &db,
                Some(Arc::new(move |at| {
                    if at == point {
                        Err(Error::Storage)
                    } else {
                        Ok(())
                    }
                })),
            );
            let outcome = db.storage().commit(&link_bundle("b", true));
            observe(&db, None);
            if point == usize::MAX {
                assert_eq!(outcome, Err(Error::Unknown));
                let replay = db.storage().commit(&link_bundle("b", true)).unwrap();
                assert_eq!(replay, link_bundle("b", true).receipt);
                assert_eq!(
                    db.storage().commit(&delete_thing("b")),
                    Err(Error::Conflict)
                );
                db.storage().commit(&delete_thing("a")).unwrap();
            } else {
                assert_eq!(outcome, Err(Error::NotCommitted));
                assert_eq!(
                    db.storage()
                        .load(&link_bundle("a", false).receipt.row.key)
                        .unwrap(),
                    Some(link_bundle("a", false).receipt.row)
                );
                assert_eq!(db.storage().receipt("replace-link").unwrap(), None);
                assert_eq!(
                    db.storage()
                        .journal(Link::KIND, None, 100, 100_000)
                        .unwrap()
                        .events
                        .len(),
                    1
                );
                assert_eq!(
                    db.storage().commit(&delete_thing("a")),
                    Err(Error::Conflict)
                );
                db.storage().commit(&delete_thing("b")).unwrap();
            }
            // Reopen performs a complete row/catalog/edge consistency check.
            drop(db);
            drop(Db::open(redb, &dir.p(&format!("fault-{point}"))));
        }
    }
}

#[test]
fn backup_restore_preserves_restrict_edges_and_rejects_missing_edges() {
    for redb in [false, true] {
        let dir = Dir::new();
        let db = Db::open(redb, &dir.p("source"));
        populate_links(&db);
        let archive = dir.p("archive");
        let manifest = db.backup(&archive, BackupLimits::default()).unwrap();
        assert_eq!(manifest.references, 1);
        assert_eq!(manifest.descriptors, 2);
        let restored =
            Db::restore(redb, &archive, &dir.p("restored"), BackupLimits::default()).unwrap();
        assert_eq!(
            restored.storage().commit(&delete_thing("a")),
            Err(Error::Conflict)
        );
        let backend = if redb { Backend::Redb } else { Backend::Sqlite };
        let (_, mut snapshot) =
            rom_backup::read(&archive, backend, BackupLimits::default()).unwrap();
        snapshot.references.clear();
        assert!(matches!(
            rom_backup::write(dir.p("bad"), backend, &snapshot, BackupLimits::default()),
            Err(Error::Storage)
        ));
        assert!(!dir.p("bad").exists());
    }
}

#[test]
fn reopening_rejects_missing_reference_index_entries_without_repair() {
    for redb in [false, true] {
        let dir = Dir::new();
        let path = dir.p("source");
        let db = Db::open(redb, &path);
        populate_links(&db);
        drop(db);
        if redb {
            let db = redb::Database::open(&path).unwrap();
            let tx = db.begin_write().unwrap();
            {
                let mut incoming = tx
                    .open_table(redb::TableDefinition::<(&str, &str, &str, &str), u8>::new(
                        "rom_references_in",
                    ))
                    .unwrap();
                incoming
                    .remove((Thing::KIND, "a", Link::KIND, "link"))
                    .unwrap();
            }
            tx.commit().unwrap();
            drop(db);
            assert!(matches!(rom_redb::Redb::open(&path), Err(Error::Storage)));
            let db = redb::Database::open(&path).unwrap();
            use redb::ReadableDatabase;
            let tx = db.begin_read().unwrap();
            let incoming = tx
                .open_table(redb::TableDefinition::<(&str, &str, &str, &str), u8>::new(
                    "rom_references_in",
                ))
                .unwrap();
            assert!(
                incoming
                    .get((Thing::KIND, "a", Link::KIND, "link"))
                    .unwrap()
                    .is_none()
            );
        } else {
            let c = rusqlite::Connection::open(&path).unwrap();
            assert_eq!(c.execute("DELETE FROM reference_edges", []).unwrap(), 1);
            drop(c);
            assert!(matches!(
                rom_sqlite::Sqlite::open(&path),
                Err(Error::Storage)
            ));
            let c = rusqlite::Connection::open(&path).unwrap();
            let remaining: i64 = c
                .query_row("SELECT COUNT(*) FROM reference_edges", [], |r| r.get(0))
                .unwrap();
            assert_eq!(remaining, 0);
        }
    }
}

#[test]
fn startup_record_budget_includes_durable_pending_work() {
    for redb in [false, true] {
        let dir = Dir::new();
        let path = dir.p("source");
        let db = Db::open(redb, &path);
        let mut b = bundle("one");
        b.reaction_limits = Some(ReactionLimits::default());
        b.reactions = ["a", "b", "c"].map(|id| pending(id, &b.receipt.row)).into();
        db.storage().commit(&b).unwrap();
        drop(db);
        // One row/receipt/event/effect/descriptor plus three work records = eight.
        let limit = BackupLimits {
            max_records: 7,
            ..Default::default()
        };
        let result = if redb {
            rom_redb::Redb::open_with_validation_limits(&path, StorageLimits::default(), limit)
                .map(|_| ())
        } else {
            rom_sqlite::Sqlite::open_with_validation_limits(&path, StorageLimits::default(), limit)
                .map(|_| ())
        };
        assert_eq!(result, Err(Error::TooLarge));
    }
}
