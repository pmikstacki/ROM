//! SQLite persistence and scalar queries. The deployment database remains host-configured.
mod index;
mod index_rebuild;
mod maintenance;
mod migration;
mod native_journal;
mod native_work;
mod operator;
mod persistence;
mod query_observation;
mod read_rows;
mod references;
mod retention;
mod snapshot;
mod store;
mod upgrade;

#[cfg(feature = "test-support")]
pub use query_observation::{QueryExecution, QueryMetrics, QueryObservation};
pub use store::Sqlite;

#[cfg(test)]
mod native_journal_tests;
#[cfg(test)]
mod native_work_batch_tests;
#[cfg(test)]
mod native_work_test_support;
#[cfg(test)]
mod native_work_tests;

#[cfg(feature = "test-support")]
mod stage_observation;
#[cfg(feature = "test-support")]
pub use stage_observation::{
    ClaimPrefixDisposition, ClaimPrefixMeasurement, ClaimPrefixReason, ClaimPrefixSnapshot,
    PublicationCategory, PublicationMeasurement, PublicationSnapshot, SemanticEffect,
    StageMeasurement, StageObservation, StageOperation, StageSnapshot, StorageStage,
    WorkCommitMeasurement, WorkCommitOrigin, WorkUtilizationSnapshot,
};
#[cfg(all(test, feature = "test-support"))]
mod stage_observation_tests;

#[cfg(all(test, feature = "test-support"))]
mod publication_observation_tests;

#[cfg(test)]
mod native_journal_maintenance_tests;

#[cfg(test)]
mod native_journal_activation_tests;

#[cfg(test)]
mod cached_read_tests;

#[cfg(all(test, feature = "test-support"))]
mod native_work_idle_tests;

#[cfg(all(test, feature = "test-support"))]
mod native_work_sync_tests;

#[cfg(all(test, feature = "test-support"))]
mod native_work_utilization_tests;

#[cfg(all(test, feature = "test-support"))]
mod native_claim_prefix_observation_tests;

#[cfg(feature = "test-support")]
mod linked_engine;
#[cfg(feature = "test-support")]
pub use linked_engine::LinkedEngineIdentity;
