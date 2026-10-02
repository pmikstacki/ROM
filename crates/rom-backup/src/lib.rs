//! Bounded private logical maintenance archives. Never exposes archives through Resource APIs.
//! SHA-256 detects accidental corruption, not hostile modification. Use a trusted parent
//! directory. Archives contain protected values; external blobs and deliveries are excluded.
use rom::{Error, Intent, Receipt, Result, Row, StorageState};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug)]
pub struct BackupLimits {
    pub max_bytes: usize,
    pub max_records: usize,
}
impl Default for BackupLimits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_records: 400_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    Sqlite,
    Redb,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub archive_version: u32,
    pub storage_format: u32,
    pub backend: Backend,
    pub rows: usize,
    pub receipts: usize,
    pub events: usize,
    pub effects: usize,
    pub work: usize,
    pub external_blobs_included: bool,
    pub external_deliveries_included: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEffect {
    pub identity: String,
    pub ordinal: u64,
    pub intent: Intent,
}
/// Full private data for adapter implementations. Deliberately has no Debug implementation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub state: StorageState,
    pub rows: Vec<Row>,
    pub receipts: Vec<Receipt>,
    pub events: Vec<(String, Row)>,
    pub effects: Vec<StoredEffect>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let mut keys = BTreeMap::new();
        for row in &self.rows {
            if row.key.kind.is_empty()
                || row.key.id.is_empty()
                || row.revision == 0
                || row.revision > i64::MAX as u64
                || keys.insert((&row.key.kind, &row.key.id), row).is_some()
            {
                return Err(Error::Storage);
            }
        }
        let mut receipts = BTreeMap::new();
        let mut current_receipts = BTreeSet::new();
        for receipt in &self.receipts {
            let row = &receipt.row;
            let current = keys
                .get(&(&row.key.kind, &row.key.id))
                .ok_or(Error::Storage)?;
            if receipt.identity.is_empty()
                || row.revision == 0
                || row.revision > current.revision
                || (row.revision == current.revision && row != *current)
                || receipts.insert(&receipt.identity, receipt).is_some()
            {
                return Err(Error::Storage);
            }
            if row.revision == current.revision {
                current_receipts.insert((&row.key.kind, &row.key.id));
            }
        }
        if current_receipts.len() != keys.len() {
            return Err(Error::Storage);
        }
        let mut events = BTreeSet::new();
        for (id, row) in &self.events {
            if !events.insert(id) || &receipts.get(id).ok_or(Error::Storage)?.row != row {
                return Err(Error::Storage);
            }
        }
        let mut effects = BTreeMap::<&str, BTreeSet<u64>>::new();
        for effect in &self.effects {
            if !receipts.contains_key(&effect.identity)
                || !effects
                    .entry(&effect.identity)
                    .or_default()
                    .insert(effect.ordinal)
            {
                return Err(Error::Storage);
            }
        }
        for ordinals in effects.values() {
            if ordinals.iter().copied().ne(0..ordinals.len() as u64) {
                return Err(Error::Storage);
            }
        }
        self.state
            .validate_archive(self.receipts.len(), self.effects.len(), &self.events)
    }
    fn manifest(&self, backend: Backend) -> Manifest {
        Manifest {
            archive_version: 1,
            storage_format: 3,
            backend,
            rows: self.rows.len(),
            receipts: self.receipts.len(),
            events: self.events.len(),
            effects: self.effects.len(),
            work: self.state.work.records().len(),
            external_blobs_included: false,
            external_deliveries_included: false,
        }
    }
}
/// Bounds raw native bytes before decoding and appending to a logical snapshot.
pub struct Collector {
    pub snapshot: Snapshot,
    limits: BackupLimits,
    bytes: usize,
    records: usize,
}
impl Collector {
    pub fn new(state: &str, limits: BackupLimits) -> Result<Self> {
        if state.len() > limits.max_bytes || limits.max_records == 0 {
            return Err(Error::TooLarge);
        }
        Ok(Self {
            snapshot: Snapshot {
                state: decode(state)?,
                rows: vec![],
                receipts: vec![],
                events: vec![],
                effects: vec![],
            },
            limits,
            bytes: state.len(),
            records: 0,
        })
    }
    fn charge(&mut self, data: &str, key_bytes: usize) -> Result<()> {
        self.records = self.records.checked_add(1).ok_or(Error::TooLarge)?;
        self.bytes = self
            .bytes
            .checked_add(data.len())
            .and_then(|n| n.checked_add(key_bytes))
            .ok_or(Error::TooLarge)?;
        if self.records > self.limits.max_records || self.bytes > self.limits.max_bytes {
            return Err(Error::TooLarge);
        }
        Ok(())
    }
    pub fn row(&mut self, kind: &str, id: &str, revision: Option<u64>, data: &str) -> Result<()> {
        self.charge(
            data,
            kind.len().checked_add(id.len()).ok_or(Error::TooLarge)?,
        )?;
        let row: Row = decode(data)?;
        if row.key.kind != kind || row.key.id != id || revision.is_some_and(|r| r != row.revision) {
            return Err(Error::Storage);
        }
        self.snapshot.rows.push(row);
        Ok(())
    }
    pub fn receipt(&mut self, id: &str, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        let receipt: Receipt = decode(data)?;
        if receipt.identity != id {
            return Err(Error::Storage);
        }
        self.snapshot.receipts.push(receipt);
        Ok(())
    }
    pub fn event(&mut self, id: &str, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        self.snapshot.events.push((id.into(), decode(data)?));
        Ok(())
    }
    pub fn effect(&mut self, id: &str, ordinal: u64, data: &str) -> Result<()> {
        self.charge(data, id.len())?;
        self.snapshot.effects.push(StoredEffect {
            identity: id.into(),
            ordinal,
            intent: decode(data)?,
        });
        Ok(())
    }
}
fn decode<T: DeserializeOwned>(data: &str) -> Result<T> {
    serde_json::from_str(data).map_err(|_| Error::Storage)
}
fn encode<T: Serialize>(data: &T, limit: usize) -> Result<Vec<u8>> {
    struct Buffer {
        bytes: Vec<u8>,
        limit: usize,
        full: bool,
    }
    impl Write for Buffer {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(data.len())
                .is_none_or(|n| n > self.limit)
            {
                self.full = true;
                return Err(std::io::Error::other("archive limit"));
            }
            self.bytes.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Buffer {
        bytes: vec![],
        limit,
        full: false,
    };
    if serde_json::to_writer(&mut out, data).is_err() {
        return Err(if out.full {
            Error::TooLarge
        } else {
            Error::Storage
        });
    }
    Ok(out.bytes)
}
const MAGIC: &[u8; 8] = b"ROMBK001";
/// Publish a checked archive without replacing any existing path.
pub fn write(
    path: impl AsRef<Path>,
    backend: Backend,
    snapshot: &Snapshot,
    limits: BackupLimits,
) -> Result<Manifest> {
    snapshot.validate()?;
    let manifest = snapshot.manifest(backend);
    check_count(&manifest, limits)?;
    let header = encode(&manifest, 4096)?;
    let body_limit = limits
        .max_bytes
        .checked_sub(header.len() + 52)
        .ok_or(Error::TooLarge)?;
    let body = encode(snapshot, body_limit)?;
    let total = header
        .len()
        .checked_add(body.len())
        .and_then(|n| n.checked_add(52))
        .ok_or(Error::TooLarge)?;
    if total > limits.max_bytes || header.len() > 4096 {
        return Err(Error::TooLarge);
    }
    let stage = Stage::new(path.as_ref())?;
    let mut file = OpenOptions::new()
        .write(true)
        .open(stage.path())
        .map_err(|_| Error::Storage)?;
    let mut hash = Sha256::new();
    hash.update(&header);
    hash.update(&body);
    for bytes in [
        MAGIC.as_slice(),
        &(header.len() as u32).to_le_bytes(),
        &(body.len() as u64).to_le_bytes(),
        &hash.finalize()[..],
        &header,
        &body,
    ] {
        file.write_all(bytes).map_err(|_| Error::Storage)?;
    }
    file.sync_all().map_err(|_| Error::Storage)?;
    drop(file);
    stage.publish()?;
    Ok(manifest)
}
fn check_count(m: &Manifest, l: BackupLimits) -> Result<()> {
    if [m.rows, m.receipts, m.events, m.effects, m.work]
        .into_iter()
        .try_fold(0usize, |n, v| n.checked_add(v))
        .ok_or(Error::TooLarge)?
        > l.max_records
    {
        return Err(Error::TooLarge);
    }
    Ok(())
}
/// Read a private archive; refuses another backend, corrupt metadata and excessive size.
pub fn read(
    path: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<(Manifest, Snapshot)> {
    let meta = fs::symlink_metadata(path.as_ref()).map_err(|_| Error::Storage)?;
    if !meta.is_file() {
        return Err(Error::Storage);
    }
    check_private(&meta)?;
    if meta.len() > limits.max_bytes as u64 {
        return Err(Error::TooLarge);
    }
    let mut data = vec![];
    File::open(path)
        .map_err(|_| Error::Storage)?
        .take((limits.max_bytes as u64).saturating_add(1))
        .read_to_end(&mut data)
        .map_err(|_| Error::Storage)?;
    if data.len() > limits.max_bytes {
        return Err(Error::TooLarge);
    }
    if data.len() < 52 || &data[..8] != MAGIC {
        return Err(Error::Storage);
    }
    let h = u32::from_le_bytes(data[8..12].try_into().map_err(|_| Error::Storage)?) as usize;
    let b = usize::try_from(u64::from_le_bytes(
        data[12..20].try_into().map_err(|_| Error::Storage)?,
    ))
    .map_err(|_| Error::TooLarge)?;
    if h > 4096 || h.checked_add(b).and_then(|n| n.checked_add(52)) != Some(data.len()) {
        return Err(Error::Storage);
    }
    let bytes = &data[52..];
    if Sha256::digest(bytes)[..] != data[20..52] {
        return Err(Error::Storage);
    }
    let m: Manifest = serde_json::from_slice(&bytes[..h]).map_err(|_| Error::Storage)?;
    if m.backend != backend
        || m.archive_version != 1
        || m.storage_format != 3
        || m.external_blobs_included
        || m.external_deliveries_included
    {
        return Err(Error::Unsupported("backup format or backend".into()));
    }
    check_count(&m, limits)?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?;
    snapshot.validate()?;
    if m != snapshot.manifest(backend) {
        return Err(Error::Storage);
    }
    Ok((m, snapshot))
}
#[cfg(unix)]
fn check_private(meta: &fs::Metadata) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if meta.permissions().mode() & 0o777 != 0o600 {
        return Err(Error::Storage);
    }
    Ok(())
}
#[cfg(not(unix))]
fn check_private(_: &fs::Metadata) -> Result<()> {
    Err(Error::Unsupported("private archives require Unix".into()))
}
/// Private sibling staging directory. Atomic hard-link publication refuses overwrite.
/// The parent directory must be trusted and on a filesystem supporting hard links/fsync.
pub struct Stage {
    directory: PathBuf,
    path: PathBuf,
    destination: PathBuf,
}
impl Stage {
    pub fn new(destination: &Path) -> Result<Self> {
        #[cfg(not(unix))]
        {
            let _ = destination;
            return Err(Error::Unsupported("private archives require Unix".into()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            static SEQ: AtomicU64 = AtomicU64::new(0);
            if fs::symlink_metadata(destination).is_ok() {
                return Err(Error::Conflict);
            }
            let parent = destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            for _ in 0..32 {
                let directory = parent.join(format!(
                    ".rom-maintenance-{}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| Error::Storage)?
                        .as_nanos(),
                    SEQ.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::DirBuilder::new().mode(0o700).create(&directory) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(_) => return Err(Error::Storage),
                }
                let stage = Self {
                    path: directory.join("data"),
                    directory,
                    destination: destination.into(),
                };
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&stage.path)
                    .map_err(|_| Error::Storage)?;
                return Ok(stage);
            }
            Err(Error::Storage)
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn publish(&self) -> Result<()> {
        File::open(&self.path)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Storage)?;
        fs::hard_link(&self.path, &self.destination).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Conflict
            } else {
                Error::Storage
            }
        })?;
        File::open(
            self.destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unknown)
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom::{StorageLimits, json};
    fn unchecked_archive(path: &Path, manifest: &serde_json::Value, snapshot: &serde_json::Value) {
        let header = serde_json::to_vec(manifest).unwrap();
        let body = serde_json::to_vec(snapshot).unwrap();
        let mut hash = Sha256::new();
        hash.update(&header);
        hash.update(&body);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&hash.finalize());
        bytes.extend(header);
        bytes.extend(body);
        fs::write(path, bytes).unwrap();
    }
    #[test]
    fn reader_rejects_valid_checksum_wrong_format_counts_or_metadata() {
        let stage = Stage::new(
            &std::env::temp_dir().join(format!("rom-backup-unit-{}", std::process::id())),
        )
        .unwrap();
        let snapshot = Snapshot {
            state: StorageState::new(StorageLimits::default()).unwrap(),
            rows: vec![],
            receipts: vec![],
            events: vec![],
            effects: vec![],
        };
        let manifest = serde_json::to_value(snapshot.manifest(Backend::Sqlite)).unwrap();
        let body = serde_json::to_value(snapshot).unwrap();
        for field in ["archive_version", "storage_format", "rows"] {
            let mut bad = manifest.clone();
            bad[field] = json!(42);
            unchecked_archive(stage.path(), &bad, &body);
            assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
        }
        for field in ["head", "receipts", "effects"] {
            let mut bad = body.clone();
            bad["state"][field] = json!(42);
            unchecked_archive(stage.path(), &manifest, &bad);
            assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
        }
        let mut bad = body.clone();
        bad["state"]["work"]["roots"] = json!({"orphan":1});
        unchecked_archive(stage.path(), &manifest, &bad);
        assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_err());
        unchecked_archive(stage.path(), &manifest, &body);
        assert!(read(stage.path(), Backend::Sqlite, BackupLimits::default()).is_ok());
    }
    #[test]
    fn publication_race_and_symlink_refuse_overwrite() {
        let destination =
            std::env::temp_dir().join(format!("rom-backup-race-{}", std::process::id()));
        let stage = Stage::new(&destination).unwrap();
        fs::write(stage.path(), b"new").unwrap();
        fs::write(&destination, b"old").unwrap();
        assert_eq!(stage.publish(), Err(Error::Conflict));
        assert_eq!(fs::read(&destination).unwrap(), b"old");
        fs::remove_file(&destination).unwrap();
        std::os::unix::fs::symlink(stage.path(), &destination).unwrap();
        assert!(matches!(
            read(&destination, Backend::Sqlite, BackupLimits::default()),
            Err(Error::Storage)
        ));
        assert!(matches!(Stage::new(&destination), Err(Error::Conflict)));
        fs::remove_file(destination).unwrap();
    }
}
