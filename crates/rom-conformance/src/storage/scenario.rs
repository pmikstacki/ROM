use rom::{Bundle, Intent, Key, Receipt, Resource, Row, json};

#[derive(Clone, Resource)]
#[resource(name = "records")]
pub(super) struct Record {
    pub done: bool,
}
pub(super) fn create() -> Bundle {
    Bundle {
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
        expected: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: "create-one".into(),
            fingerprint: "create-one-false".into(),
            row: Row {
                protected: Default::default(),
                key: Key {
                    kind: "records".into(),
                    id: "one".into(),
                },
                revision: 1,
                value: Some(json!({"done":false})),
            },
        },
        changed: true,
        effects: vec![],
    }
}
pub(super) fn update() -> Bundle {
    let mut bundle = create();
    bundle.expected = Some(1);
    bundle.receipt.identity = "update-one".into();
    bundle.receipt.fingerprint = "update-one-true".into();
    bundle.receipt.row.revision = 2;
    bundle.receipt.row.value = Some(json!({"done":true}));
    bundle.effects = vec![
        Intent::new("mail", json!({"value":1})),
        Intent::new("audit", json!({"value":2})),
    ];
    bundle
}
