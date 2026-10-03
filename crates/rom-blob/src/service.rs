//! Host-owned blob orchestration; operations share one supervised lifecycle.
mod configuration;
mod lifecycle;
mod operations;
mod types;

pub use configuration::BlobServiceBuilder;
pub use lifecycle::BlobService;
pub use types::{Limits, ObjectReceipt, UploadOutcome};
