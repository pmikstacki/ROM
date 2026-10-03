use crate::codec::encode;
use crate::{Backend, BackupLimits, Manifest, Snapshot, Stage};
use rom::{Error, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub(crate) const MAGIC: &[u8; 8] = b"ROMBK001";
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
pub(crate) fn check_count(m: &Manifest, l: BackupLimits) -> Result<()> {
    if [
        m.rows,
        m.receipts,
        m.events,
        m.effects,
        m.work,
        m.operator_receipts,
        m.descriptors,
        m.references,
    ]
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
    read_version(
        path.as_ref(),
        backend,
        limits,
        crate::model::ARCHIVE_VERSION,
        crate::model::STORAGE_FORMAT,
    )
}

pub(crate) fn read_version(
    path: &Path,
    backend: Backend,
    limits: BackupLimits,
    archive_version: u32,
    storage_format: u32,
) -> Result<(Manifest, Snapshot)> {
    let (data, h) = read_envelope(path, limits)?;
    let bytes = &data[52..];
    let mut header: serde_json::Value =
        serde_json::from_slice(&bytes[..h]).map_err(|_| Error::Storage)?;
    if header
        .get("archive_version")
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(archive_version))
        || header
            .get("storage_format")
            .and_then(serde_json::Value::as_u64)
            != Some(u64::from(storage_format))
    {
        return Err(Error::Unsupported("backup format or backend".into()));
    }
    if archive_version < crate::model::ARCHIVE_VERSION {
        let header = header.as_object_mut().ok_or(Error::Storage)?;
        if header.contains_key("operator_receipts") {
            return Err(Error::Storage);
        }
        header.insert("operator_receipts".into(), serde_json::json!(0));
    }
    let m: Manifest = serde_json::from_value(header).map_err(|_| Error::Storage)?;
    if m.backend != backend || m.external_blobs_included || m.external_deliveries_included {
        return Err(Error::Unsupported("backup format or backend".into()));
    }
    check_count(&m, limits)?;
    let snapshot: Snapshot = if archive_version < crate::model::ARCHIVE_VERSION {
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?;
        crate::legacy_state::upgrade_state(value.get_mut("state").ok_or(Error::Storage)?)?;
        serde_json::from_value(value).map_err(|_| Error::Storage)?
    } else {
        serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?
    };
    snapshot.validate()?;
    let mut expected = snapshot.manifest(backend);
    expected.archive_version = archive_version;
    expected.storage_format = storage_format;
    if m != expected {
        return Err(Error::Storage);
    }
    Ok((m, snapshot))
}

pub(crate) fn read_envelope(path: &Path, limits: BackupLimits) -> Result<(Vec<u8>, usize)> {
    let meta = fs::symlink_metadata(path).map_err(|_| Error::Storage)?;
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
    Ok((data, h))
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
