//! Bounded opened-handle admission for immutable files in host-trusted directories.
use rom::{Error, Result};
use std::path::Path;

#[cfg(feature = "provider-profile")]
pub(crate) fn read_private(path: &Path, max: usize) -> Result<String> {
    read(path, max, true, OwnerPolicy::None, false)
}
#[cfg(feature = "studio")]
pub(crate) fn read_owned_private(path: &Path, max: usize) -> Result<String> {
    read(path, max, true, OwnerPolicy::Current, true)
}
#[cfg(feature = "studio")]
pub(crate) fn read_regular(path: &Path, max: usize) -> Result<String> {
    read(path, max, false, OwnerPolicy::Trusted, true)
}
#[cfg(feature = "studio")]
pub(crate) fn read_systemd_credential(
    path: &Path,
    max: usize,
    credentials_directory: &Path,
) -> Result<String> {
    if !credentials_directory.is_absolute() || path.parent() != Some(credentials_directory) {
        return Err(Error::Denied);
    }
    read(path, max, false, OwnerPolicy::SystemdCredential, true)
}
#[derive(Clone, Copy)]
enum OwnerPolicy {
    #[cfg(feature = "provider-profile")]
    None,
    #[cfg(feature = "studio")]
    Current,
    #[cfg(feature = "studio")]
    Trusted,
    #[cfg(feature = "studio")]
    SystemdCredential,
}
#[cfg(target_os = "linux")]
fn read(
    path: &Path,
    max: usize,
    private: bool,
    owner: OwnerPolicy,
    absolute: bool,
) -> Result<String> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    if absolute && !path.is_absolute() {
        return Err(Error::Denied);
    }
    let bound = u64::try_from(max).map_err(|_| Error::Denied)?;
    let read_bound = bound.checked_add(1).ok_or(Error::Denied)?;
    // Check the opened descriptor. NOFOLLOW closes the precheck/open link race;
    // NONBLOCK prevents a replaced FIFO from blocking before metadata admission.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::Denied)?;
    let metadata = file.metadata().map_err(|_| Error::Denied)?;
    #[cfg(feature = "studio")]
    let current_uid = || {
        std::fs::metadata("/proc/self")
            .map(|process| process.uid())
            .map_err(|_| Error::Denied)
    };
    let owner_admitted = match owner {
        #[cfg(feature = "provider-profile")]
        OwnerPolicy::None => true,
        #[cfg(feature = "studio")]
        OwnerPolicy::Current => metadata.uid() == current_uid()?,
        #[cfg(feature = "studio")]
        OwnerPolicy::Trusted => {
            let uid = current_uid()?;
            (metadata.uid() == 0 || metadata.uid() == uid)
                && metadata.mode() & 0o022 == 0
                && (metadata.uid() != uid || metadata.mode() & 0o200 == 0)
        }
        #[cfg(feature = "studio")]
        OwnerPolicy::SystemdCredential => {
            (metadata.uid() == 0 || metadata.uid() == current_uid()?)
                && matches!(metadata.mode() & 0o777, 0o400 | 0o440)
        }
    };
    if !metadata.is_file()
        || metadata.len() > bound
        || (private && metadata.mode() & 0o077 != 0)
        || !owner_admitted
    {
        return Err(Error::Denied);
    }
    let mut bytes = Vec::new();
    file.take(read_bound)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Denied)?;
    if bytes.is_empty() || bytes.len() > max {
        return Err(Error::Denied);
    }
    String::from_utf8(bytes).map_err(|_| Error::Denied)
}
#[cfg(not(target_os = "linux"))]
fn read(_: &Path, _: usize, _: bool, _: OwnerPolicy, _: bool) -> Result<String> {
    // Preserve the reference profile's Linux boundary; never use a weaker fallback.
    Err(Error::Denied)
}
