//! Development-only conformance for the native alpha extension profile.
#![forbid(unsafe_code)]

#[cfg(feature = "blob")]
pub mod blob;
mod error;
pub mod field;
pub mod profile;
pub mod storage;
pub use error::{ConformanceError, ConformanceResult, FailureCategory};
pub use field::CodecCase;
pub use profile::PROFILE_VERSION;
pub use storage::{StorageFacts, StorageFixture};
