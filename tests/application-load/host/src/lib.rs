//! Bounded mechanisms and registration for the isolated application load fixture.
pub mod application;
mod authentication_evidence;
#[cfg(test)]
mod authentication_evidence_tests;
#[path = "../../../application-recovery/host/src/configuration.rs"]
pub mod configuration;
mod core_overload_evidence;
#[cfg(test)]
mod core_overload_evidence_tests;
#[path = "../../../application-recovery/host/src/fixture_identity.rs"]
pub mod fixture_identity;
#[path = "../../../application-recovery/host/src/host_configuration.rs"]
pub mod host_configuration;
mod observed_storage;
mod work_batch_observation;
pub use observed_storage::ObservedStorage;
mod queue_observation;
pub use queue_observation::QueueObservation;
#[cfg(test)]
mod application_tests;
#[cfg(feature = "storage-stage-timings")]
mod claim_prefix_terminal_proof;
#[path = "../../../application-recovery/host/src/database.rs"]
pub mod database;
#[cfg(feature = "storage-stage-timings")]
mod publication_measurements;
#[cfg(test)]
mod read_policy_tests;
pub mod runner;
mod seed_measurements;
#[cfg(feature = "storage-stage-timings")]
mod stage_measurements;
pub mod storage_calls;
#[cfg(feature = "storage-stage-timings")]
mod work_terminal_proof;

pub mod inspection;

#[cfg(feature = "storage-stage-timings")]
mod linked_engine_diagnostic;
