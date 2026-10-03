use super::*;

#[test]
fn storage_ownership_excludes_until_guard_drops() {
    let ownership = StorageOwnership::default();
    let owner = ownership.acquire().unwrap();
    assert!(matches!(ownership.acquire(), Err(Error::Conflict)));
    drop(owner);
    let next = ownership.acquire().unwrap();
    assert!(matches!(ownership.acquire(), Err(Error::Conflict)));
    drop(next);
    assert!(ownership.acquire().is_ok());
}

#[test]
fn storage_ownership_clones_share_claim_and_guard_outlives_helper() {
    let ownership = StorageOwnership::default();
    let clone = ownership.clone();
    let owner = ownership.acquire().unwrap();
    drop(ownership);
    assert!(matches!(clone.acquire(), Err(Error::Conflict)));
    drop(owner);
    assert!(clone.acquire().is_ok());
}

#[test]
fn storage_ownership_guard_can_move_between_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<StorageOwner>();
    let ownership = StorageOwnership::default();
    let owner = ownership.acquire().unwrap();
    std::thread::spawn(move || drop(owner)).join().unwrap();
    assert!(ownership.acquire().is_ok());
}
