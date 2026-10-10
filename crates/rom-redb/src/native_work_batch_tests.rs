//! Explicit atomic-group capability for predecessor files.
use rom::{Error, Storage, StorageLimits, WorkResult, WorkUpdate};

#[test]
fn atomic_predecessor_rejects_groups_and_keeps_singleton_idle() {
    let directory = std::env::temp_dir().join(format!(
        "rom-redb-atomic-predecessor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("db");
    let db = crate::Redb::open_owned_in_format(
        rom_backup::NativeOwnership::acquire(&path, rom_backup::NativeAccess::OpenOrCreate)
            .unwrap(),
        StorageLimits::default(),
        rom_backup::BackupLimits::default(),
        10,
    )
    .unwrap();
    let before = db.reaction_records().unwrap();
    assert!(matches!(
        db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }; 2]),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(db.reaction_records().unwrap(), before);
    assert_eq!(db.reaction_updates_atomic(vec![]), Ok(vec![]));
    assert_eq!(
        db.reaction_updates_atomic(vec![WorkUpdate::Claim { now: 0 }]),
        Ok(vec![WorkResult::Idle])
    );
}
