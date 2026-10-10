//! Explicit format upgrade. The source is never modified.
use crate::{Backend, BackupLimits, Manifest, Snapshot, StoredEffect, archive, write};
use rom::{Descriptor, Error, Receipt, Result, Row};
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
    state: serde_json::Value,
    rows: Vec<Row>,
    receipts: Vec<Receipt>,
    events: Vec<(String, Row)>,
    effects: Vec<StoredEffect>,
}

/// Upgrade an archive-1/storage-3 backup into a fresh current archive.
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
        operator_receipts: 0,
        descriptors: 0,
        references: 0,
        external_blobs_included: false,
        external_deliveries_included: false,
    };
    archive::check_count(&manifest, limits)?;
    let original_value: serde_json::Value =
        serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?;
    crate::legacy_state::reject_snapshot_scheduling_fields(&original_value)?;
    let original: LegacySnapshot =
        serde_json::from_value(original_value).map_err(|_| Error::Storage)?;
    let snapshot = Snapshot {
        state: crate::decode_legacy_storage_state(original.state)?,
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
    validate_legacy_retry_epochs(&snapshot)?;
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
    upgrade_catalogued_archive(source.as_ref(), destination.as_ref(), backend, limits, 2, 4)
}

/// Upgrade an archive-3/storage-5 backup to the epoch-aware archive format.
/// Existing receipt origins are retained; absent retry epochs start at zero.
pub fn upgrade_v3_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<Manifest> {
    upgrade_catalogued_archive(source.as_ref(), destination.as_ref(), backend, limits, 3, 5)
}

/// Upgrade an archive-4/storage-6 backup into a fresh current archive.
/// Preserve retry boundaries, work epochs and receipt origins exactly as stored.
pub fn upgrade_v4_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<Manifest> {
    upgrade_catalogued_archive(source.as_ref(), destination.as_ref(), backend, limits, 4, 6)
}

/// Upgrade an archive-5/storage-7 backup with explicit initial recovery metadata.
/// Preserve epochs, receipt origins, work budgets and at-least-once delivery semantics.
pub fn upgrade_v5_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<Manifest> {
    upgrade_catalogued_archive(source.as_ref(), destination.as_ref(), backend, limits, 5, 7)
}

/// Upgrade archive-6/storage-8 while retaining operator receipts and delivery profiles.
/// Scheduling metadata is absent in the predecessor and cannot be injected into it.
pub fn upgrade_v6_archive(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    backend: Backend,
    limits: BackupLimits,
) -> Result<Manifest> {
    upgrade_catalogued_archive(source.as_ref(), destination.as_ref(), backend, limits, 6, 8)
}

fn upgrade_catalogued_archive(
    source: &Path,
    destination: &Path,
    backend: Backend,
    limits: BackupLimits,
    archive_version: u32,
    storage_format: u32,
) -> Result<Manifest> {
    let (_, mut snapshot) =
        archive::read_version(source, backend, limits, archive_version, storage_format)?;
    if storage_format < 6 {
        validate_legacy_retry_epochs(&snapshot)?;
        bind_receipt_origins(&mut snapshot)?;
    }
    write(destination, backend, &snapshot, limits)
}

/// Upgrade a native legacy snapshot with an exact, explicit source catalog.
/// A catalogued source must match the supplied descriptors; no schema changes occur.
pub fn upgrade_legacy_snapshot(
    snapshot: Snapshot,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Snapshot> {
    validate_legacy_retry_epochs(&snapshot)?;
    let mut snapshot = upgrade_current_snapshot(snapshot, descriptors, limits)?;
    bind_receipt_origins(&mut snapshot)?;
    crate::maintenance_limits::check_snapshot(&snapshot, limits)?;
    Ok(snapshot)
}

/// Validate an epoch-aware native snapshot against its exact canonical source catalog.
/// Epochs, receipt origins and logical records remain unchanged. This does not bind
/// an absent catalog or transform schemas; native publication rebuilds derived data.
pub fn upgrade_current_snapshot(
    snapshot: Snapshot,
    descriptors: &[Descriptor],
    limits: BackupLimits,
) -> Result<Snapshot> {
    crate::maintenance_limits::check_snapshot(&snapshot, limits)?;
    snapshot.validate()?;
    if rom::validate_descriptors(descriptors)? != rom::validate_descriptors(&snapshot.descriptors)?
    {
        return Err(Error::Unsupported(
            "upgrade source descriptor mismatch".into(),
        ));
    }
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

/// Legacy formats predate retry epochs. Nonzero metadata cannot be interpreted safely.
/// Zero fields are accepted so default serialization remains backward-compatible.
pub fn validate_legacy_retry_epochs(snapshot: &Snapshot) -> Result<()> {
    if snapshot.state.retry_epochs() != rom::RetryEpochs::default()
        || snapshot
            .receipts
            .iter()
            .any(|receipt| receipt.retry_epoch != 0)
        || snapshot.state.work.records().iter().any(|record| {
            record.pending.cause.retry_epoch != 0
                || matches!(&record.pending.payload, rom::WorkPayload::Action(value)
                    if value.get("retry_epoch").is_some_and(|epoch| epoch.as_u64() != Some(0)))
        })
    {
        return Err(Error::Unsupported(
            "legacy storage cannot contain retry epochs".into(),
        ));
    }
    Ok(())
}
