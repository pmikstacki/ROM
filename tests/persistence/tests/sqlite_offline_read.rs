//! Offline readers preserve database bytes and existing ownership metadata.
use rom::{Actor, Command, Resource, Runtime, Storage};
use rom_backup::{BackupLimits, MigrationPlan, ResourceMigration};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Resource)]
#[resource(name = "offline")]
struct Before {
    value: String,
}
#[derive(Clone, Resource)]
#[resource(name = "offline", version = 2)]
struct After {
    value: String,
}
fn plan() -> MigrationPlan {
    MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|old| Ok(After { value: old.value })).unwrap(),
    ])
    .unwrap()
}
#[cfg(unix)]
fn wait_for_exit(mut child: std::process::Child) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("SQLite WAL child timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rom-offline-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn files(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}
#[tokio::test]
async fn clean_source_stays_identical_after_success_and_rejected_conversion() {
    let scratch = Scratch::new();
    let inputs = scratch.0.join("inputs");
    std::fs::create_dir(&inputs).unwrap();
    // These characters must be filename data, never SQLite URI options.
    let source = inputs.join("snow 雪 ?immutable=0&mode=rw#%2f.db");
    let runtime = Runtime::builder()
        .resource(
            Before::definition()
                .policy(|_, _, _| true)
                .allow_all_fields(),
        )
        .build(
            Arc::new(rom_sqlite::Sqlite::open(&source).unwrap()),
            Runtime::shared_cpu_pool(1).unwrap(),
        )
        .unwrap();
    runtime
        .execute(
            &Actor::trusted("test", "author"),
            Command::create(
                "one",
                Before {
                    value: "retained".into(),
                },
            )
            .idempotency("create"),
        )
        .await
        .unwrap();
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let original = files(&inputs);
    let mut owner_path = source.as_os_str().to_os_string();
    owner_path.push(".rom-owner");
    let owner_path = PathBuf::from(owner_path);
    assert_eq!(
        original.len(),
        2,
        "only database and persistent ownership metadata exist"
    );
    assert!(original.iter().any(|(path, _)| path == &source));
    assert!(
        original
            .iter()
            .any(|(path, bytes)| path == &owner_path && bytes.is_empty())
    );
    let invalid = MigrationPlan::new(vec![
        ResourceMigration::new::<Before, After>(|_| Err(rom::Error::Storage)).unwrap(),
    ])
    .unwrap();
    let destination = scratch.0.join("rejected");
    assert!(
        rom_sqlite::Sqlite::migrate_from(&source, &destination, &invalid, BackupLimits::default())
            .is_err()
    );
    assert!(!destination.exists());
    assert!(
        files(&inputs) == original,
        "rejected conversion must not change source or sidecar inventory"
    );
    let valid = plan();
    let migrated = rom_sqlite::Sqlite::migrate_from(
        &source,
        scratch.0.join("valid"),
        &valid,
        BackupLimits::default(),
    )
    .unwrap();
    let row = migrated
        .load(&rom::Key {
            kind: "offline".into(),
            id: "one".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(row.value, Some(rom::json!({"value":"retained"})));
    assert!(
        files(&inputs) == original,
        "successful migration must not change source or sidecar inventory"
    );
}
#[test]
fn rollback_journal_requires_recovery_without_changing_source() {
    let scratch = Scratch::new();
    let inputs = scratch.0.join("inputs");
    std::fs::create_dir(&inputs).unwrap();
    let source = inputs.join("source");
    let store = rom_sqlite::Sqlite::open(&source).unwrap();
    store.register(&[Before::descriptor()]).unwrap();
    drop(store);
    std::fs::write(inputs.join("source-journal"), []).unwrap();
    let original = files(&inputs);
    let valid = plan();
    let destination = scratch.0.join("destination");
    assert!(
        rom_sqlite::Sqlite::migrate_from(&source, &destination, &valid, BackupLimits::default())
            .is_err()
    );
    assert!(!destination.exists());
    assert!(files(&inputs) == original);
}
#[test]
fn missing_source_never_creates_source_or_destination() {
    let scratch = Scratch::new();
    let source = scratch.0.join("missing ?mode=rwc.db");
    let destination = scratch.0.join("destination");
    let valid = plan();
    assert!(
        rom_sqlite::Sqlite::migrate_from(&source, &destination, &valid, BackupLimits::default())
            .is_err()
    );
    assert!(files(&scratch.0).is_empty());
}

#[cfg(unix)]
#[test]
fn symlink_to_offline_dirty_source_reads_committed_wal() {
    let scratch = Scratch::new();
    let source = scratch.0.join("actual");
    let status = wait_for_exit(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "sqlite_wal_child"])
            .env("ROM_OFFLINE_SOURCE", &source)
            .spawn()
            .unwrap(),
    );
    assert_eq!(status.code(), Some(86));
    let wal = PathBuf::from(format!("{}-wal", source.display()));
    let original = std::fs::read(&source).unwrap();
    let original_wal = std::fs::read(&wal).unwrap();
    assert!(!original_wal.is_empty());
    let aliases = scratch.0.join("aliases");
    std::fs::create_dir(&aliases).unwrap();
    let alias = aliases.join("source");
    std::os::unix::fs::symlink(&source, &alias).unwrap();
    let valid = plan();
    let migrated = rom_sqlite::Sqlite::migrate_from(
        &alias,
        scratch.0.join("migrated"),
        &valid,
        BackupLimits::default(),
    )
    .unwrap();
    let row = migrated
        .load(&rom::Key {
            kind: "offline".into(),
            id: "one".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(row.value, Some(rom::json!({"value":"from-wal"})));
    assert!(std::fs::read(&source).unwrap() == original);
    assert!(std::fs::read(&wal).unwrap() == original_wal);
    assert_eq!(std::fs::read_dir(&aliases).unwrap().count(), 1);
}
#[test]
#[ignore = "invoked by parent to retain committed WAL without finalizers"]
fn sqlite_wal_child() {
    let Some(source) = std::env::var_os("ROM_OFFLINE_SOURCE") else {
        return;
    };
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let runtime = Runtime::builder()
            .resource(
                Before::definition()
                    .policy(|_, _, _| true)
                    .allow_all_fields(),
            )
            .build(
                Arc::new(rom_sqlite::Sqlite::open(PathBuf::from(source)).unwrap()),
                Runtime::shared_cpu_pool(1).unwrap(),
            )
            .unwrap();
        runtime
            .execute(
                &Actor::trusted("test", "author"),
                Command::create(
                    "one",
                    Before {
                        value: "from-wal".into(),
                    },
                )
                .idempotency("create"),
            )
            .await
            .unwrap();
        std::process::exit(86);
    });
}
#[cfg(unix)]
#[test]
fn non_utf8_clean_source_path_is_preserved() {
    use std::os::unix::ffi::OsStringExt;
    let scratch = Scratch::new();
    let inputs = scratch.0.join("inputs");
    std::fs::create_dir(&inputs).unwrap();
    let source = inputs.join(std::ffi::OsString::from_vec(b"native-\xff%?.db".to_vec()));
    let storage = rom_sqlite::Sqlite::open(&source).unwrap();
    storage.register(&[Before::descriptor()]).unwrap();
    drop(storage);
    let original = files(&inputs);
    let valid = plan();
    drop(
        rom_sqlite::Sqlite::migrate_from(
            &source,
            scratch.0.join("migrated"),
            &valid,
            BackupLimits::default(),
        )
        .unwrap(),
    );
    assert!(files(&inputs) == original);
}
