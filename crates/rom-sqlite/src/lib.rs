//! SQLite reference persistence capability. The deployment database remains host-configured.
mod maintenance;
mod persistence;
mod references;
mod store;

pub use store::Sqlite;
