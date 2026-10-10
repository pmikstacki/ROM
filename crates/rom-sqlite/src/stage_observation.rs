//! Opt-in, fixed-cardinality wall-clock observations for native test-support probes.
mod claim_prefix;
mod publication;
pub(crate) use claim_prefix::Guard as ClaimPrefixGuard;
pub use claim_prefix::{
    ClaimPrefixDisposition, ClaimPrefixMeasurement, ClaimPrefixReason, ClaimPrefixSnapshot,
};
mod timing;
mod work_utilization;
pub use publication::{PublicationCategory, PublicationMeasurement, PublicationSnapshot};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
pub(crate) use timing::Timer;
pub(crate) use work_utilization::Context as WorkCommitContext;
pub use work_utilization::{
    SemanticEffect, WorkCommitMeasurement, WorkCommitOrigin, WorkUtilizationSnapshot,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageOperation {
    Commit,
    WorkUpdate,
    RetryEpochs,
}
impl StageOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::WorkUpdate => "work_update",
            Self::RetryEpochs => "retry_epochs",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageStage {
    ConnectionLock,
    TransactionBegin,
    MetadataRead,
    SharedPrepare,
    NativePublication,
    NativeCommit,
}
impl StorageStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionLock => "connection_lock",
            Self::TransactionBegin => "transaction_begin",
            Self::MetadataRead => "metadata_read",
            Self::SharedPrepare => "shared_prepare",
            Self::NativePublication => "native_publication",
            Self::NativeCommit => "native_commit",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageMeasurement {
    pub operation: StageOperation,
    pub stage: StorageStage,
    /// Finished stage samples, including early errors and unwinding.
    pub samples: u64,
    /// Inclusive wall-clock duration of the named stage, not exclusive CPU time.
    pub elapsed_ns: u64,
    pub dropped_samples: u64,
    pub saturated: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageSnapshot {
    pub entries: [StageMeasurement; 18],
    pub poison_recovered: bool,
}
#[derive(Clone)]
pub struct StageObservation {
    inner: Arc<Aggregate>,
}
struct Aggregate {
    entries: Mutex<[StageMeasurement; 18]>,
    publications: Mutex<[PublicationMeasurement; 10]>,
    work_utilization: Mutex<work_utilization::Aggregate>,
    claim_prefix: Mutex<claim_prefix::Aggregate>,
    poison_recovered: AtomicBool,
}
impl Default for StageObservation {
    fn default() -> Self {
        let operations = [
            StageOperation::Commit,
            StageOperation::WorkUpdate,
            StageOperation::RetryEpochs,
        ];
        let stages = [
            StorageStage::ConnectionLock,
            StorageStage::TransactionBegin,
            StorageStage::MetadataRead,
            StorageStage::SharedPrepare,
            StorageStage::NativePublication,
            StorageStage::NativeCommit,
        ];
        Self {
            inner: Arc::new(Aggregate {
                entries: Mutex::new(std::array::from_fn(|index| StageMeasurement {
                    operation: operations[index / 6],
                    stage: stages[index % 6],
                    samples: 0,
                    elapsed_ns: 0,
                    dropped_samples: 0,
                    saturated: false,
                })),
                publications: Mutex::new(publication::empty()),
                work_utilization: Mutex::new(work_utilization::Aggregate::default()),
                claim_prefix: Mutex::new(claim_prefix::Aggregate::default()),
                poison_recovered: AtomicBool::new(false),
            }),
        }
    }
}
impl StageObservation {
    fn record(&self, operation: StageOperation, stage: StorageStage, elapsed: std::time::Duration) {
        let mut entries = self.inner.entries.lock().unwrap_or_else(|poisoned| {
            self.inner.poison_recovered.store(true, Ordering::Relaxed);
            poisoned.into_inner()
        });
        let entry = &mut entries[operation as usize * 6 + stage as usize];
        let update = u64::try_from(elapsed.as_nanos()).ok().and_then(|elapsed| {
            Some((
                entry.samples.checked_add(1)?,
                entry.elapsed_ns.checked_add(elapsed)?,
            ))
        });
        if let Some((samples, elapsed_ns)) = update {
            entry.samples = samples;
            entry.elapsed_ns = elapsed_ns;
        } else {
            entry.saturated = true;
            entry.dropped_samples = entry.dropped_samples.saturating_add(1);
        }
    }
    /// Copy the fixed aggregate. Obtain final measurements after operations quiesce.
    pub fn snapshot(&self) -> StageSnapshot {
        let entries = self.inner.entries.lock().unwrap_or_else(|poisoned| {
            self.inner.poison_recovered.store(true, Ordering::Relaxed);
            poisoned.into_inner()
        });
        StageSnapshot {
            entries: *entries,
            poison_recovered: self.inner.poison_recovered.load(Ordering::Relaxed),
        }
    }
}
impl crate::Sqlite {
    /// Enable scoped native diagnostics. No user callback or sensitive values are retained.
    pub fn observe_stages(&self) -> StageObservation {
        self.stage_observation
            .get_or_init(StageObservation::default)
            .clone()
    }
}

#[cfg(test)]
mod counter_tests;

#[cfg(test)]
mod publication_counter_tests;
