use std::os::unix::fs::DirBuilderExt;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub struct Directory(pub PathBuf);
impl Directory {
    pub fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rom-native-owner-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    pub fn database(&self) -> PathBuf {
        let path = self.file("database");
        fs::write(&path, b"unchanged database bytes").unwrap();
        path
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn sidecar(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".rom-owner");
    name.into()
}
pub fn unsupported<T>(result: rom::Result<T>) {
    assert!(
        matches!(result, Err(rom::Error::Unsupported(_))),
        "expected unsupported native path"
    );
}
