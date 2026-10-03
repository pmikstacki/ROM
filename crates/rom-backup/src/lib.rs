//! Bounded private logical maintenance archives. Never exposes archives through Resource APIs.
//! SHA-256 detects accidental corruption, not hostile modification. Use a trusted parent
//! directory. Archives contain protected values; external blobs and deliveries are excluded.

mod archive;
mod codec;
mod collector;
mod legacy;
mod model;
mod publication;
#[cfg(test)]
mod tests;

pub use archive::{read, write};
pub use collector::Collector;
pub use legacy::upgrade_v1_archive;
pub use model::{Backend, BackupLimits, Manifest, Snapshot, StoredEffect};
pub use publication::Stage;
