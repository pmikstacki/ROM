//! SQLite persistence and scalar queries. The deployment database remains host-configured.
mod index;
mod index_rebuild;
mod maintenance;
mod migration;
mod persistence;
mod references;
mod retention;
mod snapshot;
mod store;
mod upgrade;

pub use store::Sqlite;
