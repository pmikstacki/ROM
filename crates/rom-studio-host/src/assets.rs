use rom::{Error, Result};
use std::io::Read;
use std::{collections::BTreeMap, path::Path, sync::Arc};

#[derive(Clone)]
pub(crate) struct Asset {
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) content_type: &'static str,
}
pub(crate) struct Assets {
    files: BTreeMap<String, Asset>,
}
impl Assets {
    pub(crate) fn load(root: &Path, count: usize, bytes: usize) -> Result<Self> {
        if count == 0 || bytes == 0 {
            return Err(Error::TooLarge);
        }
        let mut files = BTreeMap::new();
        let mut remaining = bytes;
        let mut entries = count;
        load_directory(root, root, &mut entries, &mut remaining, &mut files)?;
        if !files.contains_key("index.html") {
            return Err(Error::Storage);
        }
        Ok(Self { files })
    }
    pub(crate) fn get(&self, relative: &str) -> Option<Asset> {
        if relative.starts_with('/')
            || relative.contains('%')
            || relative.contains('\\')
            || relative
                .split('/')
                .any(|part| part == ".." || part == "." || part.is_empty())
        {
            return None;
        }
        self.files
            .get(relative)
            .or_else(|| {
                if !relative.rsplit('/').next()?.contains('.') {
                    self.files.get("index.html")
                } else {
                    None
                }
            })
            .cloned()
    }
}
fn load_directory(
    root: &Path,
    directory: &Path,
    entries: &mut usize,
    remaining: &mut usize,
    files: &mut BTreeMap<String, Asset>,
) -> Result<()> {
    if std::fs::symlink_metadata(directory)
        .map_err(|_| Error::Storage)?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Storage);
    }
    for entry in std::fs::read_dir(directory).map_err(|_| Error::Storage)? {
        *entries = entries.checked_sub(1).ok_or(Error::TooLarge)?;
        let entry = entry.map_err(|_| Error::Storage)?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|_| Error::Storage)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Storage);
        }
        if metadata.is_dir() {
            load_directory(root, &path, entries, remaining, files)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(Error::TooLarge);
        }
        let size = usize::try_from(metadata.len()).map_err(|_| Error::TooLarge)?;
        if size > *remaining {
            return Err(Error::TooLarge);
        }
        // Startup assets are host-owned. Check the actual read size as well as metadata.
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .map_err(|_| Error::Storage)?
            .take((*remaining as u64).saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Storage)?;
        *remaining = remaining.checked_sub(bytes.len()).ok_or(Error::TooLarge)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Error::Storage)?
            .to_str()
            .ok_or(Error::Storage)?
            .replace('\\', "/");
        files.insert(
            relative,
            Asset {
                bytes: bytes.into(),
                content_type: mime(&path),
            },
        );
    }
    Ok(())
}
fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
