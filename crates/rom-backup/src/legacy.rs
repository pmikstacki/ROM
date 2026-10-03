//! Explicit format upgrade. The source is never modified.
use crate::{Backend, BackupLimits, Manifest, Snapshot, StoredEffect, archive, write};
use rom::{Descriptor, Error, Receipt, ReferenceEdge, Result, Row, StorageState};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

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

/// Upgrade an archive-1/storage-3 backup into a new archive-2/storage-4 path.
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
    let mut manifest = Manifest {
        archive_version: 2,
        storage_format: 4,
        backend,
        rows: old.rows,
        receipts: old.receipts,
        events: old.events,
        effects: old.effects,
        work: old.work,
        descriptors: descriptors.len(),
        references: 0,
        external_blobs_included: false,
        external_deliveries_included: false,
    };
    archive::check_count(&manifest, limits)?;
    let descriptors = rom::validate_descriptors(descriptors)?;
    let original: LegacySnapshot =
        serde_json::from_slice(&bytes[h..]).map_err(|_| Error::Storage)?;
    let mut snapshot = Snapshot {
        state: original.state,
        rows: original.rows,
        receipts: original.receipts,
        events: original.events,
        effects: original.effects,
        descriptors,
        references: Vec::new(),
    };
    if snapshot.manifest(backend) != manifest {
        return Err(Error::Storage);
    }
    let catalog: BTreeMap<_, _> = snapshot
        .descriptors
        .iter()
        .map(|d| (d.kind.as_str(), d))
        .collect();
    for row in &snapshot.rows {
        let descriptor = catalog
            .get(row.key.kind.as_str())
            .ok_or(Error::Unregistered)?;
        for target in descriptor.reference_targets(row.value.as_ref())? {
            manifest.references = manifest.references.checked_add(1).ok_or(Error::TooLarge)?;
            archive::check_count(&manifest, limits)?;
            snapshot.references.push(ReferenceEdge {
                source: row.key.clone(),
                target,
            });
        }
    }
    write(destination, backend, &snapshot, limits)
}
