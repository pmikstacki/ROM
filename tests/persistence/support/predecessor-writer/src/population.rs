//! Data written through the accepted predecessor public storage contract.
use super::Item;
use rom::operator::{WorkControlOperation, WorkControlRequest, WorkHandle};
use rom::*;
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
        payload: WorkPayload::Notification {
            source: row.clone(),
            payload: json!({"name":"retained"}),
        },
    }
}
pub(super) fn populate(store: &dyn Storage) -> (WorkClaim, WorkControlReceipt) {
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
