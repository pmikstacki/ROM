use rom::{
    Bundle, Descriptor, FieldDescriptor, Key, Receipt, ReferenceEdge, Row, Shape, StorageLimits,
    StorageState, json,
};
use rom_backup::{BackupLimits, Collector, Snapshot};

fn snapshot() -> Snapshot {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    let key = Key {
        kind: "items".into(),
        id: "one".into(),
    };
    let row = Row {
        key: key.clone(),
        revision: 1,
        value: Some(json!({"link":"one"})),
        protected: Default::default(),
    };
    let receipt = Receipt {
        identity: "create-one".into(),
        fingerprint: "input".into(),
        row: row.clone(),
    };
    state
        .bundle(&Bundle {
            expected: None,
            receipt: receipt.clone(),
            changed: true,
            effects: vec![],
            reactions: vec![],
            reaction_limits: None,
            completed_work: None,
        })
        .unwrap();
    Snapshot {
        state,
        rows: vec![row.clone()],
        receipts: vec![receipt],
        events: vec![("create-one".into(), row)],
        effects: vec![],
        descriptors: vec![Descriptor {
            kind: "items".into(),
            version: 1,
            fields: vec![FieldDescriptor {
                name: "link".into(),
                shape: Shape::Reference {
                    kind: "items".into(),
                },
            }],
        }],
        references: vec![ReferenceEdge {
            source: key.clone(),
            target: key,
        }],
    }
}

#[test]
fn archive_requires_exact_live_reference_graph() {
    snapshot().validate().unwrap();
    let mut absent = snapshot();
    absent.references.clear();
    assert!(absent.validate().is_err());
    let mut extra = snapshot();
    extra.references.push(extra.references[0].clone());
    assert!(extra.validate().is_err());
    let mut wrong = snapshot();
    wrong.references[0].target.id = "absent".into();
    assert!(wrong.validate().is_err());
    let mut unbound = snapshot();
    unbound.descriptors.clear();
    assert!(unbound.validate().is_err());
}

#[test]
fn exact_edges_do_not_make_dangling_references_valid() {
    let mut dangling = snapshot();
    dangling.rows[0].value = Some(json!({"link":"absent"}));
    dangling.receipts[0].row = dangling.rows[0].clone();
    dangling.events[0].1 = dangling.rows[0].clone();
    dangling.references[0].target.id = "absent".into();
    assert!(dangling.validate().is_err());
}

#[test]
fn collector_charges_schema_and_edge_records() {
    let snapshot = snapshot();
    let state = serde_json::to_string(&snapshot.state).unwrap();
    let descriptor = serde_json::to_string(&snapshot.descriptors[0]).unwrap();
    let mut collector = Collector::new(
        &state,
        BackupLimits {
            max_bytes: 100_000,
            max_records: 1,
        },
    )
    .unwrap();
    collector.descriptor("items", &descriptor).unwrap();
    assert_eq!(
        collector.reference(snapshot.references[0].clone()),
        Err(rom::Error::TooLarge)
    );
    let mut collector = Collector::new(&state, BackupLimits::default()).unwrap();
    assert!(collector.descriptor("wrong", &descriptor).is_err());
}
