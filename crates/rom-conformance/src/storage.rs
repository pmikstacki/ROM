use std::sync::Arc;
mod assertions;
mod runner;
mod scenario;
mod typed;
pub use assertions::assert_bundle;
pub use runner::{basic, for_profile};
/// Trusted test inspection facts; counts are rows, events, receipts, effects.
#[derive(Clone, PartialEq, Eq)]
pub struct StorageFacts {
    pub counts: [u64; 4],
    pub events: Vec<rom::Row>,
    pub effects: Vec<(String, rom::Intent)>,
}
/// Own a fresh exclusive backing store, and release its handle before reopen.
pub trait StorageFixture: Send {
    fn storage(&self) -> Arc<dyn rom::Storage>;
    fn facts(&self) -> rom::Result<StorageFacts>;
    fn reopen(&mut self) -> rom::Result<()>;
}
