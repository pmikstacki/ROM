//! Optional byte observations reuse the exact encoded strings of native statements.
use super::{Reader, write::save_metadata_raw};
use crate::{PublicationCategory, StageObservation, StageOperation};
use rom::{Error, Result, storage_support::metadata::StorageMetadata};
use rusqlite::Connection;

pub(crate) fn save_metadata_observed(
    connection: &Connection,
    metadata: &StorageMetadata,
    observer: Option<&StageObservation>,
) -> Result<()> {
    let raw = serde_json::to_string(metadata).map_err(|_| Error::Storage)?;
    save_metadata_raw(connection, &raw)?;
    if let Some(observer) = observer {
        observer.record_publication(
            StageOperation::Commit,
            PublicationCategory::Metadata,
            raw.len(),
        );
    }
    Ok(())
}
impl<'tx> Reader<'tx> {
    pub(crate) fn observe_publications(
        &mut self,
        observer: Option<&'tx StageObservation>,
        operation: StageOperation,
    ) {
        self.publication = observer.map(|observer| (observer, operation));
    }
    pub(crate) fn record_publication(&self, category: PublicationCategory, bytes: usize) {
        if let Some((observer, operation)) = self.publication {
            observer.record_publication(operation, category, bytes);
        }
    }
}
