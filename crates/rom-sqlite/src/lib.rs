//! SQLite persistence and scalar queries. The deployment database remains host-configured.
mod index;
mod index_rebuild;
mod maintenance;
mod migration;
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
