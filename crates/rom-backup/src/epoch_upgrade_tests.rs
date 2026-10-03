//! Epoch-aware predecessor archives preserve metadata instead of rebinding it.
use crate::tests::{legacy_manifest, legacy_state, unchecked_archive};
use crate::*;
use rom::{
    Bundle, Cause, Descriptor, Error, FieldDescriptor, Key, PendingWork, ReactionLimits, Receipt,
    RetryEpochs, Row, Shape, StorageLimits, StorageState, WorkPayload, json,
};

fn snapshot(origin: Option<u32>) -> Snapshot {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    state
        .apply_retention(
            RetryEpochs {
                current: 4,
                admission_floor: 2,
                replay_floor: 1,
            },
            0,
            0,
            0,
        )
        .unwrap();
    let row = Row {
        key: Key {
            kind: "items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(json!({"name":"unchanged"})),
        protected: Default::default(),
    };
    let receipt = Receipt {
        retry_epoch: 3,
        replay_version: origin,
        identity: "original".into(),
        fingerprint: "retained".into(),
        row: row.clone(),
    };
    state
        .bundle(&Bundle {
            expected: None,
            receipt: receipt.clone(),
            changed: true,
            effects: vec![],
            reactions: vec![PendingWork {
                delivery_profile: rom::DeliveryProfile::AtLeastOnce,
                id: "pending".into(),
                cause: Cause {
                    retry_epoch: 3,
                    root: "original".into(),
                    parent: None,
                    depth: 0,
                    started_at: 1,
                    path: vec![],
                },
                definition: "reaction".into(),
                version: 1,
                service_key: "worker".into(),
                payload: WorkPayload::Source(row.clone()),
            }],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
    Snapshot {
        state,
        rows: vec![row.clone()],
        receipts: vec![receipt],
        events: vec![("original".into(), row)],
        effects: vec![],
        references: vec![],
        descriptors: vec![Descriptor {
            kind: "items".into(),
            version: 2,
            fields: vec![FieldDescriptor {
                name: "name".into(),
                shape: Shape::String,
            }],
        }],
    }
}

#[test]
fn archive_four_requires_explicit_upgrade_and_preserves_epochs_and_origins() {
    for backend in [Backend::Sqlite, Backend::Redb] {
        for (archive_version, storage_format, origin) in
            [(4, 6, None), (4, 6, Some(1)), (5, 7, None), (5, 7, Some(1))]
        {
            let target = std::env::temp_dir().join(format!(
                "rom-epoch-upgrade-{}-{backend:?}-{origin:?}-{archive_version}",
                std::process::id()
            ));
            let source = Stage::new(&target.with_extension("source")).unwrap();
            let snapshot = snapshot(origin);
            snapshot.validate().unwrap();
            let mut manifest = snapshot.manifest(backend);
            manifest.archive_version = archive_version;
            manifest.storage_format = storage_format;
            let mut manifest = serde_json::to_value(manifest).unwrap();
            legacy_manifest(&mut manifest);
            let before = serde_json::to_value(&snapshot).unwrap();
            let mut legacy = before.clone();
            legacy_state(&mut legacy["state"]);
            unchecked_archive(source.path(), &manifest, &legacy);
            let original = std::fs::read(source.path()).unwrap();
            assert!(matches!(
                read(source.path(), backend, BackupLimits::default()),
                Err(Error::Unsupported(_))
            ));
            let upgrade = if archive_version == 4 {
                upgrade_v4_archive
            } else {
                upgrade_v5_archive
            };
            let manifest =
                upgrade(source.path(), &target, backend, BackupLimits::default()).unwrap();
            assert_eq!((manifest.archive_version, manifest.storage_format), (6, 8));
            let (_, after) = read(&target, backend, BackupLimits::default()).unwrap();
            assert_eq!(serde_json::to_value(after).unwrap(), before);
            assert!(
                std::fs::read(source.path()).unwrap() == original,
                "source archive changed"
            );
            assert_eq!(
                upgrade(source.path(), &target, backend, BackupLimits::default()),
                Err(Error::Conflict)
            );
            std::fs::remove_file(target).unwrap();
        }
    }
}

#[test]
fn legacy_decoder_rejects_injected_recovery_metadata() {
    let mut genuine = serde_json::to_value(snapshot(Some(1)).state).unwrap();
    legacy_state(&mut genuine);
    for field in ["operator", "revision", "delivery_profile", "hold"] {
        let mut invalid = genuine.clone();
        match field {
            "operator" => {
                invalid["operator"] =
                    json!({"limits":{"max_records":1,"max_bytes":1024},"receipts":{}})
            }
            "revision" => invalid["work"]["work"]["pending"]["revision"] = json!(0),
            "delivery_profile" => {
                invalid["work"]["work"]["pending"]["pending"]["delivery_profile"] =
                    json!("AtLeastOnce")
            }
            "hold" => invalid["work"]["work"]["pending"]["state"] = json!("AwaitingReconciliation"),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_string(&invalid).unwrap();
        assert!(
            matches!(
                Collector::legacy(&bytes, BackupLimits::default()),
                Err(Error::Storage)
            ),
            "injected {field}"
        );
    }
    let upgraded = decode_legacy_storage_state(genuine).unwrap();
    assert_eq!(upgraded.work.records()[0].revision, 0);
    assert_eq!(
        upgraded.work.records()[0].pending.delivery_profile,
        rom::DeliveryProfile::AtLeastOnce
    );
    assert_eq!(upgraded.operator_receipt_count(), 0);
}

#[test]
fn current_decoder_requires_each_recovery_metadata_field() {
    let current = serde_json::to_value(snapshot(Some(1)).state).unwrap();
    for field in ["operator", "revision", "delivery_profile"] {
        let mut invalid = current.clone();
        match field {
            "operator" => {
                invalid.as_object_mut().unwrap().remove(field);
            }
            "revision" => {
                invalid["work"]["work"]["pending"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            "delivery_profile" => {
                invalid["work"]["work"]["pending"]["pending"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                Collector::new(
                    &serde_json::to_string(&invalid).unwrap(),
                    BackupLimits::default()
                ),
                Err(Error::Storage)
            ),
            "absent {field}"
        );
    }
}

fn operator_snapshot() -> (Snapshot, rom::StorageWorkControl) {
    use rom::operator::{WorkControlOperation, WorkControlRequest, WorkHandle};
    let mut snapshot = snapshot(Some(1));
    let view = snapshot.state.work_snapshot(16, 65536).unwrap();
    let control = rom::StorageWorkControl {
        principal: "operator".into(),
        request: WorkControlRequest {
            handle: WorkHandle::from_work_id("pending"),
            expected: view.version(&view.records[0]),
            key: "retry".into(),
            retry_epoch: 4,
            operation: WorkControlOperation::Retry,
        },
        decision: rom::WorkControlDecision::Retry,
        now: 2,
    };
    snapshot.state.control_work(&control).unwrap();
    (snapshot, control)
}

#[test]
fn operator_receipts_count_toward_collector_archive_and_maintenance_bounds() {
    let (snapshot, _) = operator_snapshot();
    let state = serde_json::to_string(&snapshot.state).unwrap();
    let limits = BackupLimits {
        max_records: 1,
        ..BackupLimits::default()
    };
    assert!(matches!(
        Collector::new(&state, limits),
        Err(Error::TooLarge)
    ));
    let manifest = snapshot.manifest(Backend::Sqlite);
    assert_eq!(manifest.operator_receipts, 1);
    let limits = BackupLimits {
        max_records: 5,
        ..BackupLimits::default()
    };
    assert_eq!(
        crate::archive::check_count(&manifest, limits),
        Err(Error::TooLarge)
    );
    let descriptors = snapshot.descriptors.clone();
    assert!(matches!(
        upgrade_current_snapshot(snapshot, &descriptors, limits),
        Err(Error::TooLarge)
    ));
}

#[test]
fn archive_restore_and_retention_preserve_historical_operator_receipts() {
    let (mut snapshot, control) = operator_snapshot();
    let accepted = snapshot.state.operator.clone();
    let rom::WorkResult::Claimed(claim) = snapshot
        .state
        .update_work(rom::WorkUpdate::Claim { now: 2 })
        .unwrap()
    else {
        panic!("work must be claimable")
    };
    snapshot
        .state
        .update_work(rom::WorkUpdate::Finish {
            claim: claim.key(),
            now: 2,
            outcome: rom::WorkOutcome::Done,
        })
        .unwrap();
    let policy = RetentionPolicy::new(RetryEpochs {
        current: 5,
        admission_floor: 5,
        replay_floor: 4,
    });
    let (mut retained, report) =
        retain_snapshot(snapshot, &policy, BackupLimits::default()).unwrap();
    assert_eq!(report.work_records_removed, 1);
    assert!(retained.state.work.records().is_empty());
    assert_eq!(retained.state.operator, accepted);
    retained.state.prepare_restore().unwrap();
    assert_eq!(retained.state.operator, accepted);
    assert!(
        retained
            .state
            .control_work(&control)
            .unwrap()
            .result
            .replayed
    );
    let target = std::env::temp_dir().join(format!(
        "rom-operator-receipt-archive-{}",
        std::process::id()
    ));
    let manifest = write(&target, Backend::Sqlite, &retained, BackupLimits::default()).unwrap();
    assert_eq!(manifest.operator_receipts, 1);
    let (_, reopened) = read(&target, Backend::Sqlite, BackupLimits::default()).unwrap();
    assert_eq!(reopened.state.operator, accepted);
    std::fs::remove_file(target).unwrap();
}

#[test]
fn current_archive_and_restore_preserve_reconciliation_hold_and_profile() {
    let mut input = snapshot(Some(1));
    let mut state = serde_json::to_value(&input.state).unwrap();
    let record = &mut state["work"]["work"]["pending"];
    record["pending"]["payload"] = json!({"Notification": {
        "source": input.rows[0], "payload": {"provider":"protected"}
    }});
    record["pending"]["delivery_profile"] = json!("ReconcileBeforeRetry");
    record["state"] = json!("AwaitingReconciliation");
    record["attempts"] = json!(2);
    record["generation"] = json!(2);
    record["revision"] = json!(17);
    record["delivery"] = json!("TimedOut");
    state["work"]["roots"]["original"] = json!(2);
    input.state = serde_json::from_value(state).unwrap();
    let before = input.state.work.records();
    let old_view = input.state.work_snapshot(16, 65536).unwrap();
    let target =
        std::env::temp_dir().join(format!("rom-reconciliation-archive-{}", std::process::id()));
    write(&target, Backend::Sqlite, &input, BackupLimits::default()).unwrap();
    let (_, mut restored) = read(&target, Backend::Sqlite, BackupLimits::default()).unwrap();
    assert_eq!(restored.state.work.records(), before);
    restored.state.prepare_restore().unwrap();
    assert_eq!(restored.state.work.records(), before);
    assert_ne!(
        restored.state.work_snapshot(16, 65536).unwrap().generation,
        old_view.generation
    );
    std::fs::remove_file(target).unwrap();
}
#[test]
fn current_snapshot_upgrade_checks_catalog_integrity_and_complete_bounds() {
    let input = snapshot(None);
    let expected = serde_json::to_value(&input).unwrap();
    let descriptors = input.descriptors.clone();
    let output = upgrade_current_snapshot(input, &descriptors, BackupLimits::default()).unwrap();
    assert_eq!(serde_json::to_value(output).unwrap(), expected);
    assert!(matches!(
        upgrade_current_snapshot(snapshot(None), &[], BackupLimits::default()),
        Err(Error::Unsupported(_))
    ));
    let mut corrupt = snapshot(Some(1));
    corrupt.receipts[0].retry_epoch = 5;
    assert!(matches!(
        upgrade_current_snapshot(corrupt, &descriptors, BackupLimits::default()),
        Err(Error::Storage)
    ));
    for limits in [
        BackupLimits {
            max_records: 1,
            ..BackupLimits::default()
        },
        BackupLimits {
            max_bytes: 1,
            ..BackupLimits::default()
        },
    ] {
        assert!(matches!(
            upgrade_current_snapshot(snapshot(None), &descriptors, limits),
            Err(Error::TooLarge)
        ));
    }
    assert!(matches!(
        upgrade_legacy_snapshot(snapshot(Some(1)), &descriptors, BackupLimits::default()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn physical_records_share_logical_collector_budgets_without_entering_snapshot() {
    let state =
        serde_json::to_string(&StorageState::new(StorageLimits::default()).unwrap()).unwrap();
    let descriptor = Descriptor {
        kind: "items".into(),
        version: 1,
        fields: vec![],
    };
    let text = serde_json::to_string(&descriptor).unwrap();
    let total = state.len() + text.len() + "items".len() + 9;
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: total,
            max_records: 2,
        },
    )
    .unwrap();
    collect.descriptor("items", &text).unwrap();
    let original = serde_json::to_value(&collect.snapshot).unwrap();
    collect.physical(9).unwrap();
    assert_eq!(serde_json::to_value(&collect.snapshot).unwrap(), original);
    assert_eq!(collect.physical(0), Err(Error::TooLarge));
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: total - 1,
            max_records: 3,
        },
    )
    .unwrap();
    collect.physical(9).unwrap();
    assert_eq!(collect.descriptor("items", &text), Err(Error::TooLarge));
    let mut collect = Collector::new(
        &state,
        BackupLimits {
            max_bytes: usize::MAX,
            max_records: usize::MAX,
        },
    )
    .unwrap();
    assert_eq!(collect.physical(usize::MAX), Err(Error::TooLarge));
}

#[test]
fn archive_four_upgrades_genuine_legacy_work_without_current_metadata() {
    let target =
        std::env::temp_dir().join(format!("rom-legacy-work-upgrade-{}", std::process::id()));
    let source = Stage::new(&target.with_extension("source")).unwrap();
    let input = snapshot(Some(1));
    let mut manifest = serde_json::to_value(input.manifest(Backend::Sqlite)).unwrap();
    manifest["archive_version"] = json!(4);
    manifest["storage_format"] = json!(6);
    manifest
        .as_object_mut()
        .unwrap()
        .remove("operator_receipts");
    let mut body = serde_json::to_value(&input).unwrap();
    body["state"].as_object_mut().unwrap().remove("operator");
    for record in body["state"]["work"]["work"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        record.as_object_mut().unwrap().remove("revision");
        record["pending"]
            .as_object_mut()
            .unwrap()
            .remove("delivery_profile");
    }
    unchecked_archive(source.path(), &manifest, &body);
    upgrade_v4_archive(
        source.path(),
        &target,
        Backend::Sqlite,
        BackupLimits::default(),
    )
    .unwrap();
    let (_, output) = read(&target, Backend::Sqlite, BackupLimits::default()).unwrap();
    let record = &output.state.work.records()[0];
    assert_eq!(record.revision, 0);
    assert_eq!(output.state.retry_epochs(), input.state.retry_epochs());
    assert_eq!(output.receipts, input.receipts);
    std::fs::remove_file(target).unwrap();
}
