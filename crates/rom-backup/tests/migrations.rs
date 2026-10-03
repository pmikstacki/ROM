use rom::*;
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration, Snapshot, migrate_snapshot};

#[derive(Clone, Resource)]
#[resource(name = "items")]
struct Before {
    title: String,
    count: String,
}
#[derive(Clone, Resource)]
#[resource(name = "items", version = 2)]
struct After {
    label: String,
    count: u64,
    enabled: bool,
}
#[derive(Clone, Resource)]
#[resource(name = "items", version = 3)]
struct Third {
    label: String,
    count: u64,
    enabled: bool,
    note: Option<String>,
}
fn convert(old: Before) -> Result<After> {
    Ok(After {
        label: old.title,
        count: old.count.parse().map_err(|_| Error::Storage)?,
        enabled: true,
    })
}
fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(convert).unwrap(),
    ])
    .unwrap()
}
fn snapshot(work: bool) -> Snapshot {
    let row = Row {
        key: Key {
            kind: "items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(
            Before {
                title: "first".into(),
                count: "42".into(),
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    let receipt = Receipt {
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
            reactions: if work {
                vec![PendingWork {
                    id: "pending-one".into(),
                    cause: Cause {
                        root: "create-one".into(),
                        parent: None,
                        depth: 1,
                        started_at: 0,
                        path: vec!["react".into()],
                    },
                    definition: "react".into(),
                    version: 1,
                    service_key: "service".into(),
                    payload: WorkPayload::Source(row.clone()),
                }]
            } else {
                vec![]
            },
            reaction_limits: work.then(ReactionLimits::default),
            completed_work: None,
        })
        .unwrap();
    Snapshot {
        state,
        rows: vec![row.clone()],
        receipts: vec![receipt],
        events: vec![("create-one".into(), row)],
        effects: vec![],
        descriptors: vec![Before::descriptor().canonical().unwrap()],
        references: vec![],
    }
}
#[test]
fn typed_migration_converts_history_and_preserves_identity_and_codec_version() {
    let input = snapshot(false);
    let before_cursor = input.state.journal_head("items");
    let output = migrate_snapshot(input, &plan(), BackupLimits::default()).unwrap();
    output.validate().unwrap();
    assert_eq!(
        output.descriptors,
        vec![After::descriptor().canonical().unwrap()]
    );
    assert_eq!(
        output.rows[0].value,
        Some(json!({"label":"first", "count":42, "enabled":true}))
    );
    assert_eq!(output.rows[0].revision, 1);
    assert_eq!(output.receipts[0].identity, "create-one");
    assert_eq!(output.receipts[0].fingerprint, "original-input");
    assert_eq!(output.receipts[0].replay_version, Some(1));
    assert_eq!(output.rows[0], output.receipts[0].row);
    assert_eq!(output.events[0].1, output.rows[0]);
    assert_eq!(
        output
            .state
            .journal("items", None, 5, 100_000)
            .unwrap()
            .events[0]
            .row,
        output.rows[0]
    );
    assert_eq!(output.state.journal_head("items"), before_cursor); // Native restore fences, conversion alone does not.
    let next = MigrationPlan::new(vec![
        ResourceMigration::new::<After, Third>(|v| {
            Ok(Third {
                label: v.label,
                count: v.count,
                enabled: v.enabled,
                note: None,
            })
        })
        .unwrap(),
    ])
    .unwrap();
    let third = migrate_snapshot(output, &next, BackupLimits::default()).unwrap();
    assert_eq!(third.receipts[0].replay_version, Some(1));
    assert_eq!(third.descriptors[0].version, 3);
}
#[test]
fn work_requires_explicit_validation_after_source_conversion() {
    assert!(migrate_snapshot(snapshot(true), &plan(), BackupLimits::default()).is_err());
    let accepted = plan().validate_work(|pending| {
        if pending.definition != "react" || pending.version != 1 {
            return Err(Error::Unregistered);
        }
        let WorkPayload::Source(row) = &pending.payload else {
            return Err(Error::Storage);
        };
        After::decode(row.value.clone().unwrap()).map(|_| ())
    });
    let output = migrate_snapshot(snapshot(true), &accepted, BackupLimits::default()).unwrap();
    let work = output.state.work.records();
    assert_eq!(work[0].pending.id, "pending-one");
    assert_eq!(work[0].attempts, 0);
    assert_eq!(work[0].state, WorkState::Pending);
    assert!(
        migrate_snapshot(
            snapshot(true),
            &plan().validate_work(|_| Err(Error::Denied)),
            BackupLimits::default()
        )
        .is_err()
    );
}
#[test]
fn versions_catalog_panics_and_budgets_fail_closed() {
    assert!(ResourceMigration::new::<Before, Third>(|_| unreachable!()).is_err());
    assert!(ResourceMigration::new::<Before, Before>(Ok).is_err());
    assert!(MigrationPlan::new(vec![]).is_err());
    let converted = migrate_snapshot(snapshot(false), &plan(), BackupLimits::default()).unwrap();
    assert!(migrate_snapshot(converted, &plan(), BackupLimits::default()).is_err());
    let panics = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|_| panic!("converter")).unwrap(),
    ])
    .unwrap();
    assert!(matches!(
        migrate_snapshot(snapshot(false), &panics, BackupLimits::default()),
        Err(Error::Panicked)
    ));
    for limits in [
        BackupLimits {
            max_bytes: 1,
            max_records: 100,
        },
        BackupLimits {
            max_bytes: 100_000,
            max_records: 2,
        },
    ] {
        assert!(matches!(
            migrate_snapshot(snapshot(false), &plan(), limits),
            Err(Error::TooLarge)
        ));
    }
    let mut wrong = snapshot(false);
    wrong.descriptors[0].fields[0].name = "different".into();
    assert!(migrate_snapshot(wrong, &plan(), BackupLimits::default()).is_err());
}

#[derive(Clone, Resource)]
#[resource(name = "auxiliary")]
struct AuxiliaryV1 {}
#[derive(Clone, Resource)]
#[resource(name = "auxiliary", version = 2)]
struct AuxiliaryV2 {}
#[test]
fn ordered_steps_preserve_first_receipt_version_across_interleaved_kinds() {
    let mut input = snapshot(false);
    input
        .descriptors
        .push(AuxiliaryV1::descriptor().canonical().unwrap());
    let steps = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(convert).unwrap(),
        ResourceMigration::new::<AuxiliaryV1, AuxiliaryV2>(|_| Ok(AuxiliaryV2 {})).unwrap(),
        ResourceMigration::new::<After, Third>(|v| {
            Ok(Third {
                label: v.label,
                count: v.count,
                enabled: v.enabled,
                note: None,
            })
        })
        .unwrap(),
    ])
    .unwrap();
    let output = migrate_snapshot(input, &steps, BackupLimits::default()).unwrap();
    assert_eq!(output.receipts[0].replay_version, Some(1));
    assert_eq!(
        output
            .descriptors
            .iter()
            .map(|d| (&*d.kind, d.version))
            .collect::<Vec<_>>(),
        vec![("auxiliary", 2), ("items", 3)]
    );
    let wrong = MigrationPlan::new(vec![
        ResourceMigration::new::<After, Third>(|v| {
            Ok(Third {
                label: v.label,
                count: v.count,
                enabled: v.enabled,
                note: None,
            })
        })
        .unwrap(),
    ])
    .unwrap();
    assert!(migrate_snapshot(snapshot(false), &wrong, BackupLimits::default()).is_err());
}
#[test]
fn conversion_memoizes_repeated_values_and_contains_validator_panics_and_growth() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let memo = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| {
            CALLS.fetch_add(1, Ordering::SeqCst);
            convert(v)
        })
        .unwrap(),
    ])
    .unwrap()
    .validate_work(|_| Ok(()));
    migrate_snapshot(snapshot(true), &memo, BackupLimits::default()).unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 1); // Current row, receipt, both journals and work source.
    assert!(matches!(
        migrate_snapshot(
            snapshot(true),
            &plan().validate_work(|_| panic!("validator")),
            BackupLimits::default()
        ),
        Err(Error::Panicked)
    ));
    let grow = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|v| {
            Ok(After {
                label: "x".repeat(16_384),
                count: v.count.parse().unwrap(),
                enabled: true,
            })
        })
        .unwrap(),
    ])
    .unwrap();
    let input = snapshot(false);
    let bytes = serde_json::to_vec(&input).unwrap().len() + 1024;
    assert!(matches!(
        migrate_snapshot(
            input,
            &grow,
            BackupLimits {
                max_bytes: bytes,
                max_records: 100
            }
        ),
        Err(Error::TooLarge)
    ));
}
