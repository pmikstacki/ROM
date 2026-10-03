//! Provider-neutral blob storage and ordinary authorized attachment Resources.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod error;
mod identity;
mod resource;
mod service;
mod staging;
mod storage;

pub use error::Error;
pub use identity::{Digest, ObjectKey};
pub use resource::{Blob, BlobState, definition, worker_actor};
pub use service::{BlobService, BlobServiceBuilder, Limits, ObjectReceipt, UploadOutcome};
pub use storage::{BlobStore, Metadata, StoreFuture, Upload};

pub type Result<T> = std::result::Result<T, Error>;
