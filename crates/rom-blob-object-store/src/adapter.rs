//! Create-only writes and bounded reads over an object-store provider.
use crate::errors::{read_error, write_error};
use futures_util::StreamExt;
use object_store::{ObjectStore, ObjectStoreExt, PutMode, PutOptions, path::Path};
use rom_blob::{BlobStore, Error, Metadata, ObjectKey, StoreFuture};
use std::sync::Arc;

/// Provider configuration is host-owned; no SDK type crosses the blob contract.
pub struct Adapter {
    pub(crate) inner: Arc<dyn ObjectStore>,
    pub(crate) max_bytes: usize,
}
impl BlobStore for Adapter {
    fn create<'a>(&'a self, key: &'a ObjectKey, bytes: Vec<u8>) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            if bytes.len() > self.max_bytes {
                return Err(Error::TooLarge);
            }
            self.inner
                .put_opts(
                    &Path::from(key.as_str()),
                    bytes.into(),
                    PutOptions {
                        mode: PutMode::Create,
                        ..Default::default()
                    },
                )
                .await
                .map_err(write_error)?;
            Ok(())
        })
    }
    fn get<'a>(&'a self, key: &'a ObjectKey, max_bytes: usize) -> StoreFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let limit = max_bytes.min(self.max_bytes);
            let result = self
                .inner
                .get(&Path::from(key.as_str()))
                .await
                .map_err(read_error)?;
            if result.meta.size > limit as u64 {
                return Err(Error::TooLarge);
            }
            let mut stream = result.into_stream();
            let mut bytes = Vec::new();
            let mut chunks = 0usize;
            while let Some(chunk) = stream.next().await {
                chunks += 1;
                let chunk = chunk.map_err(read_error)?;
                if chunks > 65_536 || chunk.len() > limit - bytes.len() {
                    return Err(Error::TooLarge);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
    }
    fn head<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, Metadata> {
        Box::pin(async move {
            let meta = self
                .inner
                .head(&Path::from(key.as_str()))
                .await
                .map_err(read_error)?;
            Ok(Metadata { bytes: meta.size })
        })
    }
    fn delete<'a>(&'a self, key: &'a ObjectKey) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            match self.inner.delete(&Path::from(key.as_str())).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
                Err(error) => Err(write_error(error)),
            }
        })
    }
}
