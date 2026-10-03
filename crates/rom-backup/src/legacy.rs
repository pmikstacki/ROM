//! Explicit format upgrade. The source is never modified.
use crate::{Backend, BackupLimits, Manifest, Snapshot, StoredEffect, archive, write};
use rom::{Descriptor, Error, Receipt, Result, Row, StorageState};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyManifest {
    archive_version: u32,
    storage_format: u32,
    backend: Backend,
    rows: usize,
    receipts: usize,
    events: usize,
    effects: usize,
    work: usize,
    external_blobs_included: bool,
    external_deliveries_included: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySnapshot {
    state: StorageState,
    rows: Vec<Row>,
    receipts: Vec<Receipt>,
    events: Vec<(String, Row)>,
    effects: Vec<StoredEffect>,
}

/// Upgrade an archive-1/storage-3 backup into a new archive-3/storage-5 path.
/// Supply explicit descriptors for every stored kind. No row transformations occur.
/// Missing targets, incompatible values and exceeded limits prevent publication.
/// Receipt identities and pending work remain unchanged. Restore fences active claims.
pub fn upgrade_v1_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Manifest> {
    let (data, h) = archive::read_envelope(source.as_ref(), limits)?;
    let bytes = &data[52..];
    let old: LegacyManifest = serde_json::from_slice(&bytes[..h]).map_err(|_| Error::Storage)?;
    if old.backend != backend
        || old.archive_version != 1
        || old.storage_format != 3
        || old.external_blobs_included
        || old.external_deliveries_included
    {
        return Err(Error::Unsupported("legacy backup format or backend".into()));
    }
    let manifest = Manifest {
        archive_version: crate::model::ARCHIVE_VERSION,
        storage_format: crate::model::STORAGE_FORMAT,
        backend,
        rows: old.rows,
        receipts: old.receipts,
        events: old.events,
        effects: old.effects,
        work: old.work,
        descriptors: 0,
        references: 0,
        external_blobs_included: false,
        external_deliveries_included: false,
    };
    archive::check_count(&manifest, limits)?;
    let original: LegacySnapshot =
        serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?;
    let snapshot = Snapshot {
        state: original.state,
        rows: original.rows,
        receipts: original.receipts,
        events: original.events,
        effects: original.effects,
        descriptors: Vec::new(),
        references: Vec::new(),
    };
    if snapshot.manifest(backend) != manifest {
        return Err(Error::Storage);
    }
    let snapshot = bind_legacy_schema(snapshot, descriptors, limits)?;
    write(destination, backend, &snapshot, limits)
}

/// Bind an explicit schema to a collected format-3 snapshot for adapter upgrades.
/// Existing schema/index metadata is rejected. Values and historical records remain unchanged.
/// The result is validated but not published; native restore supplies generation fencing.
pub fn bind_legacy_schema(
    mut snapshot: Snapshot,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Snapshot> {
    if !snapshot.descriptors.is_empty() || !snapshot.references.is_empty() {
        return Err(Error::Unsupported(
            "legacy snapshot already has schema metadata".into(),
        ));
    }
    snapshot.descriptors = rom::validate_descriptors(descriptors)?;
    crate::schema::rebuild_references(&mut snapshot, limits)?;
    bind_receipt_origins(&mut snapshot)?;
    snapshot.validate()?;
    // The native output gains descriptors and edges. Bound that complete payload too.
    crate::codec::encode(&snapshot, limits.max_bytes)?;
    Ok(snapshot)
}

/// Upgrade an archive-2/storage-4 backup into a fresh current archive.
/// Retain the stored catalog and all record values. Bind missing receipt origins to that catalog.
pub fn upgrade_v2_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<Manifest> {
    let (_, mut snapshot) = archive::read_version(source.as_ref(), backend, limits, 2, 4)?;
    bind_receipt_origins(&mut snapshot)?;
    write(destination, backend, &snapshot, limits)
}

/// Upgrade a native legacy snapshot with an exact, explicit source catalog.
/// A catalogued source must match the supplied descriptors; no schema changes occur.
pub fn upgrade_legacy_snapshot(
    mut snapshot: Snapshot,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Snapshot> {
    snapshot.validate()?;
    if rom::validate_descriptors(descriptors)? != snapshot.descriptors {
        return Err(Error::Unsupported(
            "upgrade source descriptor mismatch".into(),
        ));
    }
    bind_receipt_origins(&mut snapshot)?;
    snapshot.validate()?;
    archive::check_count(&snapshot.manifest(Backend::Sqlite), limits)?;
    crate::codec::encode(&snapshot, limits.max_bytes)?;
    Ok(snapshot)
}

fn bind_receipt_origins(snapshot: &mut Snapshot) -> Result<()> {
    let versions: std::collections::BTreeMap<_, _> = snapshot
        .descriptors
        .iter()
        .map(|descriptor| (descriptor.kind.as_str(), descriptor.version))
        .collect();
    for receipt in &mut snapshot.receipts {
        if receipt.replay_version.is_none() {
            receipt.replay_version = Some(
                *versions
                    .get(receipt.row.key.kind.as_str())
                    .ok_or(Error::Storage)?,
            );
        }
    }
    Ok(())
}
