//! The same atomic operator contract on both maintained native adapters.
#[path = "operator/support.rs"]
mod support;
use rom::operator::*;
use rom::*;
use std::sync::Arc;
use support::{Fixture, control, receipt_count};

#[test]
fn native_control_cas_and_exact_replay_preserve_domain_state_and_budgets() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let before = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
        let command = control(&before, "operator-one", "retry-one");
        let result = fixture.db.storage().control_work(&command).unwrap();
        assert!(!result.result.replayed);
        assert_eq!(result.result.outcome, WorkControlOutcome::Scheduled);
        let changed = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
        assert_eq!(receipt_count(&changed), 1);
        assert_eq!(changed.records[0].attempts, before.records[0].attempts);
        assert_eq!(changed.records[0].pending, before.records[0].pending);
        assert!(changed.records[0].revision > before.records[0].revision);
        assert!(changed.records[0].generation > before.records[0].generation);
        assert_eq!(fixture.db.counts(), [1, 1, 1, 0]);
        let replay = fixture.db.storage().control_work(&command).unwrap();
        assert!(replay.result.replayed);
        assert_eq!(replay.result.version, result.result.version);
        assert_eq!(
            fixture.db.storage().work_snapshot(32, 100_000).unwrap(),
            changed
        );
        let mut different = command.clone();
        different.request.expected.revision += 1;
        assert_eq!(
            fixture.db.storage().control_work(&different),
            Err(Error::IdentityMismatch)
        );
        different.request.key = "another-key".into();
        different.request.expected = command.request.expected.clone();
        assert_eq!(
            fixture.db.storage().control_work(&different),
            Err(Error::Conflict)
        );
        different.principal = "operator-two".into();
        different.request.key = command.request.key.clone();
        assert_eq!(
            fixture.db.storage().control_work(&different),
            Err(Error::Conflict)
        );
        assert_eq!(
            fixture.db.storage().work_snapshot(32, 100_000).unwrap(),
            changed
        );
    }
}

#[test]
fn native_control_failures_do_not_split_work_and_receipt_publication() {
    for redb in [false, true] {
        for checkpoint in [usize::MAX - 1, 1, 0, usize::MAX] {
            let fixture = Fixture::new(redb);
            let before = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            let command = control(&before, "operator", "atomic-retry");
            fixture.db.observe(Some(Arc::new(move |point| {
                if point == checkpoint {
                    Err(Error::NotCommitted)
                } else {
                    Ok(())
                }
            })));
            let result = fixture.db.storage().control_work(&command);
            fixture.db.observe(None);
            let after = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            if checkpoint == usize::MAX {
                assert_eq!(result, Err(Error::Unknown));
                assert_eq!(receipt_count(&after), 1);
                let replay = fixture.db.storage().control_work(&command).unwrap();
                assert!(replay.result.replayed);
                assert_eq!(replay.result.version.revision, after.records[0].revision);
            } else {
                assert_eq!(result, Err(Error::NotCommitted));
                assert_eq!(after, before);
            }
            assert_eq!(fixture.db.counts(), [1, 1, 1, 0]);
        }
    }
}

#[test]
fn persisted_control_receipt_survives_reopen_and_snapshot_bounds_are_exact() {
    for redb in [false, true] {
        let mut fixture = Fixture::new(redb);
        let before = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
        let command = control(&before, "operator", "restart-retry");
        let committed = fixture.db.storage().control_work(&command).unwrap();
        fixture.reopen();
        let replay = fixture.db.storage().control_work(&command).unwrap();
        assert!(replay.result.replayed);
        assert_eq!(replay.result.version, committed.result.version);
        let snapshot = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
        let bytes = serde_json::to_vec(&snapshot).unwrap().len();
        assert_eq!(
            fixture.db.storage().work_snapshot(2, bytes).unwrap(),
            snapshot
        );
        assert_eq!(
            fixture.db.storage().work_snapshot(1, bytes),
            Err(Error::TooLarge)
        );
        assert_eq!(
            fixture.db.storage().work_snapshot(2, bytes - 1),
            Err(Error::TooLarge)
        );
        assert_eq!(fixture.db.counts(), [1, 1, 1, 0]);
    }
}

#[test]
fn concurrent_controls_arbitrate_version_or_replay_the_same_identity() {
    for redb in [false, true] {
        for same_key in [false, true] {
            let fixture = Fixture::new(redb);
            let before = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            let first = control(&before, "operator", "first");
            let second = control(
                &before,
                "operator",
                if same_key { "first" } else { "second" },
            );
            let barrier = std::sync::Barrier::new(2);
            let db = &fixture.db;
            let gate = &barrier;
            let outcomes = std::thread::scope(|scope| {
                let start = |request| {
                    scope.spawn(move || {
                        gate.wait();
                        db.storage().control_work(request)
                    })
                };
                let left = start(&first);
                let right = start(&second);
                [left.join().unwrap(), right.join().unwrap()]
            });
            if same_key {
                assert!(outcomes.iter().all(Result::is_ok));
                assert_eq!(
                    outcomes
                        .iter()
                        .filter(|r| r.as_ref().unwrap().result.replayed)
                        .count(),
                    1
                );
            } else {
                assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
                assert_eq!(
                    outcomes
                        .iter()
                        .filter(|r| matches!(r, Err(Error::Conflict)))
                        .count(),
                    1
                );
            }
            let after = fixture.db.storage().work_snapshot(32, 100_000).unwrap();
            assert_eq!(receipt_count(&after), 1);
            assert_eq!(after.records[0].revision, before.records[0].revision + 1);
            assert_eq!(after.records[0].pending, before.records[0].pending);
            assert_eq!(fixture.db.counts(), [1, 1, 1, 0]);
        }
    }
}

#[path = "operator/process.rs"]
mod process;
