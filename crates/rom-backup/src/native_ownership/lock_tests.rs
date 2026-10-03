use super::super::test_support::{Directory, sidecar, unsupported};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn sidecar_replacement_is_detected_while_the_original_inode_remains_locked() {
    let directory = Directory::new();
    let database = directory.database();
    let path = sidecar(&database);
    let held = super::acquire(&database).unwrap();
    fs::rename(&path, directory.file("old-owner")).unwrap();
    fs::write(&path, b"new inode").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    unsupported(super::verify(&path, &held.file));
    fs::remove_file(&path).unwrap();
    assert_eq!(super::verify(&path, &held.file), Err(rom::Error::Storage));
}

#[test]
fn post_lock_permission_and_link_changes_are_not_accepted() {
    let directory = Directory::new();
    let database = directory.database();
    let path = sidecar(&database);
    let held = super::acquire(&database).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    unsupported(super::verify(&path, &held.file));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::hard_link(&path, directory.file("second-owner-name")).unwrap();
    unsupported(super::verify(&path, &held.file));
}
