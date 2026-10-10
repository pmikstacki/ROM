//! Fresh complete ledgers for the additive atomic Storage contract.
use rom::*;

pub fn limits() -> ReactionLimits {
    ReactionLimits::default()
}
pub fn seed(storage: &dyn Storage) {
    storage
        .register(&[Descriptor {
            kind: "batch-source".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "enabled".into(),
                shape: Shape::Bool,
            }],
        }])
        .unwrap();
    for id in ["a", "b"] {
        let row = Row {
            key: Key {
                kind: "batch-source".into(),
                id: id.into(),
            },
            revision: 1,
            value: Some(json!({"enabled":true})),
            protected: Default::default(),
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
                    row: row.clone(),
                },
                reaction_limits: Some(limits()),
                reactions: vec![PendingWork {
                    id: id.into(),
                    cause: Cause {
                        retry_epoch: 0,
                        root: format!("root-{id}"),
                        parent: None,
                        depth: 1,
                        started_at: 0,
                        path: vec![id.into()],
                    },
                    definition: "batch-mapper".into(),
                    version: 1,
                    service_key: "batch-service".into(),
                    delivery_profile: DeliveryProfile::AtLeastOnce,
                    not_before: None,
                    payload: WorkPayload::Source(row),
                }],
            })
            .unwrap();
    }
}
