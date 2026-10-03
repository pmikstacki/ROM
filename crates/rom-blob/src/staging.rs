//! Bounded stream staging and blocking content hashing.
use crate::{Digest, Error, Limits, Result, Upload};
use futures_util::StreamExt;

pub(crate) async fn hash(bytes: Vec<u8>) -> Result<(Vec<u8>, Digest)> {
    tokio::task::spawn_blocking(move || {
        let digest = Digest::of(&bytes);
        (bytes, digest)
    })
    .await
    .map_err(|_| Error::Panicked)
}
pub(crate) async fn stage(mut input: Upload, limits: Limits) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut count = 0usize;
    while let Some(chunk) = input.next().await {
        count += 1;
        let chunk = chunk?;
        if count > limits.chunks
            || chunk.len() > limits.chunk_bytes
            || chunk.len() > limits.blob_bytes - bytes.len()
        {
            return Err(Error::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
