use rom::*;
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration, Snapshot, migrate_snapshot};

#[derive(Clone)]
struct Folded(String);
impl Field for Folded {
    fn shape() -> Shape {
        Shape::String
    }
    fn encode(&self) -> Value {
        json!(self.0)
    }
    fn decode(value: Value) -> Result<Self> {
        value
            .as_str()
            .map(|text| Self(text.to_lowercase()))
            .ok_or_else(|| Error::invalid("folded", "text"))
    }
}
#[derive(Clone, Resource)]
#[resource(name = "codec-items")]
struct PlainBefore {
    text: String,
}
#[derive(Clone, Resource)]
#[resource(name = "codec-items")]
struct FoldedBefore {
    text: Folded,
}
#[derive(Clone, Resource)]
#[resource(name = "codec-items", version = 2)]
struct PlainAfter {
    text: String,
}
#[derive(Clone, Resource)]
#[resource(name = "codec-items", version = 2)]
struct FoldedAfter {
    text: Folded,
}

fn snapshot(descriptor: Descriptor, value: Value) -> Snapshot {
    let row = Row {
        key: Key {
            kind: "codec-items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(value),
        protected: Default::default(),
    };
    let receipt = Receipt {
        retry_epoch: 0,
        identity: "create-one".into(),
        fingerprint: "original-input".into(),
        row: row.clone(),
        replay_version: None,
    };
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
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
        descriptors: vec![descriptor.canonical().unwrap()],
        references: vec![],
    }
}

#[test]
fn migration_rejects_target_bytes_that_decode_to_a_different_value() {
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<PlainBefore, FoldedAfter>(|before| {
            Ok(FoldedAfter {
                text: Folded(before.text),
            })
        })
        .unwrap(),
    ])
    .unwrap();
    let input = snapshot(PlainBefore::descriptor(), json!({"text": "UPPER"}));
    input.validate().unwrap();
    assert!(matches!(
        migrate_snapshot(input, &plan, BackupLimits::default()),
        Err(Error::Invalid { .. })
    ));
}

#[test]
fn migration_rejects_noncanonical_source_before_running_converter() {
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<FoldedBefore, PlainAfter>(|_| {
            panic!("noncanonical history must be rejected before user conversion")
        })
        .unwrap(),
    ])
    .unwrap();
    let input = snapshot(FoldedBefore::descriptor(), json!({"text": "UPPER"}));
    input.validate().unwrap();
    assert!(matches!(
        migrate_snapshot(input, &plan, BackupLimits::default()),
        Err(Error::Invalid { .. })
    ));
}

#[test]
fn canonical_custom_codecs_still_migrate() {
    let plan = MigrationPlan::new(vec![
        ResourceMigration::new::<FoldedBefore, FoldedAfter>(|before| {
            Ok(FoldedAfter { text: before.text })
        })
        .unwrap(),
    ])
    .unwrap();
    let output = migrate_snapshot(
        snapshot(FoldedBefore::descriptor(), json!({"text": "lower"})),
        &plan,
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(output.rows[0].value, Some(json!({"text": "lower"})));
    output.validate().unwrap();
}
