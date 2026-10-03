use super::super::test_support::{Directory, sidecar, unsupported};
use super::{NativeAccess, Prepared};
use std::{fs, os::unix::fs::symlink};

#[test]
fn database_replacement_or_disappearance_fails_the_post_lock_identity_check() {
    let directory = Directory::new();
    let path = directory.database();
    let prepared = Prepared::new(&path, NativeAccess::Existing).unwrap();
    fs::rename(&path, directory.file("old-database")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    unsupported(prepared.recheck(NativeAccess::Existing));
    fs::remove_file(&path).unwrap();
    assert_eq!(
        prepared.recheck(NativeAccess::Existing),
        Err(rom::Error::Storage)
    );
    assert!(!sidecar(&path).exists());
}

#[test]
fn a_new_symlink_or_hardlink_cannot_replace_a_reserved_absent_path() {
    let directory = Directory::new();
    let target = directory.database();
    let path = directory.file("fresh");
    let prepared = Prepared::new(&path, NativeAccess::Fresh).unwrap();
    symlink(&target, &path).unwrap();
    unsupported(prepared.recheck(NativeAccess::Fresh));
    fs::remove_file(&path).unwrap();
    fs::hard_link(&target, &path).unwrap();
    unsupported(prepared.recheck(NativeAccess::Fresh));
}
