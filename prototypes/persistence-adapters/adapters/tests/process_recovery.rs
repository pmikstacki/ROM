#![cfg(feature = "fault-injection")]
use rom_persistence_adapters::{RedbStore, SqliteStore};
use rom_persistence_core::*;
use std::{path::Path, process::Command};

fn recovery<S: Storage>(name: &str, open: fn(&Path) -> Result<S, Error>) {
    for checkpoint in ["resource", "event", "precommit", "committed"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let status = Command::new(env!("CARGO_BIN_EXE_crash-probe"))
            .arg(name)
            .arg(&path)
            .arg(checkpoint)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let store = open(&path).unwrap();
        let key = ResourceKey::new("tasks", "crashed");
        if checkpoint == "committed" {
            assert_eq!(
                store.load(&key).unwrap().unwrap().value,
                Some(b"saved".to_vec())
            );
            assert!(matches!(
                store.receipt("crashed").unwrap(),
                ReceiptStatus::Found(Receipt { revision: 1, .. })
            ));
            let events = store.journal(None, 10).unwrap().events;
            assert_eq!(
                events
                    .iter()
                    .map(|e| e.payload.as_slice())
                    .collect::<Vec<_>>(),
                vec![b"first".as_slice(), b"second".as_slice()]
            );
        } else {
            assert_eq!(store.load(&key).unwrap(), None);
            assert_eq!(store.receipt("crashed").unwrap(), ReceiptStatus::AbsentNow);
            assert!(store.journal(None, 10).unwrap().events.is_empty());
        }
    }
}
#[test]
fn sqlite_process_exit_recovery() {
    recovery("sqlite", SqliteStore::open);
}
#[test]
fn redb_process_exit_recovery() {
    recovery("redb", RedbStore::open);
}
