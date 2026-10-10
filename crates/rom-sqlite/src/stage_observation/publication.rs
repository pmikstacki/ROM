//! Fixed numeric counts of successful SQL publications, including later rollback.
use super::{StageObservation, StageOperation};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationCategory {
    Metadata,
    EventPayload,
    WorkHeader,
    WorkRecord,
    WorkRoot,
}
impl PublicationCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::EventPayload => "event_payload",
            Self::WorkHeader => "work_header",
            Self::WorkRecord => "work_record",
            Self::WorkRoot => "work_root",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationMeasurement {
    pub operation: StageOperation,
    pub category: PublicationCategory,
    /// Successful native SQL statements, not durable transactions.
    pub samples: u64,
    /// Exact serialized value bytes passed to those statements; excludes keys/indexes.
    pub encoded_bytes: u64,
    pub dropped_samples: u64,
    pub saturated: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationSnapshot {
    pub entries: [PublicationMeasurement; 10],
    pub poison_recovered: bool,
}
pub(super) fn empty() -> [PublicationMeasurement; 10] {
    let categories = [
        PublicationCategory::Metadata,
        PublicationCategory::EventPayload,
        PublicationCategory::WorkHeader,
        PublicationCategory::WorkRecord,
        PublicationCategory::WorkRoot,
    ];
    std::array::from_fn(|i| PublicationMeasurement {
        operation: if i < 5 {
            StageOperation::Commit
        } else {
            StageOperation::WorkUpdate
        },
        category: categories[i % 5],
        samples: 0,
        encoded_bytes: 0,
        dropped_samples: 0,
        saturated: false,
    })
}
impl StageObservation {
    pub(crate) fn record_publication(
        &self,
        operation: StageOperation,
        category: PublicationCategory,
        bytes: usize,
    ) {
        let index = match operation {
            StageOperation::Commit => category as usize,
            StageOperation::WorkUpdate => 5 + category as usize,
            StageOperation::RetryEpochs => return,
        };
        let mut entries = self.inner.publications.lock().unwrap_or_else(|poisoned| {
            self.inner.poison_recovered.store(true, Ordering::Relaxed);
            poisoned.into_inner()
        });
        let entry = &mut entries[index];
        let update = u64::try_from(bytes).ok().and_then(|bytes| {
            Some((
                entry.samples.checked_add(1)?,
                entry.encoded_bytes.checked_add(bytes)?,
            ))
        });
        if let Some((samples, encoded_bytes)) = update {
            entry.samples = samples;
            entry.encoded_bytes = encoded_bytes;
        } else {
            entry.saturated = true;
            entry.dropped_samples = entry.dropped_samples.saturating_add(1);
        }
    }
    /// Numeric statement-level publications. Obtain final values after operations quiesce.
    pub fn publication_snapshot(&self) -> PublicationSnapshot {
        let entries = self.inner.publications.lock().unwrap_or_else(|poisoned| {
            self.inner.poison_recovered.store(true, Ordering::Relaxed);
            poisoned.into_inner()
        });
        PublicationSnapshot {
            entries: *entries,
            poison_recovered: self.inner.poison_recovered.load(Ordering::Relaxed),
        }
    }
}
