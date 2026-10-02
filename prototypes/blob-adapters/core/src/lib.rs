//! Small provider-independent blob contract for an executable experiment.
use std::{future::Future, pin::Pin};

pub type FutureResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;
pub type Upload = Pin<Box<dyn futures_core::Stream<Item = Result<Vec<u8>, Error>> + Send>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidKey,
    LimitExceeded,
    InputFailed,
    UnsupportedCondition,
    NotFound,
    Denied,
    Backend,
    /// A mutating request may have committed; callers must reconcile.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Any,
    Absent,
    Matches(String),
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_blob_bytes: usize,
    pub max_chunk_bytes: usize,
    pub max_chunks: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    pub conditional_write: bool,
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub bytes: usize,
}

pub trait BlobStore: Send + Sync {
    fn capabilities(&self) -> Capabilities;
    fn put<'a>(
        &'a self,
        key: &'a str,
        input: Upload,
        condition: Condition,
    ) -> FutureResult<'a, Receipt>;
    fn get<'a>(&'a self, key: &'a str) -> FutureResult<'a, Vec<u8>>;
    /// Missing objects are already deleted and therefore succeed.
    fn delete<'a>(&'a self, key: &'a str) -> FutureResult<'a, ()>;
}
