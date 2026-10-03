//! Transfer limits and results shared by the blob service operations.
use crate::{Blob, Digest, ObjectKey};
use std::time::Duration;
#[derive(Clone, Copy)]
pub struct Limits {
    pub operations: usize,
    pub blob_bytes: usize,
    pub chunk_bytes: usize,
    pub chunks: usize,
    pub staging_timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            operations: 4,
            blob_bytes: 1024 * 1024,
            chunk_bytes: 64 * 1024,
            chunks: 1024,
            staging_timeout: Duration::from_secs(10),
        }
    }
}
#[derive(Clone, Debug)]
pub struct ObjectReceipt {
    pub key: ObjectKey,
    pub digest: Digest,
    pub bytes: u64,
}
#[derive(Clone, Debug)]
pub enum UploadOutcome {
    Attached(rom::Snapshot<Blob>),
    /// Complete object exists but metadata was not acknowledged; inspect/retry before cleanup.
    Unattached {
        object: ObjectReceipt,
        cause: rom::Error,
    },
}
