//! Checked filesystem identity; only absence is treated as absence.
use rom::{Error, Result};
use std::os::unix::fs::MetadataExt;
use std::{
    fs::{self, Metadata},
    io,
    path::Path,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}

pub(super) fn inspect(path: &Path) -> Result<Option<Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(Error::Storage),
    }
}

pub(super) fn single_file(metadata: &Metadata) -> Result<Identity> {
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(Error::Unsupported(
            "native ownership requires regular files with one link".into(),
        ));
    }
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub(super) fn unchanged(before: Identity, after: Identity) -> Result<()> {
    if before != after {
        return Err(Error::Unsupported(
            "native ownership path identity changed".into(),
        ));
    }
    Ok(())
}
