//! redb persistence adapter with atomic Resource/event/receipt/effect bundles.
//!
//! The host must run these synchronous methods on its bounded storage executor.
//! Format version four stores JSON ROM values, using tuple keys for kind/id isolation.
//! A commit error is uncertain; discard the adapter and reopen before recovery.
mod commit;
mod format;
mod maintenance;
mod preflight;
mod references;
mod storage;
mod store;
mod upgrade;

pub use store::Redb;
