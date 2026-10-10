//! One arbitration surface for current keyed and predecessor canonical journals.
use crate::native_work::Reader;
use rom::storage_support::metadata::{
    JournalDelta, MetadataHeader, StorageMetadata, prepare_bundle, prepare_native_bundle,
};
use rom::storage_support::work::{WorkDelta, WorkRead};
use rom::{Bundle, ClaimKey, Result};

pub(crate) enum Metadata {
    Canonical(StorageMetadata),
    Keyed(MetadataHeader),
}
pub(crate) struct Prepared {
    pub(crate) work: WorkDelta,
    pub(crate) journal: Option<JournalDelta>,
    pub(crate) metadata: Option<StorageMetadata>,
    pub(crate) retired: Vec<String>,
    pub(crate) encoded_header: Option<String>,
}
impl Metadata {
    pub(crate) fn load(reader: &Reader<'_>, candidate: bool) -> Result<Self> {
        if candidate {
            Ok(Self::Keyed(
                rom::storage_support::metadata::JournalRead::header(reader)?,
            ))
        } else {
            Ok(Self::Canonical(crate::native_work::metadata(
                reader.connection,
            )?))
        }
    }
    pub(crate) fn check_retry_epoch(
        &self,
        epoch: u64,
        replay: bool,
        completed: Option<(&ClaimKey, u64)>,
        work: &impl WorkRead,
    ) -> Result<()> {
        match self {
            Self::Canonical(m) => m.check_retry_epoch(epoch, replay, completed, work),
            Self::Keyed(m) => m.check_retry_epoch(epoch, replay, completed, work),
        }
    }
    pub(crate) fn prepare(&self, reader: &Reader<'_>, b: &Bundle) -> Result<Prepared> {
        match self {
            Self::Canonical(m) => {
                let (metadata, work, retired) = prepare_bundle(m, reader, b)?.into_parts();
                work.validate(reader)?;
                Ok(Prepared {
                    work,
                    journal: None,
                    metadata: Some(metadata),
                    retired,
                    encoded_header: None,
                })
            }
            Self::Keyed(_) => {
                reader.require_absent_event(&b.receipt.identity)?;
                let delta = prepare_native_bundle(reader, reader, b)?;
                delta.validate(reader, reader)?;
                let (journal, work) = delta.into_parts();
                let retired = journal
                    .retired()
                    .iter()
                    .map(|e| e.event.identity.clone())
                    .collect();
                let encoded_header = Some(
                    serde_json::to_string(&journal.header().parts())
                        .map_err(|_| rom::Error::Storage)?,
                );
                Ok(Prepared {
                    work,
                    journal: Some(journal),
                    metadata: None,
                    retired,
                    encoded_header,
                })
            }
        }
    }
}
