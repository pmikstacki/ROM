//! Explicit release trial over data produced by the independently verified 0.0.3 writer.
use rom::*;
use rom_backup::{Backend, BackupLimits, Snapshot};
use std::path::Path;

#[derive(Clone, Resource)]
#[resource(name = "upgrade-delayed-items")]
struct Item {
    name: String,
}

fn compare(before: &Snapshot, after: &Snapshot) {
    let mut expected: Snapshot =
        serde_json::from_value(serde_json::to_value(before).unwrap()).unwrap();
    expected.state.prepare_restore().unwrap();
    let mut expected = serde_json::to_value(expected).unwrap();
    let actual = serde_json::to_value(after).unwrap();
    let before_value = serde_json::to_value(before).unwrap();
    assert_ne!(
        actual["state"]["generation"],
        before_value["state"]["generation"]
    );
    // Restore generates a fresh token; all deterministic canonical fields must match.
    expected["state"]["generation"] = actual["state"]["generation"].clone();
    assert_eq!(
        actual, expected,
        "native upgrade must match the complete canonical restore oracle"
    );
    assert_eq!(after.rows, before.rows);
    assert_eq!(after.receipts, before.receipts);
    assert_eq!(after.events, before.events);
    assert_eq!(
        serde_json::to_value(&after.effects).unwrap(),
        serde_json::to_value(&before.effects).unwrap()
    );
    assert_eq!(after.descriptors, before.descriptors);
    assert_eq!(after.references, before.references);
    let old = before.state.work.records();
    let new = after.state.work.records();
    assert_eq!(old.len(), new.len());
    for (old, new) in old.iter().zip(&new) {
        assert_eq!(old.pending, new.pending);
        assert_eq!(old.attempts, new.attempts);
        if matches!(old.state, WorkState::Leased { .. }) {
            let mut expected = old.clone();
            expected.generation = expected.generation.checked_add(1).unwrap();
            expected.revision = expected.revision.checked_add(1).unwrap();
            expected.state = WorkState::Pending;
            // A predecessor has no extra delay, but its original chain age still applies.
            expected.due = old.pending.cause.started_at;
            assert_eq!(new, &expected);
        } else {
            assert_eq!(new, old);
        }
    }
    let before = serde_json::to_value(&before.state).unwrap();
    let after = serde_json::to_value(&after.state).unwrap();
    assert_eq!(after["operator"], before["operator"]);
    assert_eq!(after["work"]["roots"], before["work"]["roots"]);
}

fn inspect(store: &dyn Storage, summary: &Value) {
    let claim: WorkClaim = serde_json::from_value(summary["claim"].clone()).unwrap();
    let cursor: JournalCursor = serde_json::from_value(summary["cursor"].clone()).unwrap();
    assert!(matches!(
        store.journal(Item::KIND, Some(&cursor), 16, 65536),
        Err(Error::HistoryGap)
    ));
    assert_eq!(
        store.reaction_update(WorkUpdate::DeliveryFinished {
            claim: claim.key(),
            now: 12,
            outcome: DeliveryOutcome::Accepted
        }),
        Err(Error::Conflict)
    );
    assert_eq!(
        store
            .load(&Key {
                kind: Item::KIND.into(),
                id: "live".into()
            })
            .unwrap()
            .unwrap()
            .value,
        Some(json!({"name":"original"}))
    );
    assert!(
        store
            .load(&Key {
                kind: Item::KIND.into(),
                id: "deleted".into()
            })
            .unwrap()
            .unwrap()
            .value
            .is_none()
    );
    assert_eq!(
        store.receipt("create-live").unwrap().unwrap().fingerprint,
        "original-request"
    );
    assert_eq!(
        store.receipt("tombstone").unwrap().unwrap().fingerprint,
        "delete-request"
    );
}

#[test]
#[ignore = "release trial requires separately witnessed accepted 0.0.3 writer output; run explicitly with ROM_PREDECESSOR_EVIDENCE"]
fn accepted_release_writer_native_and_archive_upgrade_preserve_data_and_fence_claims() {
    let root = std::env::var_os("ROM_PREDECESSOR_EVIDENCE")
        .expect("explicit old-writer evidence directory");
    let root = Path::new(&root);
    for (name, backend) in [("sqlite", Backend::Sqlite), ("redb", Backend::Redb)] {
        let directory = root.join(name);
        let source = directory.join("source");
        let archive = directory.join("before.rombk");
        let source_bytes = std::fs::read(&source).unwrap();
        let archive_bytes = std::fs::read(&archive).unwrap();
        let summary: Value =
            serde_json::from_slice(&std::fs::read(directory.join("summary.json")).unwrap())
                .unwrap();
        assert_eq!(summary["writer_release"], "0.0.3");
        let before: Snapshot = serde_json::from_value(summary["snapshot"].clone()).unwrap();
        let destination = directory.join("upgraded");
        let after_archive = directory.join("after.rombk");
        match backend {
            Backend::Sqlite => {
                let store = rom_sqlite::Sqlite::upgrade_from(
                    &source,
                    &destination,
                    &[Item::descriptor()],
                    BackupLimits::default(),
                )
                .unwrap();
                inspect(&store, &summary);
                store
                    .backup_to(&after_archive, BackupLimits::default())
                    .unwrap();
            }
            Backend::Redb => {
                let store = rom_redb::Redb::upgrade_from(
                    &source,
                    &destination,
                    &[Item::descriptor()],
                    BackupLimits::default(),
                )
                .unwrap();
                inspect(&store, &summary);
                store
                    .backup_to(&after_archive, BackupLimits::default())
                    .unwrap();
            }
        }
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(&archive).unwrap(), archive_bytes);
        let (manifest, after) =
            rom_backup::read(&after_archive, backend, BackupLimits::default()).unwrap();
        assert_eq!((manifest.archive_version, manifest.storage_format), (7, 11));
        assert_eq!(manifest.operator_receipts, 1);
        compare(&before, &after);
        let converted_archive = directory.join("converted.rombk");
        let manifest = rom_backup::upgrade_v6_archive(
            &archive,
            &converted_archive,
            backend,
            BackupLimits::default(),
        )
        .unwrap();
        assert_eq!((manifest.archive_version, manifest.storage_format), (7, 11));
        assert_eq!(std::fs::read(&archive).unwrap(), archive_bytes);
        let (_, converted) =
            rom_backup::read(&converted_archive, backend, BackupLimits::default()).unwrap();
        assert_eq!(converted.rows, before.rows);
        assert_eq!(converted.receipts, before.receipts);
        assert_eq!(
            serde_json::to_value(&converted).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        println!(
            "accepted-writer-upgrade {name} native8→{} archive6→{} verified",
            manifest.storage_format, manifest.archive_version
        );
    }
}
