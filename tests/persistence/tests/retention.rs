#[path = "support/legacy_native.rs"]
mod legacy_native;
use rom::*;
use rom_backup::{BackupLimits, RetentionPolicy, RetentionReport};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Resource)]
#[resource(name = "retention-counters")]
struct Counter {
    count: u64,
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "rom-retention-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
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
    fn retain(
        redb: bool,
        source: &Path,
        dest: &Path,
        policy: &RetentionPolicy,
    ) -> Result<(Self, RetentionReport)> {
        if redb {
            rom_redb::Redb::retain_from(source, dest, policy, BackupLimits::default())
                .map(|(s, r)| (Self::Redb(s), r))
        } else {
            rom_sqlite::Sqlite::retain_from(source, dest, policy, BackupLimits::default())
                .map(|(s, r)| (Self::Sqlite(s), r))
        }
    }
    fn backup(&self, path: &Path) {
        match self {
            Self::Sqlite(s) => s.backup_to(path, BackupLimits::default()),
            Self::Redb(s) => s.backup_to(path, BackupLimits::default()),
        }
        .unwrap();
    }
    fn restore(redb: bool, source: &Path, dest: &Path) -> Self {
        if redb {
            Self::Redb(rom_redb::Redb::restore_from(source, dest, BackupLimits::default()).unwrap())
        } else {
            Self::Sqlite(
                rom_sqlite::Sqlite::restore_from(source, dest, BackupLimits::default()).unwrap(),
            )
        }
    }
}
fn bundle(id: &str, identity: &str, revision: u64, epoch: u64) -> Bundle {
    Bundle {
        expected: revision.checked_sub(1).filter(|n| *n > 0),
        receipt: Receipt {
            retry_epoch: epoch,
            replay_version: Some(1),
            identity: identity.into(),
            fingerprint: identity.into(),
            row: Row {
                key: Key {
                    kind: Counter::KIND.into(),
                    id: id.into(),
                },
                revision,
                value: Some(Counter { count: revision }.encode()),
                protected: Default::default(),
            },
        },
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
fn seed(redb: bool, path: &Path, work: bool) -> Bundle {
    let db = Db::open(redb, path);
    let s = db.storage();
    s.register(&[Counter::descriptor()]).unwrap();
    s.commit(&bundle("one", "first", 1, 0)).unwrap();
    let mut last = bundle("one", "second", 2, 0);
    if work {
        last.reaction_limits = Some(ReactionLimits::default());
        last.reactions.push(PendingWork {
            not_before: None,
            delivery_profile: rom::DeliveryProfile::AtLeastOnce,
            id: "pending".into(),
            definition: "copy".into(),
            version: 1,
            service_key: "service".into(),
            cause: Cause {
                retry_epoch: 0,
                root: "root".into(),
                parent: None,
                depth: 1,
                started_at: 0,
                path: vec!["pending".into()],
            },
            payload: WorkPayload::Action(
                serde_json::to_value(Invocation::from(
                    Command::create("child", Counter { count: 1 }).idempotency("causal"),
                ))
                .unwrap(),
            ),
        });
    }
    s.commit(&last).unwrap();
    last
}
fn expired_policy() -> RetentionPolicy {
    RetentionPolicy::new(RetryEpochs {
        current: 1,
        admission_floor: 1,
        replay_floor: 1,
    })
    .journal_through(2)
}
fn assert_expired(storage: &dyn Storage, last: &Bundle) {
    assert_eq!(
        storage.retry_epochs().unwrap(),
        RetryEpochs {
            current: 1,
            admission_floor: 1,
            replay_floor: 1
        }
    );
    assert!(
        storage.receipt("second").unwrap().is_some(),
        "current state anchor stays stored"
    );
    assert_eq!(
        storage.commit(last),
        Err(Error::IdentityExpired),
        "retained anchor cannot replay after expiry"
    );
    let mut spoof = last.clone();
    spoof.receipt.retry_epoch = 1;
    assert_eq!(
        storage.commit(&spoof),
        Err(Error::IdentityExpired),
        "changing request epoch cannot revive expired anchor"
    );
    assert_eq!(
        storage.load(&last.receipt.row.key).unwrap(),
        Some(last.receipt.row.clone())
    );
}
#[test]
fn retention_expires_even_retained_anchors_and_survives_reopen_backup() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let dest = dir.path("retained");
        let last = seed(redb, &source, false);
        let original = std::fs::read(&source).unwrap();
        let (db, report) = Db::retain(redb, &source, &dest, &expired_policy()).unwrap();
        assert_eq!(report.receipts_removed, 1);
        assert_eq!(report.receipts_retained, 1);
        assert_eq!(report.events_removed, 2);
        assert_expired(db.storage(), &last);
        assert!(db.storage().receipt("first").unwrap().is_none());
        let archive = dir.path("archive");
        db.backup(&archive);
        drop(db);
        let reopened = Db::open(redb, &dest);
        assert_expired(reopened.storage(), &last);
        drop(reopened);
        let restored = Db::restore(redb, &archive, &dir.path("restored"));
        assert_expired(restored.storage(), &last);
        restored
            .storage()
            .commit(&bundle("one", "current", 3, 1))
            .unwrap();
        assert!(std::fs::read(&source).unwrap() == original);
    }
}
#[test]
fn sealed_epoch_allows_replay_and_only_exact_causal_claim_for_fresh_work() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let last = seed(redb, &source, true);
        let policy = RetentionPolicy::new(RetryEpochs {
            current: 2,
            admission_floor: 2,
            replay_floor: 0,
        });
        let (db, _) = Db::retain(redb, &source, &dir.path("sealed"), &policy).unwrap();
        let s = db.storage();
        assert_eq!(s.commit(&last).unwrap(), last.receipt);
        assert_eq!(
            s.commit(&bundle("fresh", "not-admitted", 1, 0)),
            Err(Error::IdentityExpired)
        );
        let WorkResult::Claimed(claim) = s.reaction_update(WorkUpdate::Claim { now: 0 }).unwrap()
        else {
            panic!("persisted work must claim")
        };
        let mut causal = bundle("child", "causal", 1, 0);
        causal.completed_work = Some((claim.key(), 0));
        let mut wrong_epoch = causal.clone();
        wrong_epoch.receipt.retry_epoch = 1;
        assert!(s.commit(&wrong_epoch).is_err());
        assert!(s.load(&causal.receipt.row.key).unwrap().is_none());
        let mut wrong_claim = causal.clone();
        wrong_claim.completed_work.as_mut().unwrap().0.generation += 1;
        assert!(s.commit(&wrong_claim).is_err());
        assert_eq!(s.commit(&causal).unwrap(), causal.receipt);
        assert_eq!(s.reaction_records().unwrap()[0].state, WorkState::Done);
        assert_eq!(
            s.commit(&causal).unwrap(),
            causal.receipt,
            "causal replay no longer requires active claim"
        );
        s.commit(&bundle("fresh", "admitted", 1, 2)).unwrap();
    }
}
#[test]
#[ignore = "subprocess fixture invoked by retention_interruption_before_publication_is_retryable"]
fn retention_child_exits_before_publication() {
    let source = PathBuf::from(std::env::var_os("ROM_RETAIN_SOURCE").unwrap());
    let dest = PathBuf::from(std::env::var_os("ROM_RETAIN_DEST").unwrap());
    let before = || -> Result<()> { std::process::exit(86) };
    if std::env::var("ROM_RETAIN_REDB").unwrap() == "true" {
        rom_redb::Redb::retain_from_observed(
            source,
            dest,
            &expired_policy(),
            BackupLimits::default(),
            before,
        )
        .unwrap();
    } else {
        rom_sqlite::Sqlite::retain_from_observed(
            source,
            dest,
            &expired_policy(),
            BackupLimits::default(),
            before,
        )
        .unwrap();
    }
    panic!("checkpoint not reached");
}
#[test]
fn retention_interruption_before_publication_is_retryable() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let dest = dir.path("retained");
        let last = seed(redb, &source, false);
        let original = std::fs::read(&source).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "retention_child_exits_before_publication",
                "--ignored",
            ])
            .env("ROM_RETAIN_SOURCE", &source)
            .env("ROM_RETAIN_DEST", &dest)
            .env("ROM_RETAIN_REDB", redb.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(!dest.exists());
        assert!(std::fs::read(&source).unwrap() == original);
        let (db, _) = Db::retain(redb, &source, &dest, &expired_policy()).unwrap();
        assert_expired(db.storage(), &last);
    }
}

#[test]
fn retention_reclaims_bounded_receipt_and_effect_capacity() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let dest = dir.path("retained");
        let limits = StorageLimits {
            receipts: 2,
            effects: 1,
            ..StorageLimits::default()
        };
        let db = if redb {
            Db::Redb(rom_redb::Redb::open_with_limits(&source, limits).unwrap())
        } else {
            Db::Sqlite(rom_sqlite::Sqlite::open_with_limits(&source, limits).unwrap())
        };
        db.storage().register(&[Counter::descriptor()]).unwrap();
        let mut first = bundle("one", "first", 1, 0);
        first.effects.push(Intent::new("audit", json!({"count":1})));
        db.storage().commit(&first).unwrap();
        let anchor = bundle("one", "second", 2, 0);
        db.storage().commit(&anchor).unwrap();
        let mut next = bundle("one", "third", 3, 0);
        next.effects.push(Intent::new("audit", json!({"count":3})));
        assert_eq!(db.storage().commit(&next), Err(Error::Overloaded));
        drop(db);
        let original = std::fs::read(&source).unwrap();
        let (retained, report) = Db::retain(
            redb,
            &source,
            &dest,
            &expired_policy().settle_effects("first"),
        )
        .unwrap();
        assert_eq!(report.receipts_removed, 1);
        assert_eq!(report.effects_removed, 1);
        assert_eq!(report.receipts_retained, 1);
        assert_expired(retained.storage(), &anchor);
        next.receipt.retry_epoch = 1;
        retained.storage().commit(&next).unwrap();
        assert_eq!(
            retained.storage().commit(&bundle("one", "fourth", 4, 1)),
            Err(Error::Overloaded),
            "original bounded capacity stays active"
        );
        assert!(std::fs::read(&source).unwrap() == original);
    }
}

#[test]
fn purged_tombstone_can_be_recreated_without_reviving_expired_requests() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let source = dir.path("source");
        let db = Db::open(redb, &source);
        db.storage().register(&[Counter::descriptor()]).unwrap();
        let create = bundle("one", "create", 1, 0);
        db.storage().commit(&create).unwrap();
        let mut delete = bundle("one", "delete", 2, 0);
        delete.receipt.row.value = None;
        delete.receipt.row.protected.deletion_authorization = create.receipt.row.value.clone();
        db.storage().commit(&delete).unwrap();
        drop(db);
        let original = std::fs::read(&source).unwrap();
        let policy = expired_policy().purge_tombstone(create.receipt.row.key.clone());
        let (retained, report) = Db::retain(redb, &source, &dir.path("retained"), &policy).unwrap();
        assert_eq!(report.tombstones_removed, 1);
        assert_eq!(report.receipts_removed, 2);
        assert!(
            retained
                .storage()
                .load(&create.receipt.row.key)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            retained.storage().commit(&create),
            Err(Error::IdentityExpired)
        );
        let recreate = bundle("one", "recreate", 1, 1);
        retained.storage().commit(&recreate).unwrap();
        assert_eq!(
            retained.storage().commit(&delete),
            Err(Error::IdentityExpired)
        );
        assert_eq!(
            retained.storage().load(&create.receipt.row.key).unwrap(),
            Some(recreate.receipt.row)
        );
        assert!(std::fs::read(&source).unwrap() == original);
    }
}

#[derive(Clone, Resource)]
#[resource(name = "retention-counters", version = 2)]
struct CounterV2 {
    total: u64,
}
#[test]
fn legacy_native_format_cannot_import_nonzero_retry_epochs() {
    for redb in [false, true] {
        let dir = Scratch::new();
        let original_path = dir.path("original");
        seed(redb, &original_path, false);
        let source = dir.path("source");
        let (retained, _) = Db::retain(redb, &original_path, &source, &expired_policy()).unwrap();
        drop(retained);
        legacy_native::mark(redb, &source, 5, false);
        let bytes = std::fs::read(&source).unwrap();
        let dest = dir.path("upgraded");
        let result = if redb {
            rom_redb::Redb::upgrade_from(
                &source,
                &dest,
                &[Counter::descriptor()],
                BackupLimits::default(),
            )
            .map(|_| ())
        } else {
            rom_sqlite::Sqlite::upgrade_from(
                &source,
                &dest,
                &[Counter::descriptor()],
                BackupLimits::default(),
            )
            .map(|_| ())
        };
        assert!(matches!(result, Err(Error::Unsupported(_))));
        assert!(!dest.exists());
        let plan = rom_backup::MigrationPlan::new(vec![
            rom_backup::ResourceMigration::new::<Counter, CounterV2>(|_| {
                panic!("legacy epoch rejection must precede conversion")
            })
            .unwrap(),
        ])
        .unwrap();
        let result = if redb {
            rom_redb::Redb::migrate_from(&source, &dest, &plan, BackupLimits::default()).map(|_| ())
        } else {
            rom_sqlite::Sqlite::migrate_from(&source, &dest, &plan, BackupLimits::default())
                .map(|_| ())
        };
        assert!(matches!(result, Err(Error::Unsupported(_))));
        assert!(!dest.exists());
        assert!(std::fs::read(&source).unwrap() == bytes);
    }
}

#[test]
fn current_format_migration_preserves_nonzero_epochs_receipts_and_work() {
    for redb in [false, true] {
        for version in [6, 7, rom_backup::STORAGE_FORMAT] {
            let dir = Scratch::new();
            let initial = dir.path("initial");
            seed(redb, &initial, false);
            let source = dir.path("epoch-one");
            let epochs = RetryEpochs {
                current: 1,
                admission_floor: 1,
                replay_floor: 0,
            };
            let (db, _) =
                Db::retain(redb, &initial, &source, &RetentionPolicy::new(epochs)).unwrap();
            let mut input = bundle("work-source", "epoch-one-receipt", 1, 1);
            input.reaction_limits = Some(ReactionLimits::default());
            input.reactions.push(PendingWork {
                not_before: None,
                delivery_profile: rom::DeliveryProfile::AtLeastOnce,
                id: "epoch-one-work".into(),
                definition: "notify".into(),
                version: 1,
                service_key: "service".into(),
                cause: Cause {
                    retry_epoch: 1,
                    root: "epoch-one-root".into(),
                    parent: None,
                    depth: 1,
                    started_at: 0,
                    path: vec!["epoch-one-work".into()],
                },
                payload: WorkPayload::Notification {
                    source: input.receipt.row.clone(),
                    payload: json!({"channel":"audit"}),
                },
            });
            db.storage().commit(&input).unwrap();
            drop(db);
            if version < rom_backup::STORAGE_FORMAT {
                legacy_native::mark(redb, &source, version, false);
                let destination = dir.path("upgraded");
                let upgraded = if redb {
                    Db::Redb(
                        rom_redb::Redb::upgrade_from(
                            &source,
                            &destination,
                            &[Counter::descriptor()],
                            BackupLimits::default(),
                        )
                        .unwrap(),
                    )
                } else {
                    Db::Sqlite(
                        rom_sqlite::Sqlite::upgrade_from(
                            &source,
                            &destination,
                            &[Counter::descriptor()],
                            BackupLimits::default(),
                        )
                        .unwrap(),
                    )
                };
                assert_eq!(upgraded.storage().retry_epochs().unwrap(), epochs);
                assert_eq!(
                    upgraded.storage().receipt(&input.receipt.identity).unwrap(),
                    Some(input.receipt.clone())
                );
                let work = upgraded.storage().reaction_records().unwrap();
                assert_eq!(work[0].revision, 0);
                assert_eq!(work[0].pending, input.reactions[0]);
            }
            let original = std::fs::read(&source).unwrap();
            let plan = rom_backup::MigrationPlan::new(vec![
                rom_backup::ResourceMigration::new::<Counter, CounterV2>(|old| {
                    Ok(CounterV2 { total: old.count })
                })
                .unwrap(),
            ])
            .unwrap()
            .validate_work(|work| {
                if work.cause.retry_epoch != 1 || work.definition != "notify" || work.version != 1 {
                    return Err(Error::Storage);
                }
                let WorkPayload::Notification { source, payload } = &work.payload else {
                    return Err(Error::Storage);
                };
                CounterV2::decode(source.value.clone().ok_or(Error::Storage)?)?;
                if payload != &json!({"channel":"audit"}) {
                    return Err(Error::Storage);
                }
                Ok(())
            });
            let destination = dir.path("migrated");
            let migrated = if redb {
                Db::Redb(
                    rom_redb::Redb::migrate_from(
                        &source,
                        &destination,
                        &plan,
                        BackupLimits::default(),
                    )
                    .unwrap(),
                )
            } else {
                Db::Sqlite(
                    rom_sqlite::Sqlite::migrate_from(
                        &source,
                        &destination,
                        &plan,
                        BackupLimits::default(),
                    )
                    .unwrap(),
                )
            };
            let check = |db: &Db| {
                let storage = db.storage();
                storage.register(&[CounterV2::descriptor()]).unwrap();
                assert_eq!(storage.retry_epochs().unwrap(), epochs);
                let receipt = storage.receipt(&input.receipt.identity).unwrap().unwrap();
                assert_eq!(receipt.identity, input.receipt.identity);
                assert_eq!(receipt.fingerprint, input.receipt.fingerprint);
                assert_eq!(receipt.retry_epoch, 1);
                assert_eq!(receipt.replay_version, Some(1));
                assert_eq!(
                    CounterV2::decode(receipt.row.value.clone().unwrap())
                        .unwrap()
                        .total,
                    1
                );
                assert_eq!(storage.load(&receipt.row.key).unwrap(), Some(receipt.row));
                let records = storage.reaction_records().unwrap();
                assert_eq!(records.len(), 1);
                let pending = &records[0].pending;
                assert_eq!(pending.delivery_profile, rom::DeliveryProfile::AtLeastOnce);
                assert_eq!(pending.id, input.reactions[0].id);
                assert_eq!(pending.cause, input.reactions[0].cause);
                let WorkPayload::Notification { source, payload } = &pending.payload else {
                    panic!("notification expected")
                };
                assert_eq!(
                    CounterV2::decode(source.value.clone().unwrap())
                        .unwrap()
                        .total,
                    1
                );
                assert_eq!(payload, &json!({"channel":"audit"}));
            };
            check(&migrated);
            let archive = dir.path("archive");
            migrated.backup(&archive);
            drop(migrated);
            let restored = Db::restore(redb, &archive, &dir.path("restored"));
            check(&restored);
            assert!(std::fs::read(&source).unwrap() == original);
        }
    }
}
