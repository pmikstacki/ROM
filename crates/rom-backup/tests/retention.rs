use rom::*;
use rom_backup::{BackupLimits, RetentionPolicy, Snapshot, StoredEffect, retain_snapshot};

#[derive(Clone, Resource)]
#[resource(name = "items")]
struct Item {
    label: String,
}

fn expired() -> RetryEpochs {
    RetryEpochs {
        current: 1,
        admission_floor: 1,
        replay_floor: 1,
    }
}
fn snapshot(deleted: bool, effect: bool) -> Snapshot {
    let mut state = StorageState::new(StorageLimits::default()).unwrap();
    let mut receipts = Vec::new();
    let mut events = Vec::new();
    let mut effects = Vec::new();
    for (index, changed) in [true, true, false].into_iter().enumerate() {
        let row = Row {
            key: Key {
                kind: "items".into(),
                id: "one".into(),
            },
            revision: if index == 0 { 1 } else { 2 },
            value: if deleted && index > 0 {
                None
            } else {
                Some(json!({"label": if index == 0 {"before"} else {"after"}}))
            },
            protected: Default::default(),
        };
        let receipt = Receipt {
            identity: format!("request-{index}"),
            fingerprint: format!("input-{index}"),
            row: row.clone(),
            replay_version: Some(1),
            retry_epoch: 0,
        };
        let intents = if effect && index == 0 {
            vec![Intent {
                channel: "email".into(),
                payload: json!({"private":true}),
                delivery_version: None,
            }]
        } else {
            Vec::new()
        };
        state
            .bundle(&Bundle {
                expected: (index > 0).then_some(if index == 1 { 1 } else { 2 }),
                receipt: receipt.clone(),
                changed,
                effects: intents.clone(),
                reactions: Vec::new(),
                reaction_limits: None,
                completed_work: None,
            })
            .unwrap();
        if changed {
            events.push((receipt.identity.clone(), row));
        }
        for (ordinal, intent) in intents.into_iter().enumerate() {
            effects.push(StoredEffect {
                identity: receipt.identity.clone(),
                ordinal: ordinal as u64,
                intent,
            });
        }
        receipts.push(receipt);
    }
    let rows = vec![receipts.last().unwrap().row.clone()];
    let snapshot = Snapshot {
        state,
        rows,
        receipts,
        events,
        effects,
        descriptors: vec![Item::descriptor().canonical().unwrap()],
        references: Vec::new(),
    };
    snapshot.validate().unwrap();
    snapshot
}

#[test]
fn expiry_reclaims_noops_but_keeps_one_current_proof_and_journal_gap() {
    let original = snapshot(false, false);
    let cursor = original.state.journal_head("items");
    let (retained, report) = retain_snapshot(
        original,
        &RetentionPolicy::new(expired()).journal_through(2),
        BackupLimits::default(),
    )
    .unwrap();
    retained.validate().unwrap();
    assert_eq!(report.receipts_removed, 2);
    assert_eq!(report.events_removed, 2);
    assert_eq!(retained.receipts[0].identity, "request-1");
    assert_eq!(retained.rows[0].revision, 2);
    assert!(retained.events.is_empty());
    assert_eq!(retained.state.retry_epochs(), expired());
    assert_eq!(retained.state.journal_head("items"), cursor);
    assert!(matches!(
        retained.state.journal("items", None, 10, 100_000),
        Err(Error::HistoryGap)
    ));
    assert!(matches!(
        retained.state.check_retry_epoch(0, true, None),
        Err(Error::IdentityExpired)
    ));
    let (again, second) = retain_snapshot(
        retained,
        &RetentionPolicy::new(expired()),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(second.receipts_removed, 0);
    assert_eq!(again.receipts.len(), 1);
}

#[test]
fn retained_journal_and_unsettled_effects_pin_history() {
    let (with_journal, report) = retain_snapshot(
        snapshot(false, false),
        &RetentionPolicy::new(expired()),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(with_journal.events.len(), 2);
    assert_eq!(report.receipts_removed, 1);
    let policy = RetentionPolicy::new(expired()).journal_through(2);
    let (with_effect, report) =
        retain_snapshot(snapshot(false, true), &policy, BackupLimits::default()).unwrap();
    assert_eq!(report.effects_removed, 0);
    assert_eq!(with_effect.receipts.len(), 2);
    let (settled, report) = retain_snapshot(
        with_effect,
        &policy.settle_effects("request-0"),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(report.effects_removed, 1);
    assert!(settled.effects.is_empty());
    assert_eq!(settled.receipts.len(), 1);
}

#[test]
fn sealing_keeps_retry_history_and_explicit_purge_rejects_dependencies() {
    let sealed = RetryEpochs {
        current: 1,
        admission_floor: 1,
        replay_floor: 0,
    };
    let (retained, report) = retain_snapshot(
        snapshot(false, false),
        &RetentionPolicy::new(sealed).journal_through(2),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(report.receipts_removed, 0);
    assert_eq!(retained.receipts.len(), 3);
    let key = retained.rows[0].key.clone();
    assert!(
        retain_snapshot(
            retained,
            &RetentionPolicy::new(expired()).purge_tombstone(key.clone()),
            BackupLimits::default()
        )
        .is_err()
    );
    assert!(
        retain_snapshot(
            snapshot(true, false),
            &RetentionPolicy::new(expired()).purge_tombstone(key.clone()),
            BackupLimits::default()
        )
        .is_err()
    );
    assert!(
        retain_snapshot(
            snapshot(true, false),
            &RetentionPolicy::new(sealed)
                .journal_through(2)
                .purge_tombstone(key.clone()),
            BackupLimits::default()
        )
        .is_err()
    );
    let (purged, report) = retain_snapshot(
        snapshot(true, false),
        &RetentionPolicy::new(expired())
            .journal_through(2)
            .purge_tombstone(key),
        BackupLimits::default(),
    )
    .unwrap();
    purged.validate().unwrap();
    assert_eq!(report.tombstones_removed, 1);
    assert_eq!(report.receipts_removed, 3);
    assert!(purged.rows.is_empty());
    assert!(purged.receipts.is_empty());
}

#[test]
fn invalid_policy_and_limits_reject_without_partial_result() {
    let policies = [
        RetentionPolicy::new(RetryEpochs {
            current: 0,
            admission_floor: 1,
            replay_floor: 0,
        }),
        RetentionPolicy::new(expired()).journal_through(3),
        RetentionPolicy::new(expired()).settle_effects("absent"),
        RetentionPolicy::new(RetryEpochs::default()).settle_effects("request-0"),
    ];
    for policy in policies {
        assert!(retain_snapshot(snapshot(false, true), &policy, BackupLimits::default()).is_err());
    }
    for limits in [
        BackupLimits {
            max_bytes: 1,
            max_records: 100,
        },
        BackupLimits {
            max_bytes: 100_000,
            max_records: 1,
        },
    ] {
        assert!(matches!(
            retain_snapshot(
                snapshot(false, false),
                &RetentionPolicy::new(expired()),
                limits
            ),
            Err(Error::TooLarge)
        ));
    }
}

#[test]
fn unfinished_root_blocks_settlement_and_any_retained_work_blocks_purge() {
    let mut original = snapshot(true, true);
    let work = PendingWork {
        delivery_profile: rom::DeliveryProfile::AtLeastOnce,
        id: "work".into(),
        cause: Cause {
            retry_epoch: 0,
            root: "request-0".into(),
            parent: None,
            depth: 0,
            started_at: 0,
            path: Vec::new(),
        },
        definition: "notification".into(),
        version: 1,
        service_key: "service".into(),
        payload: WorkPayload::Notification {
            source: original.receipts[0].row.clone(),
            payload: json!({"private":true}),
        },
    };
    original
        .state
        .work
        .enqueue(&ReactionLimits::default(), vec![work])
        .unwrap();
    let policy = RetentionPolicy::new(expired())
        .journal_through(2)
        .settle_effects("request-0");
    assert!(matches!(
        retain_snapshot(original, &policy, BackupLimits::default()),
        Err(Error::Conflict)
    ));

    let mut original = snapshot(true, false);
    original
        .state
        .apply_retention(
            RetryEpochs {
                current: 1,
                admission_floor: 0,
                replay_floor: 0,
            },
            0,
            3,
            0,
        )
        .unwrap();
    original
        .state
        .work
        .enqueue(
            &ReactionLimits::default(),
            vec![PendingWork {
                delivery_profile: rom::DeliveryProfile::AtLeastOnce,
                id: "unrelated".into(),
                cause: Cause {
                    retry_epoch: 1,
                    root: "new-root".into(),
                    parent: None,
                    depth: 0,
                    started_at: 0,
                    path: Vec::new(),
                },
                definition: "callback".into(),
                version: 1,
                service_key: "service".into(),
                payload: WorkPayload::Source(original.receipts[0].row.clone()),
            }],
        )
        .unwrap();
    let key = original.rows[0].key.clone();
    assert!(matches!(
        retain_snapshot(
            original,
            &RetentionPolicy::new(expired())
                .journal_through(2)
                .purge_tombstone(key),
            BackupLimits::default()
        ),
        Err(Error::Conflict)
    ));
}

#[test]
fn proof_selection_is_order_independent_and_future_receipts_are_invalid() {
    let mut original = snapshot(false, false);
    original.receipts.reverse();
    let (retained, _) = retain_snapshot(
        original,
        &RetentionPolicy::new(expired()).journal_through(2),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(retained.receipts[0].identity, "request-1");
    let mut invalid = snapshot(false, false);
    invalid.receipts[0].retry_epoch = 1;
    assert!(matches!(invalid.validate(), Err(Error::Storage)));
}
