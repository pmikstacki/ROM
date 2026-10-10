use crate::Redb;
use redb::{ReadableDatabase, TableDefinition};
use rom::{Bundle, Error, Key, Receipt, Resource, Row, Storage, StorageLimits};
use rom_backup::BackupLimits;

#[derive(Clone, Resource)]
#[resource(name = "journal-items")]
struct Item {
    title: String,
}

fn directory(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rom-redb-journal-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
fn candidate(rows: usize) -> (Redb, std::path::PathBuf) {
    let directory = directory("candidate");
    let limits = StorageLimits {
        journal_rows: rows,
        ..StorageLimits::default()
    };
    let storage =
        Redb::open_with_validation_limits(directory.join("db"), limits, BackupLimits::default())
            .unwrap();
    storage.register(&[Item::descriptor()]).unwrap();
    (storage, directory)
}
fn predecessor_ten(path: &std::path::Path) -> Redb {
    Redb::open_owned_in_format(
        rom_backup::NativeOwnership::acquire(path, rom_backup::NativeAccess::OpenOrCreate).unwrap(),
        StorageLimits::default(),
        BackupLimits::default(),
        10,
    )
    .unwrap()
}
fn bundle(id: &str) -> Bundle {
    Bundle {
        expected: None,
        receipt: Receipt {
            retry_epoch: 0,
            replay_version: None,
            identity: format!("create-{id}"),
            fingerprint: id.into(),
            row: Row {
                key: Key {
                    kind: "journal-items".into(),
                    id: id.into(),
                },
                revision: 1,
                value: Some(rom::json!({"title":id})),
                protected: Default::default(),
            },
        },
        changed: true,
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
fn canonical(storage: &Redb) -> serde_json::Value {
    let tx = storage.db.begin_read().unwrap();
    let snapshot = crate::maintenance::snapshot_in_format(
        &tx,
        storage.validation_limits,
        crate::maintenance::NativeFormat::Exact(storage.native_format),
    )
    .unwrap();
    snapshot.validate().unwrap();
    serde_json::to_value(snapshot).unwrap()
}

#[test]
fn candidate_layout_has_scalar_header_and_position_index() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-journal-layout-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let storage = Redb::open_with_validation_limits(
        directory.join("db"),
        rom::StorageLimits::default(),
        rom_backup::BackupLimits::default(),
    )
    .unwrap();
    let tx = storage.db.begin_read().unwrap();
    let state = tx.open_table(crate::format::STATE).unwrap();
    let raw = state.get("metadata").unwrap().unwrap();
    let metadata: serde_json::Value = serde_json::from_str(raw.value()).unwrap();
    assert!(
        metadata.get("events").is_none(),
        "format 10 still embeds retained history"
    );
    assert!(
        tx.open_table(TableDefinition::<u64, &str>::new("journal_positions"))
            .is_ok()
    );
    drop((raw, state));
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn predecessor_ten_has_an_explicit_conversion_decoder() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-journal-predecessor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    drop(predecessor_ten(&path));
    let before = file_witness::capture(&path).unwrap();
    assert!(matches!(Redb::open(&path), Err(Error::Unsupported(_))));
    assert_eq!(file_witness::capture(&path).unwrap(), before);
    let snapshot = crate::maintenance::read_snapshot(
        &path,
        rom_backup::BackupLimits::default(),
        crate::maintenance::NativeFormat::PredecessorTen,
    );
    assert!(
        snapshot.is_ok(),
        "format 10 keyed Work needs its own predecessor decoder"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn candidate_commit_retains_the_canonical_prefix_and_reopens() {
    let (storage, path) = candidate(2);
    for id in ["one", "two", "three"] {
        storage.commit(&bundle(id)).unwrap();
    }
    let image = canonical(&storage);
    assert_eq!(image["state"]["floor"], 1);
    assert_eq!(image["state"]["head"], 3);
    assert_eq!(image["events"].as_array().unwrap().len(), 2);
    let cursor = rom::JournalCursor {
        generation: storage.journal_head("journal-items").unwrap().generation,
        kind: "journal-items".into(),
        position: 1,
    };
    let page = storage
        .journal("journal-items", Some(&cursor), 1, 4096)
        .unwrap();
    assert_eq!(page.events[0].identity, "create-two");
    assert_eq!(page.cursor.position, 2);
    let manifest = storage
        .backup_to(path.join("truthful.rombk"), BackupLimits::default())
        .unwrap();
    assert_eq!((manifest.archive_version, manifest.storage_format), (7, 11));
    drop(storage);
    let storage = Redb::open_with_validation_limits(
        path.join("db"),
        StorageLimits {
            journal_rows: 2,
            ..StorageLimits::default()
        },
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(canonical(&storage), image);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn scalar_operations_and_append_do_not_decode_retained_entries() {
    let (storage, path) = candidate(64);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::POSITIONS)
        .unwrap()
        .insert(1, "malformed untouched index")
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(storage.retry_epochs().unwrap().current, 0);
    assert_eq!(storage.journal_head("journal-items").unwrap().position, 1);
    storage.commit(&bundle("two")).unwrap();
    assert_eq!(storage.journal_head("journal-items").unwrap().position, 2);
    assert_eq!(
        storage.journal("journal-items", None, 10, 4096),
        Err(Error::Storage)
    );
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn borrowed_index_is_charged_before_malformed_decode() {
    let (mut storage, path) = candidate(4);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::POSITIONS)
        .unwrap()
        .insert(1, "{".repeat(4096).as_str())
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 1024,
        max_records: 64,
    };
    assert_eq!(
        storage.journal("journal-items", None, 10, 65536),
        Err(Error::TooLarge)
    );
    assert!(matches!(
        crate::maintenance::snapshot_in_format(
            &storage.db.begin_read().unwrap(),
            storage.validation_limits,
            crate::maintenance::NativeFormat::Exact(11)
        ),
        Err(Error::TooLarge)
    ));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn complete_native_admission_is_cumulative_before_any_decode() {
    let (storage, path) = candidate(4);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::STATE)
        .unwrap()
        .insert("metadata", "bad json")
        .unwrap();
    tx.open_table(crate::format::RECEIPTS)
        .unwrap()
        .insert("extra", "x".repeat(8192).as_str())
        .unwrap();
    tx.commit().unwrap();
    let limits = BackupLimits {
        max_bytes: 4096,
        max_records: 64,
    };
    assert!(matches!(
        crate::maintenance::snapshot_in_format(
            &storage.db.begin_read().unwrap(),
            limits,
            crate::maintenance::NativeFormat::Exact(11)
        ),
        Err(Error::TooLarge)
    ));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[cfg(feature = "test-support")]
#[test]
fn each_candidate_publication_checkpoint_rolls_back() {
    let (storage, path) = candidate(1);
    storage.commit(&bundle_with_work("one")).unwrap();
    let points = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = points.clone();
    storage.on_commit(Some(std::sync::Arc::new(move |n| {
        recorded.lock().unwrap().push(n);
        Ok(())
    })));
    storage.commit(&bundle_with_work("two")).unwrap();
    let ordinals = points
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|n| *n > 0 && *n < usize::MAX)
        .collect::<Vec<_>>();
    assert!(ordinals.len() >= 8);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
    for point in ordinals {
        let (storage, path) = candidate(1);
        storage.commit(&bundle_with_work("one")).unwrap();
        let before = canonical(&storage);
        storage.on_commit(Some(std::sync::Arc::new(move |n| {
            if n == point {
                Err(Error::Storage)
            } else {
                Ok(())
            }
        })));
        assert_eq!(
            storage.commit(&bundle_with_work("two")),
            Err(Error::NotCommitted)
        );
        assert_eq!(canonical(&storage), before);
        drop(storage);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[cfg(feature = "test-support")]
#[test]
fn candidate_unknown_ack_replays_without_another_event() {
    let (storage, path) = candidate(4);
    let request = bundle("one");
    storage.on_commit(Some(std::sync::Arc::new(|n| {
        if n == usize::MAX {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    })));
    assert_eq!(storage.commit(&request), Err(Error::Unknown));
    storage.on_commit(None);
    let before = canonical(&storage);
    assert_eq!(storage.commit(&request).unwrap(), request.receipt);
    assert_eq!(canonical(&storage), before);
    drop(storage);
    let storage = Redb::open_with_validation_limits(
        path.join("db"),
        StorageLimits {
            journal_rows: 4,
            ..StorageLimits::default()
        },
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(canonical(&storage), before);
    assert_eq!(storage.commit(&request).unwrap(), request.receipt);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn native_read_facts_reject_changed_retired_payload_and_header() {
    use rom::storage_support::metadata::{JournalRead, prepare_native_bundle};
    let (storage, path) = candidate(1);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    let mut reader = crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
    let delta = prepare_native_bundle(&reader, &reader, &bundle("two")).unwrap();
    let mut row = bundle("one").receipt.row;
    row.value = Some(rom::json!({"title":"eno"}));
    reader
        .events
        .insert("create-one", serde_json::to_string(&row).unwrap().as_str())
        .unwrap();
    assert_eq!(delta.validate(&reader, &reader), Err(Error::Conflict));
    reader
        .events
        .insert(
            "create-one",
            serde_json::to_string(&bundle("one").receipt.row)
                .unwrap()
                .as_str(),
        )
        .unwrap();
    let mut parts = JournalRead::header(&reader).unwrap().parts();
    parts.receipts += 1;
    reader
        .state
        .insert("metadata", serde_json::to_string(&parts).unwrap().as_str())
        .unwrap();
    assert_eq!(delta.validate(&reader, &reader), Err(Error::Conflict));
    drop(reader);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn work_only_publication_invalidates_an_earlier_joint_candidate() {
    use rom::storage_support::{metadata::prepare_native_bundle, work::prepare_update};
    let (storage, path) = candidate(4);
    let tx = storage.db.begin_write().unwrap();
    let mut reader = crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
    let candidate = prepare_native_bundle(&reader, &reader, &bundle("one")).unwrap();
    let work = prepare_update(&reader, rom::WorkUpdate::Claim { now: 0 }).unwrap();
    work.validate(&reader).unwrap();
    crate::native_work::PreparedWork::new(&work)
        .unwrap()
        .publish(
            &mut reader.state,
            &mut reader.records,
            &mut reader.roots,
            &mut reader.active,
            &mut || Ok(()),
        )
        .unwrap();
    reader.invalidate();
    assert_eq!(candidate.validate(&reader, &reader), Err(Error::Conflict));
    drop(reader);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn native_candidates_do_not_cross_transaction_contexts() {
    use rom::storage_support::metadata::prepare_native_bundle;
    let (storage, path) = candidate(4);
    let tx = storage.db.begin_write().unwrap();
    let reader = crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
    let delta = prepare_native_bundle(&reader, &reader, &bundle("one")).unwrap();
    drop(reader);
    let reader = crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
    assert_eq!(delta.validate(&reader, &reader), Err(Error::Conflict));
    drop(reader);
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn page_budget_charges_the_cumulative_borrowed_prefix() {
    let (mut storage, path) = candidate(4);
    storage.commit(&bundle("one")).unwrap();
    storage.commit(&bundle("two")).unwrap();
    let tx = storage.db.begin_read().unwrap();
    let state = tx.open_table(crate::format::STATE).unwrap();
    let index = tx.open_table(crate::format::POSITIONS).unwrap();
    let events = tx.open_table(crate::format::EVENTS).unwrap();
    let bytes = "metadata".len()
        + state.get("metadata").unwrap().unwrap().value().len()
        + 8
        + index.get(1).unwrap().unwrap().value().len()
        + "create-one".len()
        + events.get("create-one").unwrap().unwrap().value().len()
        + 8
        + index.get(2).unwrap().unwrap().value().len()
        - 1;
    drop((state, index, events));
    drop(tx);
    storage.validation_limits = BackupLimits {
        max_bytes: bytes,
        max_records: 64,
    };
    assert_eq!(
        storage.journal("journal-items", None, 10, 65536),
        Err(Error::TooLarge)
    );
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn candidate_conversion_preserves_source_and_applies_only_restore_fencing() {
    let path = directory("conversion");
    let source = path.join("source");
    let destination = path.join("destination");
    let storage = predecessor_ten(&source);
    storage.register(&[Item::descriptor()]).unwrap();
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_read().unwrap();
    let mut expected = crate::maintenance::snapshot_in_format(
        &tx,
        BackupLimits::default(),
        crate::maintenance::NativeFormat::PredecessorTen,
    )
    .unwrap();
    drop(tx);
    drop(storage);
    let before = std::fs::read(&source).unwrap();
    let original_generation = expected.state.journal_head("journal-items").generation;
    expected.state.prepare_restore().unwrap();
    let converted = Redb::upgrade_from(
        &source,
        &destination,
        &[Item::descriptor()],
        BackupLimits::default(),
    )
    .unwrap();
    assert_fenced_conversion(
        canonical(&converted),
        serde_json::to_value(expected).unwrap(),
        &original_generation,
    );
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert!(
        Redb::upgrade_from(
            &source,
            &destination,
            &[Item::descriptor()],
            BackupLimits::default()
        )
        .is_err()
    );
    drop(converted);
    let interrupted = path.join("interrupted");
    assert!(matches!(
        Redb::upgrade_from_observed(
            &source,
            &interrupted,
            &[Item::descriptor()],
            BackupLimits::default(),
            || Err(Error::Storage)
        ),
        Err(Error::Storage)
    ));
    assert!(!interrupted.exists());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
#[ignore = "Requires preserved genuine format-10 fixture path in ROM_REDB_FORMAT10_FIXTURE"]
fn genuine_captured_format_ten_converts_into_candidate() {
    let fixture = std::path::PathBuf::from(
        std::env::var_os("ROM_REDB_FORMAT10_FIXTURE").expect("genuine fixture path"),
    );
    let path = directory("genuine-conversion");
    let copy = path.join("source");
    std::fs::copy(fixture.join("source"), &copy).unwrap();
    let before = std::fs::read(&copy).unwrap();
    let limits = BackupLimits {
        max_bytes: 8 * 1024 * 1024,
        max_records: 4096,
    };
    assert!(matches!(
        Redb::open_with_validation_limits(&copy, StorageLimits::default(), limits),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    let mut expected = crate::maintenance::read_snapshot(
        &copy,
        limits,
        crate::maintenance::NativeFormat::PredecessorTen,
    )
    .unwrap();
    expected.validate().unwrap();
    let descriptors = expected.descriptors.clone();
    let original_generation = expected.state.journal_head("fixture").generation;
    expected.state.prepare_restore().unwrap();
    let storage =
        Redb::upgrade_from(&copy, path.join("destination"), &descriptors, limits).unwrap();
    assert_eq!(storage.native_format, 11);
    assert_eq!(storage.validation_limits.max_bytes, limits.max_bytes);
    assert_eq!(storage.validation_limits.max_records, limits.max_records);
    assert_fenced_conversion(
        canonical(&storage),
        serde_json::to_value(expected).unwrap(),
        &original_generation,
    );
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    // The actual predecessor archive is read directly; only fresh native destinations are written.
    let (manifest, mut archive_expected) = rom_backup::read(
        fixture.join("before.rombk"),
        rom_backup::Backend::Redb,
        limits,
    )
    .unwrap();
    assert_eq!((manifest.archive_version, manifest.storage_format), (7, 10));
    let archive_generation = archive_expected.state.journal_head("fixture").generation;
    archive_expected.state.prepare_restore().unwrap();
    let restored = Redb::restore_from(
        fixture.join("before.rombk"),
        path.join("archive-restored"),
        limits,
    )
    .unwrap();
    assert_eq!(restored.native_format, 11);
    assert_fenced_conversion(
        canonical(&restored),
        serde_json::to_value(archive_expected).unwrap(),
        &archive_generation,
    );
    drop(restored);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

fn assert_fenced_conversion(
    actual: serde_json::Value,
    mut expected: serde_json::Value,
    source_generation: &str,
) {
    assert_ne!(actual["state"]["generation"], source_generation);
    // Only the newly generated opaque token is nondeterministic. Work fencing and
    // every other canonical field must exactly match the existing transition.
    expected["state"]["generation"] = actual["state"]["generation"].clone();
    assert_eq!(actual, expected);
}

#[derive(Clone, Resource)]
#[resource(name = "journal-other")]
struct Other {
    title: String,
}

#[test]
fn mixed_kind_paging_preserves_progress_and_exact_byte_cutoffs() {
    let (storage, path) = candidate(8);
    storage
        .register(&[Item::descriptor(), Other::descriptor()])
        .unwrap();
    let first = bundle("one");
    storage.commit(&first).unwrap();
    let mut other = bundle("other");
    other.receipt.row.key.kind = "journal-other".into();
    storage.commit(&other).unwrap();
    storage.commit(&bundle("three")).unwrap();
    let page = storage.journal("journal-items", None, 1, 65536).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.cursor.position, 2);
    let bytes = serde_json::to_vec(&page.events[0]).unwrap().len();
    assert_eq!(
        storage.journal("journal-items", None, 10, bytes - 1),
        Err(Error::TooLarge)
    );
    let exact = storage.journal("journal-items", None, 10, bytes).unwrap();
    assert_eq!(exact.events, page.events);
    assert_eq!(exact.cursor.position, 2);
    let next = storage
        .journal("journal-items", Some(&exact.cursor), 10, 65536)
        .unwrap();
    assert_eq!(next.events[0].identity, "create-three");
    assert_eq!(next.cursor.position, 3);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn unsigned_maximum_position_is_exact_and_overflow_is_rejected() {
    use rom::storage_support::metadata::JournalRead;
    let (storage, path) = candidate(4);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    let mut reader = crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
    let mut entry = reader.entry(1).unwrap().unwrap();
    entry.event.position = u64::MAX;
    let bytes = serde_json::to_vec(&entry.event).unwrap().len();
    let mut parts = JournalRead::header(&reader).unwrap().parts();
    parts.head = u64::MAX;
    parts.floor = u64::MAX - 1;
    parts.journal_bytes = bytes;
    reader
        .state
        .insert("metadata", serde_json::to_string(&parts).unwrap().as_str())
        .unwrap();
    reader.positions.remove(1).unwrap();
    reader
        .positions
        .insert(
            u64::MAX,
            rom::json!({"identity":entry.event.identity,"canonical_bytes":bytes})
                .to_string()
                .as_str(),
        )
        .unwrap();
    drop(reader);
    tx.commit().unwrap();
    let head = storage.journal_head("journal-items").unwrap();
    assert_eq!(head.position, u64::MAX);
    let mut after = head.clone();
    after.position -= 1;
    let page = storage
        .journal("journal-items", Some(&after), 4, 65536)
        .unwrap();
    assert_eq!(page.events[0].position, u64::MAX);
    assert_eq!(page.cursor.position, u64::MAX);
    let before = canonical(&storage);
    assert_eq!(storage.commit(&bundle("two")), Err(Error::TooLarge));
    assert_eq!(canonical(&storage), before);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn configured_native_limits_are_independent_of_logical_collection_limits() {
    let (storage, path) = candidate(8);
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_read().unwrap();
    assert!(
        crate::maintenance::snapshot_with_limits(
            &tx,
            BackupLimits::default(),
            BackupLimits {
                max_bytes: 1,
                max_records: 4096
            },
            crate::maintenance::NativeFormat::Exact(11)
        )
        .is_err()
    );
    let custom = BackupLimits {
        max_bytes: 256 * 1024 * 1024,
        max_records: 800_000,
    };
    let snapshot = crate::maintenance::snapshot_with_limits(
        &tx,
        BackupLimits::default(),
        custom,
        crate::maintenance::NativeFormat::Exact(11),
    )
    .unwrap();
    snapshot.validate().unwrap();
    drop(tx);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

fn bundle_with_work(id: &str) -> Bundle {
    let mut b = bundle(id);
    b.reactions = vec![rom::PendingWork {
        id: format!("work-{id}"),
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
        payload: rom::WorkPayload::Source(b.receipt.row.clone()),
    }];
    b.reaction_limits = Some(rom::ReactionLimits::default());
    b
}

#[test]
fn candidate_work_selection_and_materialization_keep_the_existing_lifecycle() {
    let (storage, path) = candidate(4);
    storage.commit(&bundle_with_work("one")).unwrap();
    let rom::WorkResult::Claimed(claim) = storage
        .reaction_update(rom::WorkUpdate::Claim { now: 0 })
        .unwrap()
    else {
        panic!("claim expected");
    };
    storage
        .reaction_update(rom::WorkUpdate::Materialize {
            claim: claim.key(),
            now: 0,
            children: vec![],
        })
        .unwrap();
    assert_eq!(
        storage
            .reaction_update(rom::WorkUpdate::Claim { now: 0 })
            .unwrap(),
        rom::WorkResult::Idle
    );
    let records = storage.reaction_records().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, rom::WorkState::Done);
    let image = canonical(&storage);
    drop(storage);
    let storage = Redb::open_with_validation_limits(
        path.join("db"),
        StorageLimits {
            journal_rows: 4,
            ..StorageLimits::default()
        },
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(canonical(&storage), image);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn changed_false_commit_adds_a_receipt_without_a_position_or_payload() {
    let (storage, path) = candidate(4);
    let first = bundle("one");
    storage.commit(&first).unwrap();
    let mut second = first;
    second.expected = Some(1);
    second.changed = false;
    second.receipt.identity = "read-only".into();
    second.receipt.fingerprint = "read-only".into();
    storage.commit(&second).unwrap();
    let image = canonical(&storage);
    assert_eq!(image["state"]["receipts"], 2);
    assert_eq!(image["state"]["head"], 1);
    assert_eq!(image["events"].as_array().unwrap().len(), 1);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn incoming_event_identity_cannot_overwrite_an_orphan_payload() {
    let (storage, path) = candidate(4);
    let b = bundle("one");
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::EVENTS)
        .unwrap()
        .insert(
            b.receipt.identity.as_str(),
            serde_json::to_string(&b.receipt.row).unwrap().as_str(),
        )
        .unwrap();
    tx.commit().unwrap();
    assert_eq!(storage.commit(&b), Err(Error::Storage));
    assert_eq!(storage.counts().unwrap(), [0, 1, 0, 0]);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn touched_schema_is_physically_admitted_before_descriptor_decode() {
    let (mut storage, path) = candidate(4);
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::references::SCHEMAS)
        .unwrap()
        .insert("journal-items", "{".repeat(32768).as_str())
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 16384,
        max_records: 128,
    };
    assert_eq!(storage.commit(&bundle("one")), Err(Error::TooLarge));
    assert_eq!(storage.counts().unwrap(), [0, 0, 0, 0]);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[derive(Clone, Resource)]
#[resource(name = "journal-links")]
struct Link {
    target: rom::ResourceRef<Item>,
}

#[test]
fn touched_reference_target_is_physically_admitted_before_row_decode() {
    let (mut storage, path) = candidate(4);
    storage
        .register(&[Item::descriptor(), Link::descriptor()])
        .unwrap();
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::ROWS)
        .unwrap()
        .insert(("journal-items", "one"), "{".repeat(32768).as_str())
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 16384,
        max_records: 128,
    };
    let mut b = bundle("link");
    b.receipt.row.key.kind = "journal-links".into();
    b.receipt.row.value = Some(rom::json!({"target":"one"}));
    assert_eq!(storage.commit(&b), Err(Error::TooLarge));
    assert_eq!(storage.counts().unwrap(), [1, 1, 1, 0]);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn touched_reference_edge_is_charged_before_validation_or_copy() {
    let (mut storage, path) = candidate(4);
    storage
        .register(&[Item::descriptor(), Link::descriptor()])
        .unwrap();
    storage.commit(&bundle("one")).unwrap();
    let tx = storage.db.begin_write().unwrap();
    let enormous = "x".repeat(32768);
    tx.open_table(crate::references::OUTGOING)
        .unwrap()
        .insert(
            ("journal-links", "link", "journal-items", enormous.as_str()),
            0,
        )
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 16384,
        max_records: 128,
    };
    let mut b = bundle("link");
    b.receipt.row.key.kind = "journal-links".into();
    b.receipt.row.value = Some(rom::json!({"target":"one"}));
    assert_eq!(storage.commit(&b), Err(Error::TooLarge));
    assert_eq!(storage.counts().unwrap(), [1, 1, 1, 0]);
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn resource_snapshot_does_not_charge_payload_beyond_kind_boundary() {
    let (mut storage, path) = candidate(4);
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::ROWS)
        .unwrap()
        .insert(("zz-unrelated", "one"), "{".repeat(32768).as_str())
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 1024,
        max_records: 8,
    };
    assert_eq!(storage.snapshot("aa-empty", 10, 1024), Ok(vec![]));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn reference_ranges_do_not_charge_edges_beyond_the_resource_boundary() {
    for edge in [crate::references::OUTGOING, crate::references::INCOMING] {
        let (mut storage, path) = candidate(4);
        storage.commit(&bundle("one")).unwrap();
        let tx = storage.db.begin_write().unwrap();
        let enormous = "x".repeat(32768);
        tx.open_table(edge)
            .unwrap()
            .insert(
                ("zz-unrelated", enormous.as_str(), "journal-items", "one"),
                0,
            )
            .unwrap();
        tx.commit().unwrap();
        storage.validation_limits = BackupLimits {
            max_bytes: 16384,
            max_records: 128,
        };
        let mut deleted = bundle("one");
        deleted.expected = Some(1);
        deleted.receipt.identity = "delete-one".into();
        deleted.receipt.row.revision = 2;
        deleted.receipt.row.value = None;
        assert_eq!(storage.commit(&deleted), Ok(deleted.receipt));
        drop(storage);
        std::fs::remove_dir_all(path).unwrap();
    }
}

mod activation_tests;
mod file_witness;
mod lookup_tests;
mod maintenance_tests;

#[test]
fn complete_inventory_rejects_index_payload_and_scalar_disagreement() {
    use redb::ReadableTable;
    use rom::storage_support::metadata::JournalRead;
    for fault in [
        "bytes",
        "duplicate",
        "missing",
        "extra",
        "hole",
        "aggregate",
    ] {
        let (storage, path) = candidate(4);
        storage.commit(&bundle("one")).unwrap();
        storage.commit(&bundle("two")).unwrap();
        let tx = storage.db.begin_write().unwrap();
        let mut reader =
            crate::native_journal::Reader::writer(&tx, BackupLimits::default()).unwrap();
        match fault {
            "bytes" => {
                reader
                    .positions
                    .insert(1, "{\"identity\":\"create-one\",\"canonical_bytes\":1}")
                    .unwrap();
            }
            "duplicate" => {
                let index = reader.positions.get(1).unwrap().unwrap().value().to_owned();
                reader.positions.insert(2, index.as_str()).unwrap();
            }
            "missing" => {
                reader.events.remove("create-one").unwrap();
            }
            "extra" => {
                reader
                    .events
                    .insert(
                        "orphan",
                        serde_json::to_string(&bundle("one").receipt.row)
                            .unwrap()
                            .as_str(),
                    )
                    .unwrap();
            }
            "hole" => {
                let index = reader.positions.get(2).unwrap().unwrap().value().to_owned();
                reader.positions.remove(2).unwrap();
                reader.positions.insert(3, index.as_str()).unwrap();
            }
            "aggregate" => {
                let mut parts = JournalRead::header(&reader).unwrap().parts();
                parts.journal_bytes += 1;
                reader
                    .state
                    .insert("metadata", serde_json::to_string(&parts).unwrap().as_str())
                    .unwrap();
            }
            _ => unreachable!(),
        }
        drop(reader);
        tx.commit().unwrap();
        assert!(
            matches!(
                crate::maintenance::snapshot_in_format(
                    &storage.db.begin_read().unwrap(),
                    BackupLimits::default(),
                    crate::maintenance::NativeFormat::Exact(11)
                ),
                Err(Error::Storage)
            ),
            "{fault}"
        );
        drop(storage);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn candidate_operator_snapshot_keeps_separate_work_and_native_bounds() {
    let (mut storage, path) = candidate(4);
    storage.commit(&bundle_with_work("one")).unwrap();
    assert!(matches!(
        storage.operator_snapshot(1, 1),
        Err(Error::TooLarge)
    ));
    let tx = storage.db.begin_write().unwrap();
    tx.open_table(crate::format::POSITIONS)
        .unwrap()
        .insert(1, "{".repeat(32768).as_str())
        .unwrap();
    tx.commit().unwrap();
    storage.validation_limits = BackupLimits {
        max_bytes: 16384,
        max_records: 128,
    };
    assert!(matches!(
        storage.operator_snapshot(1000, 64 * 1024 * 1024),
        Err(Error::TooLarge)
    ));
    drop(storage);
    std::fs::remove_dir_all(path).unwrap();
}
