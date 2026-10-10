//! Native Work layout and coherent transaction regression tests.
use crate::Redb;
use redb::{ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};

#[test]
fn fresh_native_work_has_separate_bounded_header_and_keyed_tables() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-incremental-layout-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    let tx = storage.db.begin_read().unwrap();
    let marker = tx.open_table(crate::format::META).unwrap();
    assert_eq!(marker.get("format").unwrap().unwrap().value(), 11);
    let state = tx.open_table(crate::format::STATE).unwrap();
    assert!(state.get("state").unwrap().is_none());
    assert_eq!(state.len().unwrap(), 3);
    let metadata: serde_json::Value =
        serde_json::from_str(state.get("metadata").unwrap().unwrap().value()).unwrap();
    assert!(metadata.get("work").is_none());
    assert!(metadata.get("operator").is_none());
    assert!(state.get("work_header").unwrap().is_some());
    assert!(state.get("operator").unwrap().is_some());
    for name in ["work_records", "work_roots"] {
        let table = tx
            .open_table(TableDefinition::<&str, &str>::new(name))
            .unwrap();
        assert_eq!(table.len().unwrap(), 0);
    }
    let active = tx
        .open_table(TableDefinition::<&str, u8>::new("work_active"))
        .unwrap();
    assert_eq!(active.len().unwrap(), 0);
    drop((active, state, marker));
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[derive(Clone, rom::Resource)]
#[resource(name = "native_items")]
struct Item {
    title: String,
}

fn pending(id: &str, row: &rom::Row) -> rom::PendingWork {
    rom::PendingWork {
        id: id.into(),
        cause: rom::Cause {
            retry_epoch: 0,
            root: "request".into(),
            parent: None,
            depth: 0,
            started_at: 0,
            path: vec![],
        },
        definition: "test".into(),
        version: 1,
        service_key: "service".into(),
        delivery_profile: rom::DeliveryProfile::AtLeastOnce,
        not_before: None,
        payload: rom::WorkPayload::Source(row.clone()),
    }
}

#[test]
fn retained_done_work_does_not_enter_native_candidate_scan() {
    use rom::storage_support::work::WorkRead;
    use rom::{Resource, Storage, WorkResult, WorkUpdate};
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-active-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    storage.register(&[Item::descriptor()]).unwrap();
    let row = rom::Row {
        key: rom::Key {
            kind: "native_items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(rom::json!({"title":"one"})),
        protected: Default::default(),
    };
    storage
        .commit(&rom::Bundle {
            expected: None,
            receipt: rom::Receipt {
                retry_epoch: 0,
                replay_version: None,
                identity: "request".into(),
                fingerprint: "fingerprint".into(),
                row: row.clone(),
            },
            changed: true,
            effects: vec![],
            reactions: (0..100)
                .map(|i| pending(&format!("work-{i:03}"), &row))
                .collect(),
            reaction_limits: Some(rom::ReactionLimits {
                max_fanout: 128,
                ..Default::default()
            }),
            completed_work: None,
        })
        .unwrap();
    for _ in 0..99 {
        let WorkResult::Claimed(claim) = storage
            .reaction_update(WorkUpdate::Claim { now: 0 })
            .unwrap()
        else {
            panic!("expected claim");
        };
        storage
            .reaction_update(WorkUpdate::Materialize {
                claim: claim.key(),
                now: 0,
                children: vec![],
            })
            .unwrap();
    }
    let tx = storage.db.begin_write().unwrap();
    let state = tx.open_table(crate::format::STATE).unwrap();
    let work = crate::native_work::NativeWork::new(&tx, &state).unwrap();
    assert_eq!(
        work.next_candidate(0, None).unwrap().unwrap().pending.id,
        "work-099"
    );
    assert_eq!(work.examined_candidates(), 1);
    drop(work);
    drop(state);
    drop(tx);
    assert_eq!(storage.reaction_records().unwrap().len(), 100);
    drop(storage);
    let reopened = Redb::open(directory.join("db")).unwrap();
    assert_eq!(
        reopened
            .reaction_records()
            .unwrap()
            .iter()
            .filter(|record| record.state == rom::WorkState::Done)
            .count(),
        99
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn format_nine_requires_explicit_source_preserving_upgrade() {
    use rom::Storage;
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-nine-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("source");
    let storage = Redb::open(&path).unwrap();
    let canonical = crate::maintenance::snapshot(
        &storage.db.begin_read().unwrap(),
        rom_backup::BackupLimits::default(),
    )
    .unwrap()
    .state;
    drop(storage);
    let db = redb::Database::open(&path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut state = tx.open_table(crate::format::STATE).unwrap();
        for key in ["metadata", "work_header", "operator"] {
            state.remove(key).unwrap();
        }
        state
            .insert("state", serde_json::to_string(&canonical).unwrap().as_str())
            .unwrap();
    }
    tx.delete_table(crate::format::WORK).unwrap();
    tx.delete_table(crate::format::ROOTS).unwrap();
    tx.delete_table(crate::format::ACTIVE).unwrap();
    tx.delete_table(crate::format::POSITIONS).unwrap();
    tx.open_table(crate::format::META)
        .unwrap()
        .insert("format", 9)
        .unwrap();
    tx.commit().unwrap();
    drop(db);
    let original = std::fs::read(&path).unwrap();
    assert!(matches!(Redb::open(&path), Err(rom::Error::Unsupported(_))));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let upgraded = Redb::upgrade_from(
        &path,
        directory.join("upgraded"),
        &[],
        rom_backup::BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(upgraded.retry_epochs().unwrap(), canonical.retry_epochs());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    drop(upgraded);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn current_native_header_corruption_is_rejected_on_reopen() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-header-corruption-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    drop(Redb::open(&path).unwrap());
    let db = redb::Database::open(&path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut state = tx.open_table(crate::format::STATE).unwrap();
        let mut header: serde_json::Value =
            serde_json::from_str(state.get("work_header").unwrap().unwrap().value()).unwrap();
        header["accounting"]["work"]["entries"] = rom::json!(1);
        state
            .insert(
                "work_header",
                serde_json::to_string(&header).unwrap().as_str(),
            )
            .unwrap();
    }
    tx.commit().unwrap();
    drop(db);
    assert!(matches!(Redb::open(&path), Err(rom::Error::Storage)));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_delta_cannot_cross_writer_contexts_even_without_changes() {
    use rom::storage_support::work::prepare_update;
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-read-fence-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    let mut state = tx.open_table(crate::format::STATE).unwrap();
    let first = crate::native_work::NativeWork::new(&tx, &state).unwrap();
    let delta = prepare_update(&first, rom::WorkUpdate::Claim { now: 0 }).unwrap();
    drop(first);
    let mut other = crate::native_work::NativeWork::new(&tx, &state).unwrap();
    assert_eq!(
        other.apply(delta, &mut state, &mut || Ok(())),
        Err(rom::Error::Conflict)
    );
    drop(other);
    drop(state);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn operator_snapshot_rejects_oversized_raw_work_before_decode() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-operator-budget-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::WORK)
        .unwrap()
        .insert("too-big", "x".repeat(1000).as_str())
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        storage.operator_snapshot(10, 512),
        Err(rom::Error::TooLarge)
    );
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "test-support")]
pub(super) fn single_work_storage() -> (Redb, std::path::PathBuf) {
    use rom::{Resource, Storage};
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-work-fault-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    storage.register(&[Item::descriptor()]).unwrap();
    let row = rom::Row {
        key: rom::Key {
            kind: "native_items".into(),
            id: "one".into(),
        },
        revision: 1,
        value: Some(rom::json!({"title":"one"})),
        protected: Default::default(),
    };
    storage
        .commit(&rom::Bundle {
            expected: None,
            receipt: rom::Receipt {
                retry_epoch: 0,
                replay_version: None,
                identity: "request".into(),
                fingerprint: "fingerprint".into(),
                row: row.clone(),
            },
            changed: true,
            effects: vec![],
            reactions: vec![pending("work", &row)],
            reaction_limits: Some(Default::default()),
            completed_work: None,
        })
        .unwrap();
    (storage, directory)
}

#[cfg(feature = "test-support")]
#[test]
fn native_work_update_exposes_every_write_and_commit_boundary() {
    use rom::Storage;
    use std::sync::{Arc, Mutex};
    let (storage, directory) = single_work_storage();
    let points = Arc::new(Mutex::new(vec![]));
    let observed = points.clone();
    storage.on_commit(Some(Arc::new(move |point| {
        observed.lock().unwrap().push(point);
        Ok(())
    })));
    assert!(matches!(
        storage.reaction_update(rom::WorkUpdate::Claim { now: 0 }),
        Ok(rom::WorkResult::Claimed(_))
    ));
    assert_eq!(*points.lock().unwrap(), vec![1, 2, 3, 4, 0, usize::MAX]);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "test-support")]
#[test]
fn each_work_native_write_fault_rolls_back_but_lost_ack_retains_claim() {
    use rom::Storage;
    use std::sync::Arc;
    for point in [1, 2, 3, 4, 0, usize::MAX] {
        let (storage, directory) = single_work_storage();
        let before = serde_json::to_value(
            crate::maintenance::snapshot(
                &storage.db.begin_read().unwrap(),
                rom_backup::BackupLimits::default(),
            )
            .unwrap()
            .state,
        )
        .unwrap();
        storage.on_commit(Some(Arc::new(move |actual| {
            if actual == point {
                Err(rom::Error::Storage)
            } else {
                Ok(())
            }
        })));
        let expected = if point == usize::MAX {
            rom::Error::Unknown
        } else {
            rom::Error::NotCommitted
        };
        assert_eq!(
            storage.reaction_update(rom::WorkUpdate::Claim { now: 0 }),
            Err(expected)
        );
        storage.on_commit(None);
        drop(storage);
        let reopened = Redb::open(directory.join("db")).unwrap();
        let after = serde_json::to_value(
            crate::maintenance::snapshot(
                &reopened.db.begin_read().unwrap(),
                rom_backup::BackupLimits::default(),
            )
            .unwrap()
            .state,
        )
        .unwrap();
        if point == usize::MAX {
            assert_ne!(after, before);
            assert!(matches!(
                reopened.reaction_records().unwrap()[0].state,
                rom::WorkState::Leased { .. }
            ));
        } else {
            assert_eq!(after, before);
        }
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn native_delta_rechecks_persisted_header_in_same_context() {
    use rom::storage_support::work::prepare_update;
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-header-fence-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open(directory.join("db")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    let mut state = tx.open_table(crate::format::STATE).unwrap();
    let mut work = crate::native_work::NativeWork::new(&tx, &state).unwrap();
    let delta = prepare_update(&work, rom::WorkUpdate::Claim { now: 0 }).unwrap();
    let mut header: serde_json::Value =
        serde_json::from_str(state.get("work_header").unwrap().unwrap().value()).unwrap();
    header["retry_epochs"]["current"] = rom::json!(1);
    state
        .insert(
            "work_header",
            serde_json::to_string(&header).unwrap().as_str(),
        )
        .unwrap();
    assert_eq!(
        work.apply(delta, &mut state, &mut || Ok(())),
        Err(rom::Error::Conflict)
    );
    drop(work);
    drop(state);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn operator_reconstruction_retains_configured_native_budget() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-configured-budget-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open_with_validation_limits(
        directory.join("db"),
        Default::default(),
        rom_backup::BackupLimits {
            max_bytes: 4096,
            ..Default::default()
        },
    )
    .unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::WORK)
        .unwrap()
        .insert("too-big", "x".repeat(8192).as_str())
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        storage.operator_snapshot(100, 65536),
        Err(rom::Error::TooLarge)
    );
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn bulk_work_read_rejects_oversized_raw_record_before_decode() {
    use rom::Storage;
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-bulk-budget-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open_with_validation_limits(
        directory.join("db"),
        Default::default(),
        rom_backup::BackupLimits {
            max_bytes: 4096,
            ..Default::default()
        },
    )
    .unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::WORK)
        .unwrap()
        .insert("too-big", "x".repeat(8192).as_str())
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(storage.reaction_records(), Err(rom::Error::TooLarge));
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}
