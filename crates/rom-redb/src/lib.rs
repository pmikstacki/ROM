//! redb persistence adapter with atomic Resource/event/receipt/effect bundles.
//!
//! The host must run these synchronous methods on its bounded storage executor.
//! The current format stores JSON ROM values, using tuple keys for kind/id isolation.
//! A commit error is uncertain; discard the adapter and reopen before recovery.
mod admission;
mod commit;
mod format;
mod maintenance;
mod migration;
mod native_journal;
#[cfg(test)]
mod native_journal_tests;
mod native_records;
mod native_state;
mod native_work;
#[cfg(test)]
mod native_work_batch_tests;
#[cfg(test)]
mod native_work_tests;
mod operator;
mod preflight;
mod references;
mod retention;
mod storage;
mod store;
mod upgrade;
#[cfg(test)]
mod upgrade_tests;

pub use store::Redb;
