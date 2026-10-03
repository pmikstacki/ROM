use crate::{Backend, BackupLimits, Snapshot, archive, codec};
use rom::Result;

pub(crate) fn check_snapshot(snapshot: &Snapshot, limits: BackupLimits) -> Result<()> {
    archive::check_count(&snapshot.manifest(Backend::Sqlite), limits)?;
    codec::encode(snapshot, limits.max_bytes)?;
    Ok(())
}
