use super::{NativeAccess, NativeOwnership, test_support::Directory};
use rom::Error;

#[test]
fn incidental_descriptor_duplicate_does_not_extend_guard_ownership() {
    let directory = Directory::new();
    let path = directory.database();
    let owner = NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
    // A fork can retain the same open-file description until exec closes it.
    // Duplicating the private handle models that window without unsafe fork code.
    let inherited = owner._lock.file.try_clone().unwrap();
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Existing).err(),
        Some(Error::Conflict)
    );
    drop(owner);
    let replacement = NativeOwnership::acquire(&path, NativeAccess::Existing)
        .expect("guard drop must release ownership despite an incidental descriptor duplicate");
    drop(inherited);
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Existing).err(),
        Some(Error::Conflict),
        "closing the old duplicate must not release the replacement owner's lock"
    );
    drop(replacement);
    NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
}
