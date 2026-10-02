#![cfg(feature = "fault-injection")]
use rom_persistence_adapters::{
    RedbStore, SqliteStore,
    probe::{Checkpoint, Probe},
};
use rom_persistence_core::*;
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

trait Harness: Storage + Clone + Sized + 'static {
    fn open(path: &Path) -> Result<Self, Error>;
    fn with_probe(self, probe: Probe) -> Self;
    fn independent_reader(&self, path: &Path) -> Self;
}
impl Harness for SqliteStore {
    fn open(path: &Path) -> Result<Self, Error> {
        Self::open(path)
    }
    fn with_probe(self, probe: Probe) -> Self {
        self.with_probe(probe)
    }
    fn independent_reader(&self, path: &Path) -> Self {
        Self::open(path).unwrap()
    }
}
impl Harness for RedbStore {
    fn open(path: &Path) -> Result<Self, Error> {
        Self::open(path)
    }
    fn with_probe(self, probe: Probe) -> Self {
        self.with_probe(probe)
    }
    fn independent_reader(&self, _: &Path) -> Self {
        self.clone()
    }
}
fn command() -> Transition {
    Transition {
        action: "action".into(),
        key: ResourceKey::new("tasks", "one"),
        expected_revision: None,
        value: Some(b"committed".to_vec()),
        events: vec![b"one".to_vec(), b"two".to_vec()],
    }
}
fn rollback<S: Harness>() {
    for target in [
        Checkpoint::AfterResource,
        Checkpoint::AfterReceipt,
        Checkpoint::AfterEvent,
        Checkpoint::BeforeCommit,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db");
        let c = command();
        let store = S::open(&path).unwrap().with_probe(Probe::new(move |point| {
            if point == target {
                Err(Error::NotCommitted)
            } else {
                Ok(())
            }
        }));
        assert_eq!(
            store.commit(&c),
            Err(Error::NotCommitted),
            "checkpoint: {target:?}"
        );
        drop(store);
        let reopened = S::open(&path).unwrap();
        assert_eq!(reopened.load(&c.key).unwrap(), None);
        assert_eq!(
            reopened.receipt(&c.action).unwrap(),
            ReceiptStatus::AbsentNow
        );
        let empty = reopened.journal(None, 10).unwrap();
        assert!(empty.events.is_empty());
        reopened.commit(&c).unwrap();
        assert_eq!(
            reopened
                .journal(Some(&empty.cursor), 10)
                .unwrap()
                .events
                .len(),
            2
        );
    }
}
fn lost_ack<S: Harness>() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let store = S::open(&path).unwrap().with_probe(Probe::new(|point| {
        if point == Checkpoint::AfterCommit {
            Err(Error::Unknown {
                action: "action".into(),
            })
        } else {
            Ok(())
        }
    }));
    let runtime = Runtime::new(store, Requirements::default()).unwrap();
    assert_eq!(
        runtime.execute(&command()),
        Err(Error::Unknown {
            action: "action".into()
        })
    );
    drop(runtime);
    let runtime = Runtime::new(S::open(&path).unwrap(), Requirements::default()).unwrap();
    assert!(matches!(
        runtime.receipt("action").unwrap(),
        ReceiptStatus::Found(Receipt { revision: 1, .. })
    ));
    assert_eq!(runtime.execute(&command()).unwrap().revision, 1);
    assert_eq!(runtime.journal(None, 10).unwrap().events.len(), 2);
}
fn absent_while_in_flight<S: Harness>() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let store = S::open(&path).unwrap();
    let reader = store.independent_reader(&path);
    let retry = store.independent_reader(&path);
    let (entered, wait_entered) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let wait_release = Arc::new(Mutex::new(wait_release));
    let delayed = store.with_probe(Probe::new(move |point| {
        if point == Checkpoint::BeforeCommit {
            entered.send(()).unwrap();
            wait_release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
        Ok(())
    }));
    let original = std::thread::spawn(move || delayed.commit(&command()));
    wait_entered
        .recv_timeout(Duration::from_secs(2))
        .expect("writer reached precommit");
    // Represents a client deadline expiring while its native transaction remains in flight.
    assert_eq!(reader.receipt("action").unwrap(), ReceiptStatus::AbsentNow);
    assert_eq!(reader.load(&command().key).unwrap(), None);
    assert!(reader.journal(None, 10).unwrap().events.is_empty());
    let racing_retry = std::thread::spawn(move || retry.commit(&command()));
    release.send(()).unwrap();
    assert_eq!(original.join().unwrap().unwrap().revision, 1);
    assert_eq!(racing_retry.join().unwrap().unwrap().revision, 1);
    assert_eq!(reader.journal(None, 10).unwrap().events.len(), 2);
}
macro_rules! suite {
    ($module:ident, $store:ty) => {
        mod $module {
            use super::*;
            #[test]
            fn rollback_at_each_write_boundary() {
                rollback::<$store>();
            }
            #[test]
            fn lost_ack_resolves_after_restart() {
                lost_ack::<$store>();
            }
            #[test]
            fn absent_receipt_can_commit_later() {
                absent_while_in_flight::<$store>();
            }
        }
    };
}
suite!(sqlite, SqliteStore);
suite!(redb, RedbStore);
