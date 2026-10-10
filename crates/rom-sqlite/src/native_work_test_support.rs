//! Small real-store fixtures; no external provider or simulated persistence.
use crate::Sqlite;
use rom::*;

pub(super) fn descriptor() -> Descriptor {
    Descriptor {
        kind: "native-work".into(),
        version: 1,
        fields: vec![FieldDescriptor {
            name: "amount".into(),
            shape: Shape::U64,
        }],
    }
}
pub(super) fn limits() -> ReactionLimits {
    ReactionLimits {
        max_records: 4096,
        max_bytes: 16 * 1024 * 1024,
        ..ReactionLimits::default()
    }
}
pub(super) fn bundle(id: &str) -> Bundle {
    let row = Row {
        key: Key {
            kind: "native-work".into(),
            id: id.into(),
        },
        revision: 1,
        value: Some(json!({"amount": 1})),
        protected: Default::default(),
    };
    Bundle {
        expected: None,
        receipt: Receipt {
            identity: format!("create-{id}"),
            fingerprint: format!("native-{id}"),
            retry_epoch: 0,
            replay_version: None,
            row: row.clone(),
        },
        changed: true,
        effects: vec![],
        reactions: vec![PendingWork {
            id: format!("work-{id}"),
            cause: Cause {
                retry_epoch: 0,
                root: format!("root-{id}"),
                parent: None,
                depth: 1,
                started_at: 0,
                path: vec![format!("work-{id}")],
            },
            definition: "native-source".into(),
            version: 1,
            service_key: "native-service".into(),
            delivery_profile: DeliveryProfile::AtLeastOnce,
            not_before: None,
            payload: WorkPayload::Source(row),
        }],
        reaction_limits: Some(limits()),
        completed_work: None,
    }
}
pub(super) fn fixture() -> Sqlite {
    let db = Sqlite::open(":memory:").unwrap();
    db.register(&[descriptor()]).unwrap();
    db
}
pub(super) fn predecessor_fixture() -> Sqlite {
    let db = Sqlite::open_connection_for_layout(
        std::path::Path::new(":memory:"),
        StorageLimits::default(),
        rom_backup::BackupLimits::default(),
        None,
        false,
    )
    .unwrap();
    db.register(&[descriptor()]).unwrap();
    db
}
pub(super) fn claim(db: &Sqlite) -> WorkClaim {
    match db.reaction_update(WorkUpdate::Claim { now: 0 }).unwrap() {
        WorkResult::Claimed(claim) => *claim,
        other => panic!("expected claim, got {other:?}"),
    }
}
pub(super) fn done(db: &Sqlite) {
    let claim = claim(db);
    db.reaction_update(WorkUpdate::Materialize {
        claim: claim.key(),
        now: 0,
        children: vec![],
    })
    .unwrap();
}

pub(super) struct Scratch(pub(super) std::path::PathBuf);
impl Scratch {
    pub(super) fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-sqlite-incremental-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub(super) fn path(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if std::thread::panicking()
            || std::env::var("ROM_SQLITE_LARGE_NATIVE_BUDGET").as_deref() == Ok("1")
        {
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(super) fn file_fixture(path: &std::path::Path) -> Sqlite {
    let db = Sqlite::open(path).unwrap();
    db.register(&[descriptor()]).unwrap();
    db
}
