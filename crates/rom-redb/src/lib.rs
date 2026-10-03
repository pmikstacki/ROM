//! redb persistence adapter with atomic Resource/event/receipt/effect bundles.
//!
//! The host must run these synchronous methods on its bounded storage executor.
//! The current format stores JSON ROM values, using tuple keys for kind/id isolation.
//! A commit error is uncertain; discard the adapter and reopen before recovery.
mod commit;
mod format;
mod maintenance;
mod migration;
mod operator;
mod preflight;
mod references;
mod retention;
mod state;
mod storage;
mod store;
mod upgrade;
#[cfg(test)]
mod upgrade_tests;

pub use store::Redb;
