//! Native ownership tests use actual adapters, not a mocked filesystem lock.
#[path = "ownership/support.rs"]
mod support;
use rom::{Error, Resource, Storage};
use support::{BACKENDS, Scratch};

#[test]
fn native_duplicate_open_conflicts_until_final_adapter_drop() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let first = backend.open(&scratch.path()).unwrap();
        assert!(
            matches!(backend.open(&scratch.path()), Err(Error::Conflict)),
            "{backend:?}: second native owner must conflict"
        );
        drop(first);
        let replacement = backend.open(&scratch.path()).unwrap();
        drop(replacement);
    }
}

#[cfg(unix)]
#[test]
fn symlink_alias_cannot_create_a_second_native_owner() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        let first = backend.open(&scratch.path()).unwrap();
        let alias = scratch.0.join("alias");
        std::os::unix::fs::symlink(scratch.path(), &alias).unwrap();
        assert!(
            matches!(backend.open(&alias), Err(Error::Conflict)),
            "{backend:?}: symlink alias must share ownership"
        );
        drop(first);
        drop(backend.open(&alias).unwrap());
    }
}

#[cfg(unix)]
#[test]
fn hardlinked_database_is_rejected_before_native_open() {
    for backend in BACKENDS {
        let scratch = Scratch::new();
        drop(backend.open(&scratch.path()).unwrap());
        let alias = scratch.0.join("hardlink");
        std::fs::hard_link(scratch.path(), &alias).unwrap();
        let before = std::fs::read(scratch.path()).unwrap();
        assert!(
            matches!(backend.open(&alias), Err(Error::Unsupported(_))),
            "{backend:?}: hardlinks must fail closed"
        );
        assert_eq!(std::fs::read(scratch.path()).unwrap(), before);
    }
}

#[derive(Clone, Resource)]
#[resource(name = "ownership-memory-records")]
struct MemoryRecord {
    value: String,
}

#[test]
fn sqlite_memory_sentinel_keeps_stores_independent() {
    let first = rom_sqlite::Sqlite::open(":memory:").unwrap();
    let second = rom_sqlite::Sqlite::open(":memory:").unwrap();
    first.register(&[MemoryRecord::descriptor()]).unwrap();
    assert_eq!(first.counts().unwrap(), [0; 4]);
    assert_eq!(second.counts().unwrap(), [0; 4]);
    // Distinct catalogs establish independent stores without relying on empty row counts.
    let mut different = MemoryRecord::descriptor();
    different.fields.clear();
    second.register(&[different]).unwrap();
}

#[path = "ownership/process.rs"]
mod process;

#[path = "ownership/maintenance.rs"]
mod maintenance;

#[path = "ownership/offline_recovery.rs"]
mod offline_recovery;

#[test]
fn sqlite_uri_input_is_explicitly_unsupported_instead_of_reinterpreted() {
    let scratch = Scratch::new();
    let uri = format!("file:{}?mode=ro", scratch.path().display());
    assert!(matches!(
        rom_sqlite::Sqlite::open(&uri),
        Err(Error::Unsupported(_))
    ));
    assert!(!scratch.path().exists());
    assert!(!scratch.0.join("database.rom-owner").exists());
    // A colon inside an explicit filesystem path is an ordinary filename.
    drop(rom_sqlite::Sqlite::open(scratch.0.join("file:literal")).unwrap());
}
