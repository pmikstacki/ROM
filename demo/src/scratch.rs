//! Private temporary directories for finite local application journeys.
pub(crate) struct Scratch(pub(crate) std::path::PathBuf);
impl Scratch {
    pub(crate) fn new(label: &str) -> crate::smoke::SmokeResult<Self> {
        let path = std::env::temp_dir().join(format!(
            "rom-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
