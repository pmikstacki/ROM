use super::support::{Directory, sidecar, unsupported};
use rom::Error;
use rom_backup::{NativeAccess, NativeOwnership};
use std::os::unix::{
    ffi::OsStringExt,
    fs::{MetadataExt, PermissionsExt, symlink},
    net::UnixListener,
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[test]
fn separate_opens_conflict_until_guard_drop_and_keep_the_same_sidecar() {
    let directory = Directory::new();
    let path = directory.database();
    let owner = NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
    assert_eq!(owner.path(), fs::canonicalize(&path).unwrap());
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::OpenOrCreate).err(),
        Some(Error::Conflict)
    );
    let before = fs::metadata(sidecar(&path)).unwrap();
    assert_eq!(before.mode() & 0o777, 0o600);
    assert_eq!(before.nlink(), 1);
    drop(owner);
    let again = NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
    let after = fs::metadata(sidecar(&path)).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_eq!(fs::read(&path).unwrap(), b"unchanged database bytes");
    drop(again);
}

#[test]
fn relative_and_symlink_aliases_share_one_canonical_owner() {
    let directory = Directory::new();
    let path = directory.database();
    let file_alias = directory.file("alias");
    symlink(&path, &file_alias).unwrap();
    let parent_alias = directory.file("parent-alias");
    symlink(&directory.0, &parent_alias).unwrap();
    let mut relative = PathBuf::new();
    for _ in std::env::current_dir().unwrap().components().skip(1) {
        relative.push("..");
    }
    relative.push(path.strip_prefix(Path::new("/")).unwrap());
    let owner = NativeOwnership::acquire(&file_alias, NativeAccess::Existing).unwrap();
    for alias in [&path, &relative, &parent_alias.join("database")] {
        assert_eq!(
            NativeOwnership::acquire(alias, NativeAccess::Existing).err(),
            Some(Error::Conflict)
        );
    }
    assert!(!sidecar(&file_alias).exists());
    drop(owner);
    NativeOwnership::acquire(&relative, NativeAccess::Existing).unwrap();
}

#[test]
fn missing_paths_are_reserved_without_creating_databases_and_fresh_guard_can_transfer() {
    let directory = Directory::new();
    let path = directory.file("missing");
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Existing).err(),
        Some(Error::Storage)
    );
    assert!(!path.exists());
    assert!(!sidecar(&path).exists());
    let owner = NativeOwnership::acquire(&path, NativeAccess::Fresh).unwrap();
    assert!(!path.exists());
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::OpenOrCreate).err(),
        Some(Error::Conflict)
    );
    fs::write(owner.path(), b"published database").unwrap();
    let transferred = std::convert::identity(owner);
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Existing).err(),
        Some(Error::Conflict)
    );
    drop(transferred);
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Fresh).err(),
        Some(Error::Conflict)
    );
    NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
    let create = directory.file("created-later");
    NativeOwnership::acquire(&create, NativeAccess::OpenOrCreate).unwrap();
    assert!(!create.exists());
}

#[test]
fn missing_parent_aliases_and_non_utf8_names_preserve_native_path_identity() {
    let directory = Directory::new();
    let alias = directory.file("parent");
    symlink(&directory.0, &alias).unwrap();
    let name = OsString::from_vec(b"database-\xff".to_vec());
    let path = directory.0.join(&name);
    let aliased = alias.join(&name);
    let owner = NativeOwnership::acquire(&aliased, NativeAccess::Fresh).unwrap();
    assert_eq!(owner.path(), path);
    assert_eq!(
        NativeOwnership::acquire(&path, NativeAccess::Fresh).err(),
        Some(Error::Conflict)
    );
    assert!(sidecar(&path).exists());
}

#[test]
fn dangling_symlinks_hardlinks_nonregular_databases_and_reserved_names_are_rejected() {
    let directory = Directory::new();
    let path = directory.database();
    let alias = directory.file("hardlink");
    fs::hard_link(&path, &alias).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    unsupported(NativeOwnership::acquire(&alias, NativeAccess::Existing));
    assert!(!sidecar(&path).exists());
    let dangling = directory.file("dangling");
    symlink(directory.file("absent-target"), &dangling).unwrap();
    unsupported(NativeOwnership::acquire(
        &dangling,
        NativeAccess::OpenOrCreate,
    ));
    assert!(!directory.file("absent-target").exists());
    unsupported(NativeOwnership::acquire(
        &directory.0,
        NativeAccess::Existing,
    ));
    let socket = directory.file("socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    unsupported(NativeOwnership::acquire(&socket, NativeAccess::Existing));
    unsupported(NativeOwnership::acquire(
        &directory.file("reserved.rom-owner"),
        NativeAccess::Fresh,
    ));
    let reserved = directory.file("existing.rom-owner");
    fs::write(&reserved, b"not a database").unwrap();
    let indirect = directory.file("indirect");
    symlink(&reserved, &indirect).unwrap();
    unsupported(NativeOwnership::acquire(&indirect, NativeAccess::Existing));
}

#[test]
fn unsafe_owner_sidecars_are_rejected_without_following_or_truncating_them() {
    let directory = Directory::new();
    let path = directory.database();
    let owner = sidecar(&path);
    let target = directory.file("other");
    fs::write(&target, b"other bytes").unwrap();
    symlink(&target, &owner).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    assert_eq!(fs::read(&target).unwrap(), b"other bytes");
    fs::remove_file(&owner).unwrap();
    symlink(directory.file("missing-owner-target"), &owner).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    assert!(!directory.file("missing-owner-target").exists());
    fs::remove_file(&owner).unwrap();
    fs::create_dir(&owner).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    fs::remove_dir(&owner).unwrap();
    let listener = UnixListener::bind(&owner).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    drop(listener);
    fs::remove_file(&owner).unwrap();
    fs::write(&owner, b"persistent marker").unwrap();
    fs::set_permissions(&owner, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = directory.file("owner-hardlink");
    fs::hard_link(&owner, &alias).unwrap();
    unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    fs::remove_file(alias).unwrap();
    for mode in [0o644, 0o660, 0o606] {
        fs::set_permissions(&owner, fs::Permissions::from_mode(mode)).unwrap();
        unsupported(NativeOwnership::acquire(&path, NativeAccess::Existing));
    }
    fs::set_permissions(&owner, fs::Permissions::from_mode(0o600)).unwrap();
    NativeOwnership::acquire(&path, NativeAccess::Existing).unwrap();
    assert_eq!(fs::read(owner).unwrap(), b"persistent marker");
}

#[test]
fn metadata_errors_do_not_become_missing_file_creation() {
    let directory = Directory::new();
    let regular = directory.database();
    let nested = regular.join("not-a-directory");
    assert_eq!(
        NativeOwnership::acquire(&nested, NativeAccess::OpenOrCreate).err(),
        Some(Error::Storage)
    );
    let long = directory.file(&"x".repeat(256));
    assert_eq!(
        NativeOwnership::acquire(&long, NativeAccess::Fresh).err(),
        Some(Error::Storage)
    );
    assert_eq!(fs::read(regular).unwrap(), b"unchanged database bytes");
}
