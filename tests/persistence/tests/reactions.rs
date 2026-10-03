use rom::{
    Cause, PendingWork, ReactionLimits, StopReason, WorkLedger, WorkPayload, WorkResult, WorkState,
    WorkUpdate, json,
};
fn pending(id: &str) -> PendingWork {
    PendingWork {
        id: id.into(),
        cause: Cause {
            root: "root".into(),
            parent: None,
            depth: 1,
            started_at: 0,
            path: vec![id.into()],
        },
        definition: "copy".into(),
        version: 1,
        service_key: "service".into(),
        payload: WorkPayload::Action(json!({"frozen":1})),
    }
}
#[test]
fn persisted_work_lease_and_input_are_fenced() {
    let limits = ReactionLimits {
        lease_seconds: 2,
        ..ReactionLimits::default()
    };
    let mut ledger = WorkLedger::default();
    ledger.enqueue(&limits, vec![pending("one")]).unwrap();
    let WorkResult::Claimed(first) = ledger.apply(WorkUpdate::Claim { now: 0 }).unwrap() else {
        panic!()
    };
    assert!(matches!(
        ledger.apply(WorkUpdate::Claim { now: 1 }).unwrap(),
        WorkResult::Idle
    ));
    let serialized = serde_json::to_vec(&ledger).unwrap();
    let mut recovered: WorkLedger = serde_json::from_slice(&serialized).unwrap();
    let WorkResult::Claimed(second) = recovered.apply(WorkUpdate::Claim { now: 2 }).unwrap() else {
        panic!()
    };
    assert_eq!(first.work.pending, second.work.pending);
    assert!(
        recovered
            .apply(WorkUpdate::Finish {
                claim: first.key(),
                now: 2,
                outcome: rom::WorkOutcome::Done
            })
            .is_err()
    );
    recovered
        .apply(WorkUpdate::Finish {
            claim: second.key(),
            now: 2,
            outcome: rom::WorkOutcome::Done,
        })
        .unwrap();
    assert_eq!(recovered.records()[0].state, WorkState::Done);
}
#[test]
fn finite_work_budget_allows_receipt_resolution_but_no_new_execution() {
    let limits = ReactionLimits {
        max_work: 1,
        lease_seconds: 1,
        ..ReactionLimits::default()
    };
    let mut ledger = WorkLedger::default();
    ledger.enqueue(&limits, vec![pending("one")]).unwrap();
    let WorkResult::Claimed(first) = ledger.apply(WorkUpdate::Claim { now: 0 }).unwrap() else {
        panic!()
    };
    assert!(!first.resolution_only);
    let WorkResult::Claimed(recovery) = ledger.apply(WorkUpdate::Claim { now: 1 }).unwrap() else {
        panic!()
    };
    assert!(recovery.resolution_only);
    ledger
        .apply(WorkUpdate::Finish {
            claim: recovery.key(),
            now: 1,
            outcome: rom::WorkOutcome::Stop(StopReason::WorkBudget),
        })
        .unwrap();
    assert!(matches!(
        ledger.records()[0].state,
        WorkState::Stopped(StopReason::WorkBudget)
    ));
}
#[derive(Clone, rom::Resource)]
#[resource(name = "things")]
struct Thing {
    title: String,
}
fn database(
    redb: bool,
    path: &std::path::Path,
    limits: rom::StorageLimits,
) -> Box<dyn rom::Storage> {
    let store: Box<dyn rom::Storage> = if redb {
        Box::new(rom_redb::Redb::open_with_limits(path, limits).unwrap())
    } else {
        Box::new(rom_sqlite::Sqlite::open_with_limits(path, limits).unwrap())
    };
    store
        .register(&[<Thing as rom::Resource>::descriptor()])
        .unwrap();
    store
}
fn bundle(id: &str) -> rom::Bundle {
    rom::Bundle {
        expected: None,
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
        receipt: rom::Receipt {
            identity: id.into(),
            fingerprint: id.into(),
            row: rom::Row {
                protected: Default::default(),
                key: rom::Key {
                    kind: "things".into(),
                    id: id.into(),
                },
                revision: 1,
                value: Some(json!({"title":id})),
            },
        },
    }
}
#[test]
fn native_commit_reopen_and_journal_retention() {
    for redb in [false, true] {
        let path = std::env::temp_dir().join(format!("rom-react-{}-{redb}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let limits = rom::StorageLimits {
            journal_rows: 1,
            receipts: 3,
            ..Default::default()
        };
        let db = database(redb, &path, limits.clone());
        let mut b = bundle("one");
        b.reactions = vec![pending("first")];
        b.reaction_limits = Some(ReactionLimits::default());
        db.commit(&b).unwrap();
        let page = db.journal("things", None, 2, 4096).unwrap();
        assert_eq!(page.events.len(), 1);
        let WorkResult::Claimed(first) = db.reaction_update(WorkUpdate::Claim { now: 0 }).unwrap()
        else {
            panic!()
        };
        drop(db);
        let db = database(redb, &path, limits);
        assert_eq!(
            db.reaction_records().unwrap()[0].pending,
            first.work.pending
        );
        db.commit(&bundle("two")).unwrap();
        assert_eq!(
            db.journal("things", None, 1, 4096),
            Err(rom::Error::HistoryGap)
        );
        assert_eq!(
            db.journal("things", Some(&page.cursor), 1, 4096)
                .unwrap()
                .events[0]
                .row
                .key
                .id,
            "two"
        );
        db.commit(&bundle("three")).unwrap();
        assert_eq!(db.commit(&bundle("four")), Err(rom::Error::Overloaded));
        assert!(db.load(&bundle("four").receipt.row.key).unwrap().is_none());
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
#[test]
fn saturated_work_rejects_entire_upstream_bundle() {
    for redb in [false, true] {
        let path =
            std::env::temp_dir().join(format!("rom-react-cap-{}-{redb}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = database(redb, &path, Default::default());
        let mut b = bundle("one");
        b.reactions = vec![pending("one")];
        b.reaction_limits = Some(ReactionLimits {
            max_records: 1,
            ..Default::default()
        });
        db.commit(&b).unwrap();
        let mut second = bundle("two");
        second.reactions = vec![pending("two")];
        second.reaction_limits = b.reaction_limits.clone();
        assert_eq!(db.commit(&second), Err(rom::Error::Overloaded));
        assert!(db.receipt("two").unwrap().is_none());
        assert!(db.load(&second.receipt.row.key).unwrap().is_none());
        assert_eq!(
            db.journal("things", None, 10, 4096).unwrap().events.len(),
            1
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
#[test]
fn native_precommit_failure_and_postcommit_loss_keep_obligations_atomic() {
    for redb in [false, true] {
        for after in [false, true] {
            let path = std::env::temp_dir().join(format!(
                "rom-react-atomic-{}-{redb}-{after}",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            let hook: std::sync::Arc<dyn Fn(usize) -> rom::Result<()> + Send + Sync> =
                std::sync::Arc::new(move |point| {
                    if point == if after { usize::MAX } else { 0 } {
                        Err(rom::Error::Storage)
                    } else {
                        Ok(())
                    }
                });
            let db: Box<dyn rom::Storage> = if redb {
                let d = rom_redb::Redb::open(&path).unwrap();
                d.on_commit(Some(hook));
                Box::new(d)
            } else {
                let d = rom_sqlite::Sqlite::open(&path).unwrap();
                d.on_commit(Some(hook));
                Box::new(d)
            };
            db.register(&[<Thing as rom::Resource>::descriptor()])
                .unwrap();
            let mut b = bundle("one");
            b.reactions = vec![pending("one")];
            b.reaction_limits = Some(ReactionLimits::default());
            assert_eq!(
                db.commit(&b),
                Err(if after {
                    rom::Error::Unknown
                } else {
                    rom::Error::NotCommitted
                })
            );
            drop(db);
            let db = database(redb, &path, Default::default());
            assert_eq!(db.reaction_records().unwrap().len(), usize::from(after));
            assert_eq!(db.receipt("one").unwrap().is_some(), after);
            assert_eq!(db.load(&b.receipt.row.key).unwrap().is_some(), after);
            assert_eq!(
                db.journal("things", None, 10, 4096).unwrap().events.len(),
                usize::from(after)
            );
            drop(db);
            let _ = std::fs::remove_file(path);
        }
    }
}
