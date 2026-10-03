//! One exact frozen action resolver serves operator reconciliation and ordinary workers.
use super::{support::*, wrapper::Wrapped};
use rom::operator::*;
use rom::*;
use std::sync::Arc;
fn builder_with_reaction() -> Builder {
    builder().reaction(Reaction::<Item, Item, String>::new(
        "copy",
        1,
        service(),
        CHANGE,
        |_| Ok(vec![]),
    ))
}
fn seed_committed_action(fixture: &Fixture) {
    let row = Row {
        key: Key {
            kind: Item::KIND.into(),
            id: "target".into(),
        },
        revision: 1,
        value: Some(
            Item {
                secret: "secret-before".into(),
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    let invocation = Invocation {
        retry_epoch: 0,
        kind: Item::KIND.into(),
        id: "target".into(),
        expected: Some(1),
        idempotency: "frozen".into(),
        operation: Operation::Action {
            name: "change".into(),
            input: json!("secret-input"),
        },
    };
    let pending = PendingWork {
        id: "frozen-action-work".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "secret-root".into(),
            parent: None,
            depth: 0,
            started_at: 10,
            path: vec![],
        },
        definition: "copy".into(),
        version: 1,
        service_key: json!(["host", "service", "secret-worker"]).to_string(),
        delivery_profile: DeliveryProfile::AtLeastOnce,
        payload: WorkPayload::Action(serde_json::to_value(invocation).unwrap()),
    };
    fixture
        .store
        .commit(&Bundle {
            expected: None,
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: None,
                identity: "seed".into(),
                fingerprint: "seed".into(),
                row: row.clone(),
            },
            changed: true,
            effects: vec![],
            reactions: vec![pending],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
    let mut committed = row;
    committed.revision = 2;
    committed.value = Some(
        Item {
            secret: "secret-input".into(),
        }
        .encode(),
    );
    let identity = json!([
        "host",
        "service",
        "secret-worker",
        Item::KIND,
        "target",
        ["custom", "change"],
        "frozen"
    ])
    .to_string();
    fixture
        .store
        .commit(&Bundle {
            expected: Some(1),
            receipt: Receipt {
                retry_epoch: 0,
                replay_version: Some(1),
                identity,
                fingerprint: json!([1, "secret-input"]).to_string(),
                row: committed,
            },
            changed: true,
            effects: vec![Intent {
                channel: "legacy-effect".into(),
                payload: json!("secret-effect"),
                delivery_version: None,
            }],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
}
#[tokio::test]
async fn committed_unacknowledged_action_reconciles_without_domain_writes() {
    for redb in [false, true] {
        for operator in [false, true] {
            let fixture = Fixture::new(redb, &[]);
            seed_committed_action(&fixture);
            let runtime = builder_with_reaction()
                .operator_authorizer(Arc::new(Policy::all()))
                .build(fixture.store.clone(), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            let before = (fixture.counts)();
            let snapshot = fixture.snapshot();
            let payload = snapshot.records[0].pending.clone();
            if operator {
                let mut req = request(&snapshot, "resolve");
                req.operation = WorkControlOperation::Reconcile { evidence_ref: None };
                assert_eq!(
                    runtime.work_control(&actor(), req).await.unwrap().outcome,
                    WorkControlOutcome::Completed
                );
            } else {
                assert_eq!(runtime.process_work(1).await.unwrap(), 1);
            }
            assert_eq!((fixture.counts)(), before);
            let after = fixture.snapshot();
            assert_eq!(after.records[0].state, WorkState::Done);
            assert_eq!(after.records[0].pending, payload);
            if operator {
                assert_eq!(after.records[0].attempts, 0);
                assert_eq!(after.roots["secret-root"], 0);
            }
        }
    }
}
#[tokio::test]
async fn same_identity_wrong_fingerprint_or_target_cannot_confirm_frozen_action() {
    for redb in [false, true] {
        for target in [false, true] {
            let fixture = Fixture::new(redb, &[]);
            seed_committed_action(&fixture);
            let mut wrapped = Wrapped::new(fixture.store.clone());
            wrapped.receipt = Some(Arc::new(move |receipt| {
                Ok(receipt.map(|mut receipt| {
                    if target {
                        receipt.row.key.id = "another-target".into();
                    } else {
                        receipt.fingerprint = "secret-wrong-input".into();
                    }
                    receipt
                }))
            }));
            let runtime = builder_with_reaction()
                .operator_authorizer(Arc::new(Policy::all()))
                .build(Arc::new(wrapped), Runtime::shared_cpu_pool(2).unwrap())
                .unwrap();
            let before = fixture.snapshot();
            let counts = (fixture.counts)();
            let mut req = request(&before, "wrong");
            req.operation = WorkControlOperation::Reconcile { evidence_ref: None };
            assert_eq!(
                runtime.work_control(&actor(), req).await,
                Err(Error::IdentityMismatch)
            );
            assert_eq!(fixture.snapshot(), before);
            assert_eq!((fixture.counts)(), counts);
            runtime.process_work(1).await.unwrap();
            assert_eq!(
                fixture.snapshot().records[0].state,
                WorkState::Stopped(StopReason::Invalid)
            );
            assert_eq!((fixture.counts)(), counts);
        }
    }
}
