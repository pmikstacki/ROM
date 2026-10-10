//! Additive canonical journal import support for trusted native adapters.
use super::metadata::StorageMetadata;
use super::*;
mod header;
mod image;
pub use header::{MetadataHeader, MetadataHeaderParts};
pub use image::JournalImage;
#[cfg(test)]
mod header_tests;
#[cfg(test)]
mod image_tests;

mod delta;
mod page;
pub(super) mod prepare;
mod read;
pub use delta::{JournalDelta, NativeBundleDelta};
pub use page::journal_page;
pub use prepare::prepare_native_bundle;
pub use read::{JournalEntry, JournalRead};
#[cfg(test)]
mod oracle_tests;

#[cfg(test)]
mod readset_tests;

#[cfg(test)]
mod serialization_tests;
