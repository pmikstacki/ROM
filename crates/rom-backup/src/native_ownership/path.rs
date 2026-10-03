//! Canonical database identity before and after ownership acquisition.
use super::{
    NativeAccess,
    metadata::{self, Identity},
};
use rom::{Error, Result};
use std::os::unix::ffi::OsStrExt;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;

pub(super) struct Prepared {
    pub path: PathBuf,
    before: Option<Identity>,
}

impl Prepared {
    pub(super) fn new(path: &Path, access: NativeAccess) -> Result<Self> {
        validate_name(path)?;
        let (path, before) = match metadata::inspect(path)? {
            Some(metadata) => {
                let canonical = fs::canonicalize(path).map_err(|error| {
                    if metadata.is_symlink() && error.kind() == io::ErrorKind::NotFound {
                        Error::Unsupported("dangling database symlinks are not supported".into())
                    } else {
                        Error::Storage
                    }
                })?;
                validate_name(&canonical)?;
                let identity =
                    metadata::single_file(&metadata::inspect(&canonical)?.ok_or(Error::Storage)?)?;
                (canonical, Some(identity))
            }
            None => {
                if access == NativeAccess::Existing {
                    return Err(Error::Storage);
                }
                let parent = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new("."));
                let canonical = fs::canonicalize(parent)
                    .map_err(|_| Error::Storage)?
                    .join(path.file_name().ok_or(Error::Storage)?);
                (canonical, None)
            }
        };
        Ok(Self { path, before })
    }

    pub(super) fn recheck(&self, access: NativeAccess) -> Result<()> {
        match metadata::inspect(&self.path)? {
            Some(metadata) => {
                let after = metadata::single_file(&metadata)?;
                if let Some(before) = self.before {
                    metadata::unchanged(before, after)?;
                }
                if access == NativeAccess::Fresh {
                    return Err(Error::Conflict);
                }
            }
            None => {
                if self.before.is_some() || access == NativeAccess::Existing {
                    return Err(Error::Storage);
                }
            }
        }
        Ok(())
    }
}

fn validate_name(path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::Unsupported("native database path must name a file".into()))?;
    if name.as_bytes().ends_with(b".rom-owner") {
        return Err(Error::Unsupported(
            ".rom-owner is reserved for native ownership metadata".into(),
        ));
    }
    Ok(())
}
