//! Offline operations after abrupt exit and callback changes to process state.
use super::{
    MemoryRecord,
    support::{BACKENDS, Backend, Process, Scratch, assert_saved, seed},
};
use rom::{Error, Resource, Storage};
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::{fs, path::PathBuf};

#[cfg(target_os = "linux")]
#[test]
fn sqlite_offline_rebuild_after_sigkill_preserves_committed_source_wal() {
    use std::os::{unix::fs::MetadataExt, unix::process::ExitStatusExt};
    let scratch = Scratch::new();
    // Reuse the established process fixture; it parks with the native engine open.
    let mut child = Process::spawn("process::native_owner_child", &scratch.0, Backend::Sqlite);
    child.wait_ready(&scratch.0.join("ready"));
    let source = scratch.path();
    let sidecar = scratch.0.join("database.rom-owner");
    let owner_inode = fs::metadata(&sidecar).unwrap().ino();
    assert!(matches!(
        rom_sqlite::Sqlite::open(&source),
        Err(Error::Conflict)
    ));
    assert_eq!(child.kill().signal(), Some(9));
    assert_eq!(fs::metadata(&sidecar).unwrap().ino(), owner_inode);

    let wal = scratch.0.join("database-wal");
    let source_bytes = fs::read(&source).unwrap();
    let wal_bytes = fs::read(&wal).unwrap();
    assert!(
        !wal_bytes.is_empty(),
        "fixture must retain committed WAL frames"
    );

    // A copy of the main file alone must not contain the committed record.
    let main_only = scratch.0.join("main-only");
    fs::write(&main_only, &source_bytes).unwrap();
    let isolated = rusqlite::Connection::open_with_flags(
        &main_only,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let has_rows_table: bool = isolated
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='resources')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    if has_rows_table {
        let count: i64 = isolated
            .query_row("SELECT COUNT(*) FROM resources", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "the committed record must be WAL-only");
    }
    drop(isolated);

    let destination = scratch.0.join("rebuilt");
    let replacement =
        rom_sqlite::Sqlite::rebuild_indexes_from(&source, &destination, BackupLimits::default())
            .unwrap();
    assert_saved(&replacement, "committed before process exit", 2);
    assert_eq!(
        replacement
            .journal(MemoryRecord::KIND, None, 10, 100_000)
            .unwrap()
            .events
            .len(),
        2
    );
    assert_eq!(
        fs::read(&source).unwrap(),
        source_bytes,
        "offline rebuild changed source main file"
    );
    assert_eq!(
        fs::read(&wal).unwrap(),
        wal_bytes,
        "offline rebuild changed source WAL"
    );
    assert_eq!(fs::metadata(&sidecar).unwrap().ino(), owner_inode);
    drop(replacement);
    let reopened = rom_sqlite::Sqlite::open(&destination).unwrap();
    assert_saved(&reopened, "committed before process exit", 2);
    assert_eq!(fs::read(&source).unwrap(), source_bytes);
    assert_eq!(fs::read(&wal).unwrap(), wal_bytes);
}

#[derive(Clone, Resource)]
#[resource(name = "ownership-memory-records", version = 2)]
struct UpdatedRecord {
    value: String,
}

#[test]
fn maintenance_cwd_child() {
    let Some(root) = std::env::var_os("ROM_OWNERSHIP_TEST_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    for backend in BACKENDS {
        let directory = root.join(backend.name());
        let elsewhere = directory.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let source = directory.join("database");
        let destination = directory.join("replacement");
        let storage = backend.open(&source).unwrap();
        seed(&*storage, "canonical source");
        drop(storage);
        let source_bytes = fs::read(&source).unwrap();
        fs::write(elsewhere.join("database"), b"unrelated source").unwrap();
        fs::write(elsewhere.join("replacement"), b"unrelated destination").unwrap();
        let plan = MigrationPlan::new(vec![
            ResourceMigration::new::<MemoryRecord, UpdatedRecord>(|value| {
                Ok(UpdatedRecord { value: value.value })
            })
            .unwrap(),
        ])
        .unwrap();
        std::env::set_current_dir(&directory).unwrap();
        let observe = || {
            std::env::set_current_dir(&elsewhere).unwrap();
            assert!(matches!(backend.open(&source), Err(Error::Conflict)));
            assert!(matches!(backend.open(&destination), Err(Error::Conflict)));
            Ok(())
        };
        let replacement: Box<dyn Storage> = match backend {
            Backend::Sqlite => Box::new(
                rom_sqlite::Sqlite::migrate_from_observed(
                    "database",
                    "replacement",
                    &plan,
                    BackupLimits::default(),
                    observe,
                )
                .unwrap(),
            ),
            Backend::Redb => Box::new(
                rom_redb::Redb::migrate_from_observed(
                    "database",
                    "replacement",
                    &plan,
                    BackupLimits::default(),
                    observe,
                )
                .unwrap(),
            ),
        };
        assert_eq!(std::env::current_dir().unwrap(), elsewhere);
        assert_saved(&*replacement, "canonical source", 1);
        assert!(destination.is_file());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_eq!(
            fs::read(elsewhere.join("database")).unwrap(),
            b"unrelated source"
        );
        assert_eq!(
            fs::read(elsewhere.join("replacement")).unwrap(),
            b"unrelated destination"
        );
        drop(replacement);
        let reopened = backend.open(&destination).unwrap();
        assert_saved(&*reopened, "canonical source", 1);
    }
}

#[test]
fn callback_cwd_change_keeps_native_maintenance_paths_canonical() {
    let scratch = Scratch::new();
    let parent_directory = std::env::current_dir().unwrap();
    let mut child = Process::spawn(
        "offline_recovery::maintenance_cwd_child",
        &scratch.0,
        Backend::Sqlite,
    );
    assert!(
        child.wait().success(),
        "isolated cwd callback regression failed"
    );
    assert_eq!(std::env::current_dir().unwrap(), parent_directory);
}
