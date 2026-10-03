//! Public SQLite maintenance boundaries; raw SQL constructs compatibility/corruption fixtures.
use rom::*;
use rom_backup::{Backend, BackupLimits};
use rom_sqlite::Sqlite;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "index-recovery-items")]
struct Item {
    amount: u64,
    label: String,
}
#[derive(Clone, Resource)]
#[resource(name = "index-recovery-omitted")]
struct Omitted {
    enabled: bool,
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-index-recovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn bundle<R: Resource>(id: &str, revision: u64, value: R) -> Bundle {
    Bundle {
        expected: (revision > 1).then_some(revision - 1),
        changed: true,
        receipt: Receipt {
            identity: format!("{}-{id}-{revision}", R::KIND),
            fingerprint: format!("{revision}"),
            retry_epoch: 0,
            replay_version: None,
            row: Row {
                key: Key {
                    kind: R::KIND.into(),
                    id: id.into(),
                },
                revision,
                value: Some(value.encode()),
                protected: Default::default(),
            },
        },
        effects: vec![],
        reactions: vec![],
        reaction_limits: None,
        completed_work: None,
    }
}
fn seeded(s: &Scratch, amount: usize) -> Sqlite {
    let db = Sqlite::open(s.path("source.db")).unwrap();
    db.register(&[Item::descriptor()]).unwrap();
    for i in 0..amount {
        db.commit(&bundle(
            &i.to_string(),
            1,
            Item {
                amount: i as u64,
                label: format!("item {i}"),
            },
        ))
        .unwrap();
    }
    db
}
fn request<R: Resource>() -> StorageQuery {
    StorageQuery {
        descriptor: R::descriptor(),
        spec: QuerySpec::all(),
        semantics: QUERY_SEMANTICS_VERSION,
        selection: SelectionMode::UniformReadAndFields,
    }
}
fn rows(db: &Sqlite) -> Vec<Row> {
    db.snapshot(Item::KIND, 1000, 1_000_000).unwrap()
}
fn bytes(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| serde_json::to_vec(row).unwrap().len())
        .sum()
}
fn bound(bytes: usize) -> QueryBounds {
    QueryBounds {
        max_rows: 1000,
        max_bytes: bytes,
    }
}
fn files(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut names = vec![path.to_path_buf()];
    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    names.push(wal.into());
    names
        .into_iter()
        .filter_map(|path| match std::fs::read(&path) {
            Ok(bytes) => Some((path, bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("cannot capture {}: {error}", path.display()),
        })
        .collect()
}
fn unchanged(path: &Path, before: &[(PathBuf, Vec<u8>)]) {
    assert!(
        files(path) == before,
        "maintenance changed source main/WAL data"
    );
}
fn pad_row(path: &Path, id: &str, padding: usize) {
    let mut raw = rusqlite::Connection::open(path).unwrap();
    let tx = raw.transaction().unwrap();
    let data: String = tx
        .query_row(
            "SELECT data FROM resources WHERE kind=? AND id=?",
            rusqlite::params![Item::KIND, id],
            |r| r.get(0),
        )
        .unwrap();
    tx.execute(
        "UPDATE resources SET data=? WHERE kind=? AND id=?",
        rusqlite::params![format!("{}{data}", " ".repeat(padding)), Item::KIND, id],
    )
    .unwrap();
    tx.execute(
        "UPDATE query_kinds SET row_bytes=row_bytes+? WHERE kind=?",
        rusqlite::params![padding as i64, Item::KIND],
    )
    .unwrap();
    tx.commit().unwrap();
}
#[test]
fn noncanonical_text_is_charged_then_decremented_and_rebuild_uses_destination_bytes() {
    let s = Scratch::new();
    let db = seeded(&s, 3);
    let canonical = bytes(&rows(&db));
    drop(db);
    pad_row(&s.path("source.db"), "0", 37);
    let db = Sqlite::open(s.path("source.db")).unwrap();
    assert_eq!(
        db.query_read(&request::<Item>(), bound(canonical + 36)),
        Err(Error::TooLarge)
    );
    assert!(
        db.query_read(&request::<Item>(), bound(canonical + 37))
            .is_ok()
    );
    db.commit(&bundle(
        "0",
        2,
        Item {
            amount: 77,
            label: "replacement".into(),
        },
    ))
    .unwrap();
    let expected = rows(&db);
    let canonical = bytes(&expected);
    assert!(
        db.query_read(&request::<Item>(), bound(canonical)).is_ok(),
        "replacement must subtract the old stored length, including whitespace"
    );
    drop(db);
    pad_row(&s.path("source.db"), "1", 53);
    let original = files(&s.path("source.db"));
    let rebuilt = Sqlite::rebuild_indexes_from(
        s.path("source.db"),
        s.path("rebuilt.db"),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(rows(&rebuilt), expected);
    assert!(
        rebuilt
            .query_read(&request::<Item>(), bound(canonical))
            .is_ok(),
        "rebuilt counters must use the new canonical native rows"
    );
    unchanged(&s.path("source.db"), &original);
    drop(rebuilt);
    assert!(Sqlite::open(s.path("rebuilt.db")).is_ok());
}
#[test]
fn omitted_catalog_kinds_survive_registration_and_index_rebuild() {
    let s = Scratch::new();
    let db = seeded(&s, 1);
    db.register(&[Omitted::descriptor()]).unwrap();
    let omitted = db
        .commit(&bundle("other", 1, Omitted { enabled: true }))
        .unwrap()
        .row;
    db.register(&[Item::descriptor()]).unwrap();
    drop(db);
    let rebuilt = Sqlite::rebuild_indexes_from(
        s.path("source.db"),
        s.path("rebuilt.db"),
        BackupLimits::default(),
    )
    .unwrap();
    rebuilt.register(&[Item::descriptor()]).unwrap();
    assert_eq!(rebuilt.load(&omitted.key).unwrap(), Some(omitted.clone()));
    let response = rebuilt
        .query_read(&request::<Omitted>(), bound(1_000_000))
        .unwrap();
    let selected = match response {
        QueryRead::Reference { rows } | QueryRead::NativeCandidates { rows, .. } => rows,
    };
    assert_eq!(selected, vec![omitted]);
    rebuilt
        .backup_to(s.path("archive"), BackupLimits::default())
        .unwrap();
    let (_, snapshot) =
        rom_backup::read(s.path("archive"), Backend::Sqlite, BackupLimits::default()).unwrap();
    assert!(snapshot.descriptors.iter().any(|d| d.kind == Omitted::KIND));
}
#[test]
fn rebuilding_derived_data_cannot_repair_missing_authoritative_receipt_proof() {
    let s = Scratch::new();
    drop(seeded(&s, 1));
    let raw = rusqlite::Connection::open(s.path("source.db")).unwrap();
    raw.execute_batch("DELETE FROM query_keys; DELETE FROM receipts;")
        .unwrap();
    drop(raw);
    let original = files(&s.path("source.db"));
    assert!(matches!(
        Sqlite::rebuild_indexes_from(
            s.path("source.db"),
            s.path("rejected.db"),
            BackupLimits::default()
        ),
        Err(Error::Storage)
    ));
    assert!(!s.path("rejected.db").exists());
    unchanged(&s.path("source.db"), &original);
}
#[test]
fn logical_archive_fits_but_combined_native_index_budget_blocks_publication() {
    let s = Scratch::new();
    let db = seeded(&s, 1);
    db.backup_to(s.path("archive"), BackupLimits::default())
        .unwrap();
    drop(db);
    // One descriptor, row, receipt and event fit. The derived profile, kind and
    // scalar entries must share the same budget during native reconstruction.
    let limits = BackupLimits {
        max_records: 4,
        ..BackupLimits::default()
    };
    assert!(rom_backup::read(s.path("archive"), Backend::Sqlite, limits).is_ok());
    let original = files(&s.path("source.db"));
    assert!(matches!(
        Sqlite::rebuild_indexes_from(s.path("source.db"), s.path("limited.db"), limits),
        Err(Error::TooLarge)
    ));
    assert!(!s.path("limited.db").exists());
    unchanged(&s.path("source.db"), &original);
    assert!(matches!(
        Sqlite::restore_from(s.path("archive"), s.path("restore-limited.db"), limits),
        Err(Error::TooLarge)
    ));
    assert!(!s.path("restore-limited.db").exists());
}
#[test]
#[ignore = "invoked by the process-exit parent"]
fn child_exits_after_index_validation_before_publication() {
    let source = std::env::var_os("ROM_INDEX_REBUILD_SOURCE").unwrap();
    let destination = std::env::var_os("ROM_INDEX_REBUILD_DESTINATION").unwrap();
    let _ = Sqlite::rebuild_indexes_from_observed(
        PathBuf::from(source),
        PathBuf::from(destination),
        BackupLimits::default(),
        || std::process::exit(86),
    );
    panic!("publication checkpoint was not reached");
}
#[test]
fn process_exit_leaves_source_unchanged_and_no_published_index_rebuild() {
    let s = Scratch::new();
    let db = seeded(&s, 3);
    let expected = rows(&db);
    drop(db);
    let original = files(&s.path("source.db"));
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_exits_after_index_validation_before_publication",
            "--ignored",
        ])
        .env("ROM_INDEX_REBUILD_SOURCE", s.path("source.db"))
        .env("ROM_INDEX_REBUILD_DESTINATION", s.path("interrupted.db"))
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(86),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(!s.path("interrupted.db").exists());
    unchanged(&s.path("source.db"), &original);
    let retried = Sqlite::rebuild_indexes_from(
        s.path("source.db"),
        s.path("interrupted.db"),
        BackupLimits::default(),
    )
    .unwrap();
    assert_eq!(rows(&retried), expected);
    unchanged(&s.path("source.db"), &original);
}
