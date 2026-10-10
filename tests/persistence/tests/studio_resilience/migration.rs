use super::support::*;
use rom::operator::*;
use rom::*;
use rom_backup::{MigrationPlan, ResourceMigration};
use std::sync::atomic::{AtomicUsize, Ordering};

static CONVERSIONS: AtomicUsize = AtomicUsize::new(0);
fn counted_conversion(record: Record) -> Result<RecordV2> {
    CONVERSIONS.fetch_add(1, Ordering::SeqCst);
    conversion(record)
}

#[tokio::test]
async fn native_maintenance_requires_closed_owner_then_fresh_identity_and_current_replay() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let destination = scratch.path("migrated");
        let db = Database::open(redb, &source);
        let runtime = base()
            .resource(record_definition())
            .build(db.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        let provider_activation = provision(&runtime).await;
        let actor = human(&runtime, &provider_activation).await;
        let original: Invocation = Command::create(
            "one",
            Record {
                owner: "reader".into(),
                value: 7,
            },
        )
        .idempotency("record")
        .into();
        runtime.invoke(&actor, original.clone()).await.unwrap();
        runtime
            .execute(
                &actor,
                Command::create(
                    "ledger",
                    Ledger {
                        owner: "reader".into(),
                        total: 3,
                    },
                )
                .idempotency("ledger"),
            )
            .await
            .unwrap();
        let plan = MigrationPlan::new(vec![
            ResourceMigration::new::<Record, RecordV2>(counted_conversion).unwrap(),
        ])
        .unwrap();
        CONVERSIONS.store(0, Ordering::SeqCst);
        let before = db.counts();
        assert!(Database::migrate(redb, &source, &destination, &plan).is_err());
        assert!(!destination.exists());
        assert_eq!(CONVERSIONS.load(Ordering::SeqCst), 0);
        assert_eq!(db.counts(), before);
        shutdown(&runtime).await;
        drop(runtime);
        drop(db);
        let original_bytes = std::fs::read(&source).unwrap();
        let migrated = Database::migrate(redb, &source, &destination, &plan).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original_bytes);
        assert!(CONVERSIONS.load(Ordering::SeqCst) > 0);
        let archive = migrated.backup(&scratch.path("archive"));
        assert!(
            archive
                .descriptors
                .iter()
                .any(|descriptor| descriptor.kind == rom_identity::IdentityProvider::KIND)
        );
        let runtime = after()
            .build(migrated.storage(), Runtime::shared_cpu_pool(2).unwrap())
            .unwrap();
        // Re-establish identity from current managed configuration; never carry a serialized Actor across maintenance.
        let current = activation(&runtime).await;
        let fresh = human(&runtime, &current).await;
        let counts = migrated.counts();
        let replay = runtime.invoke(&fresh, original.clone()).await.unwrap();
        assert_eq!(replay.revision, 1);
        assert_eq!(replay.value, Some(json!({"owner":"reader","total":7})));
        assert_eq!(migrated.counts(), counts);
        assert_eq!(
            runtime
                .read::<Ledger>(&fresh, "ledger")
                .await
                .unwrap()
                .value
                .unwrap()
                .total,
            3
        );
        enabled(&runtime, &identity_targets()[2], 1, false).await;
        assert!(matches!(
            runtime.invoke(&fresh, original).await,
            Err(Error::Denied)
        ));
        shutdown(&runtime).await;
    }
}

fn seed(storage: &dyn Storage) {
    storage
        .register(&[Record::descriptor(), Ledger::descriptor()])
        .unwrap();
    let row = Row {
        key: Key {
            kind: Record::KIND.into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(
            Record {
                owner: "reader".into(),
                value: 7,
            }
            .encode(),
        ),
        protected: Default::default(),
    };
    storage
        .commit(&Bundle {
            expected: None,
            receipt: Receipt {
                identity: "seed".into(),
                fingerprint: "seed".into(),
                retry_epoch: 0,
                replay_version: None,
                row: row.clone(),
            },
            changed: true,
            effects: vec![],
            reactions: vec![PendingWork {
                id: "stable-notice-id".into(),
                definition: "notice".into(),
                version: 1,
                service_key: "stable-service".into(),
                not_before: None,
                delivery_profile: DeliveryProfile::ReconcileBeforeRetry,
                cause: Cause {
                    retry_epoch: 0,
                    root: "original-root".into(),
                    parent: None,
                    depth: 1,
                    started_at: NOW,
                    path: vec!["stable-notice-id".into()],
                },
                payload: WorkPayload::Notification {
                    source: row,
                    payload: json!("unchanged delivery"),
                },
            }],
            reaction_limits: Some(ReactionLimits::default()),
            completed_work: None,
        })
        .unwrap();
}
fn hold(storage: &dyn Storage, now: u64) -> ClaimKey {
    let WorkResult::Claimed(claim) = storage.reaction_update(WorkUpdate::Claim { now }).unwrap()
    else {
        panic!("claim expected");
    };
    let key = claim.key();
    storage
        .reaction_update(WorkUpdate::DeliveryStarted {
            claim: key.clone(),
            now,
        })
        .unwrap();
    storage
        .reaction_update(WorkUpdate::DeliveryFinished {
            claim: key.clone(),
            now,
            outcome: DeliveryOutcome::Unknown,
        })
        .unwrap();
    key
}

#[test]
fn seeded_strict_hold_and_operator_receipt_survive_native_migration_and_replay() {
    for redb in [false, true] {
        let scratch = Scratch::new();
        let source = scratch.path("source");
        let target = scratch.path("target");
        let db = Database::open(redb, &source);
        let storage = db.storage();
        seed(storage.as_ref());
        hold(storage.as_ref(), NOW);
        let view = storage.work_snapshot(64, 65536).unwrap();
        let control = StorageWorkControl {
            principal: "trusted-operator".into(),
            request: WorkControlRequest {
                handle: WorkHandle::from_work_id("stable-notice-id"),
                expected: view.version(&view.records[0]),
                key: "original-reconcile-key".into(),
                retry_epoch: 0,
                operation: WorkControlOperation::Reconcile {
                    evidence_ref: Some("original-reference".into()),
                },
            },
            decision: WorkControlDecision::DeliveryNotAccepted {
                evidence: "durable-provider-evidence".into(),
            },
            now: NOW + 1,
        };
        let accepted = storage.control_work(&control).unwrap();
        assert_eq!(accepted.result.outcome, WorkControlOutcome::Scheduled);
        let claim = hold(storage.as_ref(), NOW + 2);
        let before = storage.work_snapshot(64, 65536).unwrap();
        assert_eq!(before.records[0].state, WorkState::AwaitingReconciliation);
        let cause = before.records[0].pending.cause.clone();
        let counts = db.counts();
        drop(storage);
        drop(db);
        let source_bytes = std::fs::read(&source).unwrap();
        let migrated = Database::migrate(redb, &source, &target, &plan()).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(migrated.counts(), counts);
        let storage = migrated.storage();
        let after = storage.work_snapshot(64, 65536).unwrap();
        assert_ne!(after.generation, before.generation);
        assert_eq!(after.roots, before.roots);
        assert_eq!(after.limits, before.limits);
        let old = &before.records[0];
        let record = &after.records[0];
        assert_eq!(record.pending.id, old.pending.id);
        assert_eq!(record.pending.service_key, old.pending.service_key);
        assert_eq!(record.pending.cause, cause);
        assert_eq!(record.state, WorkState::AwaitingReconciliation);
        assert_eq!(record.attempts, old.attempts);
        assert_eq!(
            record.pending.delivery_profile,
            DeliveryProfile::ReconcileBeforeRetry
        );
        assert!(record.revision > old.revision);
        // The hold has no active lease; source conversion changes revision,
        // while restore fences storage versions and active claims independently.
        assert_eq!(record.generation, old.generation);
        let WorkPayload::Notification { source, payload } = &record.pending.payload else {
            panic!("notification expected");
        };
        assert_eq!(
            RecordV2::decode(source.value.clone().unwrap())
                .unwrap()
                .total,
            7
        );
        assert_eq!(payload, &json!("unchanged delivery"));
        assert_eq!(after.operator, before.operator);
        let mut expected = accepted.clone();
        expected.result.replayed = true;
        assert_eq!(storage.control_work(&control).unwrap(), expected);
        assert_eq!(storage.work_snapshot(64, 65536).unwrap(), after);
        assert_eq!(
            storage.reaction_update(WorkUpdate::DeliveryFinished {
                claim,
                now: NOW + 3,
                outcome: DeliveryOutcome::Accepted
            }),
            Err(Error::Conflict)
        );
        let mut changed = control.clone();
        changed.request.operation = WorkControlOperation::Retry;
        assert_eq!(storage.control_work(&changed), Err(Error::IdentityMismatch));
        let mut stale = control.clone();
        stale.request.key = "new-key-old-version".into();
        assert_eq!(storage.control_work(&stale), Err(Error::Conflict));
        let mut bypass = control.clone();
        bypass.request.key = "retry-cannot-bypass-hold".into();
        bypass.request.expected = after.version(record);
        bypass.request.operation = WorkControlOperation::Retry;
        bypass.decision = WorkControlDecision::Retry;
        assert_eq!(storage.control_work(&bypass), Err(Error::Conflict));
        assert_eq!(
            storage
                .reaction_update(WorkUpdate::Claim { now: NOW + 3 })
                .unwrap(),
            WorkResult::Idle
        );
        let archived = migrated.backup(&scratch.path("migrated-archive"));
        assert_eq!(archived.state.operator, before.operator);
        drop(storage);
        drop(migrated);
        let reopened = Database::open(redb, &target);
        assert_eq!(reopened.storage().work_snapshot(64, 65536).unwrap(), after);
        assert_eq!(reopened.storage().control_work(&control).unwrap(), expected);
    }
}
