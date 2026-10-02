//! Real bytes outside the metadata database, accessed through the ordinary Blob Resource.
use crate::{session_actor, smoke::SmokeResult};
use rom::Runtime;
use rom_blob::{BlobService, Digest, UploadOutcome};
use std::{path::Path, sync::Arc};

pub const CONTENT: &[u8] = b"Resource author workshop attachment\n";
pub const ID: &str = "workshop-guide";
/// `root` and all ancestors must be trusted host-owned directories, not client input.
pub fn open(runtime: Runtime, root: &Path) -> SmokeResult<BlobService> {
    std::fs::create_dir_all(root)?;
    let adapter = rom_blob_object_store::Adapter::trusted_folder(root, 1024 * 1024)?;
    Ok(BlobService::builder(runtime)
        .store("attachments", Arc::new(adapter))
        .build()?)
}
/// Explicit host demonstration; no route can invoke the trusted Blob worker directly.
pub async fn attach(service: &BlobService) -> SmokeResult<()> {
    service
        .reserve(
            &session_actor(),
            ID,
            "attachments",
            Digest::of(CONTENT),
            CONTENT.len() as u64,
            "demo-reserve-guide",
        )
        .await?;
    let input = Box::pin(futures_util::stream::iter([Ok(CONTENT.to_vec())]));
    match service.upload(&session_actor(), ID, input).await? {
        UploadOutcome::Attached(_) => {}
        UploadOutcome::Unattached { .. } => {
            return Err("attachment finalization needs reconciliation".into());
        }
    }
    assert_eq!(service.read(&session_actor(), ID).await?, CONTENT);
    Ok(())
}
