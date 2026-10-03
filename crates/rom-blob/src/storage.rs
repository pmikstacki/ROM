//! Provider-neutral storage contract.
use crate::{ObjectKey, Result};
use std::{future::Future, pin::Pin};

pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub type Upload = Pin<Box<dyn futures_util::Stream<Item = Result<Vec<u8>>> + Send>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub bytes: u64,
}
/// Trusted storage port. A successful create publishes one complete object;
/// existing keys return Conflict without overwrite. Unknown must not mean rollback.
pub trait BlobStore: Send + Sync + 'static {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()>;
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>>;
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata>;
    /// Trusted maintenance only: caller must prove detachment, grace and quiescence.
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()>;
}
