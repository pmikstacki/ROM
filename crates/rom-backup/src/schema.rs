//! Bounded reconstruction of live reference metadata for maintenance candidates.
use crate::{Backend, BackupLimits, Snapshot, archive};
use rom::{Error, ReferenceEdge, Result};
use std::collections::BTreeMap;

pub(crate) fn rebuild_references(snapshot: &mut Snapshot, limits: BackupLimits) -> Result<()> {
    snapshot.descriptors = rom::validate_descriptors(&snapshot.descriptors)?;
    snapshot.references.clear();
    let mut manifest = snapshot.manifest(Backend::Sqlite);
    archive::check_count(&manifest, limits)?;
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
    Ok(())
}
