use rom::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Resource)]
#[resource(name = "receipt-version-records", version = 2)]
struct Record {
    label: String,
}

struct Fixture {
    storage: Box<dyn Storage>,
    path: PathBuf,
}
impl Fixture {
    fn new(redb: bool) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-receipt-version-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let storage: Box<dyn Storage> = if redb {
            Box::new(rom_redb::Redb::open(&path).unwrap())
        } else {
            Box::new(rom_sqlite::Sqlite::open(&path).unwrap())
        };
        storage.register(&[Record::descriptor()]).unwrap();
        Self { storage, path }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn bundle(version: Option<u32>) -> Bundle {
    Bundle {
        expected: None,
        receipt: Receipt {
            replay_version: version,
            identity: "create".into(),
            fingerprint: "original".into(),
            row: Row {
                key: Key {
                    kind: Record::KIND.into(),
                    id: "one".into(),
                },
                revision: 1,
                value: Some(json!({"label":"original"})),
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
#[test]
fn new_receipt_requires_the_registered_version_without_partial_writes() {
    for redb in [false, true] {
        for version in [0, 1, 3] {
            let fixture = Fixture::new(redb);
            let rejected = bundle(Some(version));
            assert_eq!(
                fixture.storage.commit(&rejected),
                Err(Error::invalid(Record::KIND, "replay version")),
                "backend redb={redb}, version={version}"
            );
            assert!(
                fixture
                    .storage
                    .load(&rejected.receipt.row.key)
                    .unwrap()
                    .is_none()
            );
            assert!(fixture.storage.receipt("create").unwrap().is_none());
            assert!(
                fixture
                    .storage
                    .journal(Record::KIND, None, 10, 100_000)
                    .unwrap()
                    .events
                    .is_empty()
            );
            let accepted = bundle(Some(2));
            assert_eq!(fixture.storage.commit(&accepted).unwrap(), accepted.receipt);
        }
    }
}
#[test]
fn matching_receipt_replay_precedes_candidate_version_validation() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let committed = fixture.storage.commit(&bundle(Some(2))).unwrap();
        for version in [Some(0), Some(1), Some(3), None] {
            let mut candidate = bundle(version);
            assert_eq!(fixture.storage.commit(&candidate).unwrap(), committed);
            candidate.receipt.fingerprint = "different".into();
            assert_eq!(
                fixture.storage.commit(&candidate),
                Err(Error::IdentityMismatch)
            );
        }
        assert_eq!(
            fixture
                .storage
                .journal(Record::KIND, None, 10, 100_000)
                .unwrap()
                .events
                .len(),
            1
        );
    }
}
#[test]
fn unmarked_receipt_uses_the_registered_version() {
    for redb in [false, true] {
        let fixture = Fixture::new(redb);
        let candidate = bundle(None);
        assert_eq!(
            fixture.storage.commit(&candidate).unwrap(),
            candidate.receipt
        );
    }
}
