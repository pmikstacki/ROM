//! Concrete mechanics for the throwaway persistence probe.
mod common;
mod kv;
mod sqlite;
pub use kv::RedbStore;
pub use sqlite::SqliteStore;
#[cfg(feature = "fault-injection")]
pub mod probe;
