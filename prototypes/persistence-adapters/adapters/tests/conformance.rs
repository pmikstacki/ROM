use rom_persistence_adapters::{RedbStore, SqliteStore};
use rom_persistence_core::*;
use std::path::Path;
use std::sync::{Arc, Barrier};

fn command(action: &str, expected_revision: Option<u64>) -> Transition {
    Transition {
        action: action.into(),
        key: ResourceKey::new("tasks", "one"),
        expected_revision,
        value: Some(b"open".to_vec()),
        events: vec![b"created".to_vec(), b"indexed".to_vec()],
    }
}

fn atomic_restart<S: Storage>(open: fn(&Path) -> Result<S, Error>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let c = command("create", None);
    let cursor;
    {
        let runtime = Runtime::new(open(&path).unwrap(), Requirements::default()).unwrap();
        assert_eq!(runtime.execute(&c).unwrap().revision, 1);
        assert_eq!(
            runtime.load(&c.key).unwrap().unwrap().value,
            Some(b"open".to_vec())
        );
        let page = runtime.journal(None, 1).unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.events[0].payload, b"created");
        cursor = page.cursor;
    }
    let runtime = Runtime::new(open(&path).unwrap(), Requirements::default()).unwrap();
    assert_eq!(runtime.load(&c.key).unwrap().unwrap().revision, 1);
    assert!(matches!(
        runtime.receipt("create").unwrap(),
        ReceiptStatus::Found(Receipt { revision: 1, .. })
    ));
    let page = runtime.journal(Some(&cursor), 1).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].ordinal, 1);
    assert_eq!(page.events[0].payload, b"indexed");
    assert!(
        runtime
            .journal(Some(&page.cursor), 1)
            .unwrap()
            .events
            .is_empty()
    );
}

fn duplicate_and_conflict<S: Storage>(open: fn(&Path) -> Result<S, Error>) {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("db")).unwrap();
    let c = command("create", None);
    let original = store.commit(&c).unwrap();
    assert_eq!(store.commit(&c).unwrap(), original);
    let mut changed = c.clone();
    changed.events = vec![b"different".to_vec()];
    assert_eq!(store.commit(&changed), Err(Error::IdentityMismatch));
    assert_eq!(
        store.commit(&command("loser", None)),
        Err(Error::Conflict { actual: Some(1) })
    );
    assert_eq!(store.receipt("loser").unwrap(), ReceiptStatus::AbsentNow);
    assert_eq!(store.journal(None, 32).unwrap().events.len(), 2);
    let mut deletion = command("delete", Some(1));
    deletion.value = None;
    store.commit(&deletion).unwrap();
    assert_eq!(store.load(&c.key).unwrap().unwrap().value, None);
    assert_eq!(
        store.commit(&command("recreate-wrong", None)),
        Err(Error::Conflict { actual: Some(2) })
    );
    assert_eq!(
        store
            .commit(&command("recreate", Some(2)))
            .unwrap()
            .revision,
        3
    );
    // A retry must resolve its old receipt before testing today's revision.
    assert_eq!(store.commit(&c).unwrap(), original);
}

fn competing_writers<S: Storage + Clone + 'static>(open: fn(&Path) -> Result<S, Error>) {
    let dir = tempfile::tempdir().unwrap();
    let store = open(&dir.path().join("db")).unwrap();
    store.commit(&command("create", None)).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = ["a", "b"]
        .into_iter()
        .map(|id| {
            let s = store.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                b.wait();
                s.commit(&command(id, Some(1)))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(Error::Conflict { actual: Some(2) }))
            .count(),
        1
    );
    assert_eq!(
        store
            .load(&command("", None).key)
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    assert_eq!(store.journal(None, 32).unwrap().events.len(), 4);
}

fn capability_and_limits<S: Storage>(open: fn(&Path) -> Result<S, Error>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    assert!(matches!(
        Runtime::new(
            open(&path).unwrap(),
            Requirements {
                snapshot_queries: true,
                ..Requirements::default()
            }
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        Runtime::new(
            open(&path).unwrap(),
            Requirements {
                multi_resource_commit: true,
                ..Requirements::default()
            }
        ),
        Err(Error::Unsupported(_))
    ));
    let store = open(&path).unwrap();
    let mut c = command("large", None);
    c.value = Some(vec![0; store.capabilities().max_bytes + 1]);
    assert!(matches!(store.commit(&c), Err(Error::Rejected(_))));
    assert_eq!(store.load(&c.key).unwrap(), None);
    assert_eq!(store.receipt(&c.action).unwrap(), ReceiptStatus::AbsentNow);
    c.value = None;
    c.events = vec![vec![]; store.capabilities().max_events + 1];
    assert!(matches!(store.commit(&c), Err(Error::Rejected(_))));
    assert!(store.journal(None, 32).unwrap().events.is_empty());
    assert!(matches!(store.journal(None, 0), Err(Error::Rejected(_))));
}

fn cursor_scope_and_tail<S: Storage>(open: fn(&Path) -> Result<S, Error>) {
    let dir = tempfile::tempdir().unwrap();
    let first = open(&dir.path().join("first")).unwrap();
    let second = open(&dir.path().join("second")).unwrap();
    let empty = first.journal(None, 1).unwrap();
    first.commit(&command("create", None)).unwrap();
    let page = first.journal(Some(&empty.cursor), 1).unwrap();
    assert_eq!(page.events.len(), 1);
    assert_eq!(
        second.journal(Some(&page.cursor), 1),
        Err(Error::InvalidCursor)
    );
    assert_eq!(
        first.journal(Some(&Cursor::from_adapter_bytes(vec![0])), 1),
        Err(Error::InvalidCursor)
    );
    first.commit(&command("update", Some(1))).unwrap();
    let mut cursor = page.cursor;
    let mut seen = vec![];
    loop {
        let page = first.journal(Some(&cursor), 1).unwrap();
        cursor = page.cursor;
        if page.events.is_empty() {
            break;
        }
        seen.push((page.events[0].action.clone(), page.events[0].ordinal));
    }
    assert_eq!(
        seen,
        vec![
            ("create".into(), 1),
            ("update".into(), 0),
            ("update".into(), 1)
        ]
    );
}

macro_rules! suite {
    ($module:ident, $store:ty) => {
        mod $module {
            use super::*;
            #[test]
            fn atomic_bundle_survives_reopen() {
                atomic_restart(<$store>::open);
            }
            #[test]
            fn identity_conflict_and_tombstones() {
                duplicate_and_conflict(<$store>::open);
            }
            #[test]
            fn one_concurrent_revision_winner() {
                competing_writers(<$store>::open);
            }
            #[test]
            fn rejects_unsupported_and_oversize() {
                capability_and_limits(<$store>::open);
            }
            #[test]
            fn scoped_cursor_has_no_tail_gaps() {
                cursor_scope_and_tail(<$store>::open);
            }
        }
    };
}
suite!(sqlite, SqliteStore);
suite!(redb, RedbStore);
