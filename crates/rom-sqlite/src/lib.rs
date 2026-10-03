//! SQLite reference persistence capability. The deployment database remains host-configured.
mod maintenance;
mod migration;
mod persistence;
mod references;
mod snapshot;
mod store;
mod upgrade;

pub use store::Sqlite;
